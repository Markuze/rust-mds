//! `crush_do_rule` and the bucket choose functions, ported from `src/crush/mapper.c` (LGPL-2.1)
//! of the reference tree. The control flow, integer widths and signedness follow the C code so
//! that placement is bit-for-bit identical; places where the C code would crash (`BUG_ON`, NULL
//! buckets) return "no item" instead.

use super::ln_table::{LL_TBL, RH_LH_TBL};
use super::*;
use crate::hash::{CRUSH_HASH_RJENKINS1, crush_hash32_2, crush_hash32_3, crush_hash32_4};

/// `CRUSH_ITEM_UNDEF`: undefined result (internal).
pub const CRUSH_ITEM_UNDEF: i32 = 0x7fff_fffe;
/// `CRUSH_ITEM_NONE`: no item.
pub const CRUSH_ITEM_NONE: i32 = 0x7fff_ffff;

/// `struct crush_work_bucket` (permutation state of one bucket within one `do_rule` call).
#[derive(Default, Clone)]
struct WorkBucket {
    perm_x: u32,
    perm_n: u32,
    perm: Vec<u32>,
}

/// `struct crush_work`: per-call scratch (`crush_init_workspace` sets every `perm_x`/`perm_n`
/// to 0). Entries are created on first use (only uniform buckets and the local fallback
/// permutation need them), keyed by bucket id.
struct Work {
    buckets: std::collections::HashMap<i32, WorkBucket>,
}

impl Work {
    fn new(_map: &CrushMap) -> Self {
        Work { buckets: std::collections::HashMap::new() }
    }

    fn bucket(&mut self, b: &Bucket) -> &mut WorkBucket {
        let w = self.buckets.entry(b.id).or_default();
        if w.perm.len() != b.items.len() {
            w.perm = vec![0; b.items.len()];
        }
        w
    }
}

/// `bucket_perm_choose`: choose by a pseudo-random permutation of the bucket's items, computed
/// lazily up to position `r % size` (with a shortcut for `r % size == 0`).
fn bucket_perm_choose(bucket: &Bucket, work: &mut WorkBucket, x: i32, r: i32) -> i32 {
    let size = bucket.items.len() as u32;
    let pr = (r as u32) % size;
    let id = bucket.id as u32;
    if work.perm_x != x as u32 || work.perm_n == 0 {
        work.perm_x = x as u32;
        // optimize common r=0 case
        if pr == 0 {
            let s = crush_hash32_3(bucket.hash, x as u32, id, 0) % size;
            work.perm[0] = s;
            work.perm_n = 0xffff; // magic value, see below
            return bucket.items[s as usize];
        }
        for i in 0..size {
            work.perm[i as usize] = i;
        }
        work.perm_n = 0;
    } else if work.perm_n == 0xffff {
        // clean up after the r=0 case above
        for i in 1..size {
            work.perm[i as usize] = i;
        }
        let p0 = work.perm[0] as usize;
        work.perm[p0] = 0;
        work.perm_n = 1;
    }
    // calculate permutation up to pr
    while work.perm_n <= pr {
        let p = work.perm_n;
        // no point in swapping the final entry
        if p < size - 1 {
            let i = crush_hash32_3(bucket.hash, x as u32, id, p) % (size - p);
            if i != 0 {
                work.perm.swap((p + i) as usize, p as usize);
            }
        }
        work.perm_n += 1;
    }
    let s = work.perm[pr as usize];
    bucket.items[s as usize]
}

/// `bucket_list_choose`.
fn bucket_list_choose(bucket: &Bucket, item_weights: &[u32], sum_weights: &[u32], x: i32, r: i32) -> i32 {
    for i in (0..bucket.items.len()).rev() {
        let mut w = u64::from(crush_hash32_4(bucket.hash, x as u32, bucket.items[i] as u32, r as u32, bucket.id as u32));
        w &= 0xffff;
        w *= u64::from(sum_weights[i]);
        w >>= 16;
        if w < u64::from(item_weights[i]) {
            return bucket.items[i];
        }
    }
    bucket.items[0]
}

fn tree_height(mut n: i32) -> i32 {
    let mut h = 0;
    while n & 1 == 0 {
        h += 1;
        n >>= 1;
    }
    h
}

/// `bucket_tree_choose`.
fn bucket_tree_choose(bucket: &Bucket, num_nodes: u8, node_weights: &[u32], x: i32, r: i32) -> i32 {
    // start at root
    let mut n = i32::from(num_nodes >> 1);
    if n == 0 {
        // height(0) would never terminate in the C code; a tree needs at least two nodes.
        return bucket.items[0];
    }
    let nw = |i: i32| node_weights.get(i as usize).copied().unwrap_or(0);
    while n & 1 == 0 {
        // pick point in [0, w)
        let w = nw(n);
        let t = (u64::from(crush_hash32_4(bucket.hash, x as u32, n as u32, r as u32, bucket.id as u32)) * u64::from(w)) >> 32;
        // descend to the left or right?
        let h = tree_height(n);
        let l = n - (1 << (h - 1));
        if t < u64::from(nw(l)) {
            n = l;
        } else {
            n += 1 << (h - 1);
        }
    }
    bucket.items.get((n >> 1) as usize).copied().unwrap_or(bucket.items[0])
}

/// `bucket_straw_choose`.
fn bucket_straw_choose(bucket: &Bucket, straws: &[u32], x: i32, r: i32) -> i32 {
    let mut high = 0usize;
    let mut high_draw = 0u64;
    for (i, (&item, &straw)) in bucket.items.iter().zip(straws).enumerate() {
        let mut draw = u64::from(crush_hash32_3(bucket.hash, x as u32, item as u32, r as u32));
        draw &= 0xffff;
        draw *= u64::from(straw);
        if i == 0 || draw > high_draw {
            high = i;
            high_draw = draw;
        }
    }
    bucket.items[high]
}

/// `crush_ln`: 2^44 * log2(xin + 1), fixed point, from the lookup tables.
pub fn crush_ln(xin: u32) -> u64 {
    let mut x = xin.wrapping_add(1);
    // normalize input
    let mut iexpon: i32 = 15;
    if x & 0x18000 == 0 {
        let bits = (x & 0x1ffff).leading_zeros() as i32 - 16;
        x <<= bits;
        iexpon = 15 - bits;
    }
    let index1 = ((x >> 8) << 1) as usize;
    // RH ~ 2^56/index1
    let rh = RH_LH_TBL[index1 - 256] as u64;
    // LH ~ 2^48 * log2(index1/256)
    let mut lh = RH_LH_TBL[index1 + 1 - 256] as u64;
    // RH*x ~ 2^48 * (2^15 + xf), xf<2^8
    let mut xl64 = (i64::from(x) as u64).wrapping_mul(rh);
    xl64 >>= 48;
    let mut result = iexpon as u64;
    result <<= 12 + 32;
    let index2 = (xl64 & 0xff) as usize;
    // LL ~ 2^48*log2(1.0+index2/2^15)
    let ll = LL_TBL[index2] as u64;
    lh = lh.wrapping_add(ll);
    lh >>= 48 - 12 - 32;
    result.wrapping_add(lh)
}

/// `generate_exponential_distribution`: `ln(u) / weight` for straw2 (the weight is a C `int`).
fn generate_exponential_distribution(hash: u8, x: i32, y: i32, z: i32, weight: u32) -> i64 {
    let mut u = crush_hash32_3(hash, x as u32, y as u32, z as u32);
    u &= 0xffff;
    // the natural log lookup table maps [0,0xffff] (corresponding to real numbers
    // [1/0x10000, 1]) to [0, 0xffffffffffff] (corresponding to real numbers [-11.090355,0]).
    let ln = (crush_ln(u) as i64).wrapping_sub(0x1_0000_0000_0000);
    // divide by 16.16 fixed-point weight
    ln.wrapping_div(i64::from(weight as i32))
}

/// `bucket_straw2_choose` with `choose_args` replacement weights (per `position`) and ids.
fn bucket_straw2_choose(bucket: &Bucket, item_weights: &[u32], x: i32, r: i32, arg: Option<&ChooseArg>, position: i32) -> i32 {
    // get_choose_arg_weights / get_choose_arg_ids
    let weights: &[u32] = match arg {
        Some(a) if !a.weight_set.is_empty() => {
            let mut p = position as u32 as usize;
            if p >= a.weight_set.len() {
                p = a.weight_set.len() - 1;
            }
            &a.weight_set[p]
        }
        _ => item_weights,
    };
    let ids: &[i32] = match arg {
        Some(a) if !a.ids.is_empty() => &a.ids,
        _ => &bucket.items,
    };
    let mut high = 0usize;
    let mut high_draw = 0i64;
    for i in 0..bucket.items.len() {
        let w = weights.get(i).copied().unwrap_or(0);
        let draw = if w != 0 {
            generate_exponential_distribution(bucket.hash, x, ids.get(i).copied().unwrap_or(bucket.items[i]), r, w)
        } else {
            i64::MIN
        };
        if i == 0 || draw > high_draw {
            high = i;
            high_draw = draw;
        }
    }
    bucket.items[high]
}

/// `crush_bucket_choose`.
fn crush_bucket_choose(bucket: &Bucket, work: &mut Work, x: i32, r: i32, arg: Option<&ChooseArg>, position: i32) -> i32 {
    if bucket.items.is_empty() {
        // BUG_ON(in->size == 0) in C.
        return CRUSH_ITEM_NONE;
    }
    match &bucket.data {
        BucketData::Uniform { .. } => {
            let w = work.bucket(bucket);
            bucket_perm_choose(bucket, w, x, r)
        }
        BucketData::List { item_weights, sum_weights } => bucket_list_choose(bucket, item_weights, sum_weights, x, r),
        BucketData::Tree { num_nodes, node_weights } => bucket_tree_choose(bucket, *num_nodes, node_weights, x, r),
        BucketData::Straw { straws, .. } => bucket_straw_choose(bucket, straws, x, r),
        BucketData::Straw2 { item_weights } => bucket_straw2_choose(bucket, item_weights, x, r, arg, position),
    }
}

fn choose_arg_for<'a>(choose_args: Option<&'a [ChooseArg]>, bucket: &Bucket) -> Option<&'a ChooseArg> {
    choose_args.and_then(|a| a.get((-1 - bucket.id) as usize))
}

/// `is_out`: true if device `item` is marked "out" (weight 0, or probabilistically for partial
/// weights) for input `x`.
fn is_out(weight: &[u32], item: i32, x: i32) -> bool {
    if item < 0 || item as usize >= weight.len() {
        return true;
    }
    let w = weight[item as usize];
    if w >= 0x10000 {
        return false;
    }
    if w == 0 {
        return true;
    }
    (crush_hash32_2(CRUSH_HASH_RJENKINS1, x as u32, item as u32) & 0xffff) >= w
}

/// Type of `item`: 0 for devices, the bucket type for buckets; `None` for a bucket id with no
/// bucket (the C code would dereference NULL).
fn item_type(map: &CrushMap, item: i32) -> Option<i32> {
    if item < 0 { map.bucket(item).map(|b| i32::from(b.bucket_type)) } else { Some(0) }
}

/// Parameters of one `crush_choose_firstn` call that stay fixed across its recursion.
struct FirstnParams<'a> {
    weight: &'a [u32],
    x: i32,
    choose_args: Option<&'a [ChooseArg]>,
}

/// `crush_choose_firstn`: choose `numrep` distinct items of type `ty` below `bucket` into
/// `out[outpos..]`; with `recurse_to_leaf`, also one device under each into `out2`.
#[allow(clippy::too_many_arguments)]
fn crush_choose_firstn(
    map: &CrushMap,
    work: &mut Work,
    p: &FirstnParams<'_>,
    bucket: &Bucket,
    numrep: i32,
    ty: i32,
    out: &mut [i32],
    mut outpos: i32,
    out_size: i32,
    tries: u32,
    recurse_tries: u32,
    local_retries: u32,
    local_fallback_retries: u32,
    recurse_to_leaf: bool,
    vary_r: u32,
    stable: u32,
    mut out2: Option<&mut [i32]>,
    parent_r: i32,
) -> i32 {
    let x = p.x;
    let mut count = out_size;
    let mut rep = if stable != 0 { 0 } else { outpos };
    while rep < numrep && count > 0 {
        // keep trying until we get a non-out, non-colliding item
        let mut ftotal: u32 = 0;
        let mut skip_rep = false;
        let mut item: i32 = 0;
        loop {
            let mut retry_descent = false;
            let mut inb: &Bucket = bucket; // initial bucket
            // choose through intervening buckets
            let mut flocal: u32 = 0;
            loop {
                let mut collide = false;
                let mut retry_bucket = false;
                let r = rep.wrapping_add(parent_r).wrapping_add(ftotal as i32);
                let reject = if inb.items.is_empty() {
                    true
                } else {
                    let size = inb.items.len() as u32;
                    item = if local_fallback_retries > 0 && flocal >= (size >> 1) && flocal > local_fallback_retries {
                        let w = work.bucket(inb);
                        bucket_perm_choose(inb, w, x, r)
                    } else {
                        crush_bucket_choose(inb, work, x, r, choose_arg_for(p.choose_args, inb), outpos)
                    };
                    if item >= map.max_devices {
                        skip_rep = true;
                        break;
                    }
                    // desired type?
                    let Some(itemtype) = item_type(map, item) else {
                        skip_rep = true;
                        break;
                    };
                    // keep going?
                    if itemtype != ty {
                        if item >= 0 || (-1 - item) >= map.max_buckets() {
                            skip_rep = true;
                            break;
                        }
                        inb = map.bucket(item).expect("checked by item_type");
                        // retry_bucket = 1; continue (skips the reject handling)
                        continue;
                    }
                    // collision?
                    collide = out[..outpos as usize].contains(&item);
                    let mut rej = false;
                    if !collide && recurse_to_leaf {
                        if item < 0 {
                            let sub_r = if vary_r != 0 {
                                let sh = vary_r - 1;
                                if sh >= 32 { if r < 0 { -1 } else { 0 } } else { r >> sh }
                            } else {
                                0
                            };
                            let sub = map.bucket(item).expect("checked by item_type");
                            let o2 = out2.as_deref_mut().expect("chooseleaf needs out2");
                            let got = crush_choose_firstn(
                                map,
                                work,
                                p,
                                sub,
                                if stable != 0 { 1 } else { outpos + 1 },
                                0,
                                o2,
                                outpos,
                                count,
                                recurse_tries,
                                0,
                                local_retries,
                                local_fallback_retries,
                                false,
                                vary_r,
                                stable,
                                None,
                                sub_r,
                            );
                            if got <= outpos {
                                // didn't get leaf
                                rej = true;
                            }
                        } else if let Some(o2) = out2.as_deref_mut() {
                            // we already have a leaf!
                            o2[outpos as usize] = item;
                        }
                    }
                    if !rej && !collide && itemtype == 0 {
                        // out?
                        rej = is_out(p.weight, item, x);
                    }
                    rej
                };
                // reject:
                if reject || collide {
                    ftotal += 1;
                    flocal += 1;
                    if collide && flocal <= local_retries {
                        // retry locally a few times
                        retry_bucket = true;
                    } else if local_fallback_retries > 0 && flocal <= (inb.items.len() as u32).wrapping_add(local_fallback_retries) {
                        // exhaustive bucket search
                        retry_bucket = true;
                    } else if ftotal < tries {
                        // then retry descent
                        retry_descent = true;
                    } else {
                        // else give up
                        skip_rep = true;
                    }
                }
                if !retry_bucket {
                    break;
                }
            }
            if !retry_descent {
                break;
            }
        }
        if !skip_rep {
            out[outpos as usize] = item;
            outpos += 1;
            count -= 1;
        }
        rep += 1;
    }
    outpos
}

/// `crush_choose_indep`: breadth-first, positionally stable choice of `left` items of type
/// `ty` into `out[outpos..outpos+left]` (holes are `CRUSH_ITEM_NONE`).
#[allow(clippy::too_many_arguments)]
fn crush_choose_indep(
    map: &CrushMap,
    work: &mut Work,
    p: &FirstnParams<'_>,
    bucket: &Bucket,
    mut left: i32,
    numrep: i32,
    ty: i32,
    out: &mut [i32],
    outpos: i32,
    tries: u32,
    recurse_tries: u32,
    recurse_to_leaf: bool,
    mut out2: Option<&mut [i32]>,
    parent_r: i32,
) {
    let x = p.x;
    let endpos = outpos + left;
    // initially my result is undefined
    for rep in outpos..endpos {
        out[rep as usize] = CRUSH_ITEM_UNDEF;
        if let Some(o2) = out2.as_deref_mut() {
            o2[rep as usize] = CRUSH_ITEM_UNDEF;
        }
    }
    let mut ftotal: u32 = 0;
    while left > 0 && ftotal < tries {
        for rep in outpos..endpos {
            if out[rep as usize] != CRUSH_ITEM_UNDEF {
                continue;
            }
            let mut inb: &Bucket = bucket; // initial bucket
            // choose through intervening buckets
            loop {
                // note: we base the choice on the position even in the nested call.
                let mut r = rep.wrapping_add(parent_r);
                // be careful
                let n = numrep as u32;
                if inb.alg == CRUSH_BUCKET_UNIFORM && n != 0 && (inb.items.len() as u32).is_multiple_of(n) {
                    // r'=r+(n+1)*f_total
                    r = r.wrapping_add(n.wrapping_add(1).wrapping_mul(ftotal) as i32);
                } else {
                    // r' = r + n*f_total
                    r = r.wrapping_add(n.wrapping_mul(ftotal) as i32);
                }
                // bucket choose
                if inb.items.is_empty() {
                    break;
                }
                let item = crush_bucket_choose(inb, work, x, r, choose_arg_for(p.choose_args, inb), outpos);
                if item >= map.max_devices {
                    out[rep as usize] = CRUSH_ITEM_NONE;
                    if let Some(o2) = out2.as_deref_mut() {
                        o2[rep as usize] = CRUSH_ITEM_NONE;
                    }
                    left -= 1;
                    break;
                }
                // desired type?
                let itemtype = item_type(map, item);
                // keep going?
                if itemtype != Some(ty) {
                    if item >= 0 || (-1 - item) >= map.max_buckets() || itemtype.is_none() {
                        out[rep as usize] = CRUSH_ITEM_NONE;
                        if let Some(o2) = out2.as_deref_mut() {
                            o2[rep as usize] = CRUSH_ITEM_NONE;
                        }
                        left -= 1;
                        break;
                    }
                    inb = map.bucket(item).expect("checked by item_type");
                    continue;
                }
                // collision?
                if out[outpos as usize..endpos as usize].contains(&item) {
                    break;
                }
                if recurse_to_leaf {
                    if item < 0 {
                        let sub = map.bucket(item).expect("checked by item_type");
                        let o2 = out2.as_deref_mut().expect("chooseleaf needs out2");
                        crush_choose_indep(map, work, p, sub, 1, numrep, 0, o2, rep, recurse_tries, 0, false, None, r);
                        if o2[rep as usize] == CRUSH_ITEM_NONE {
                            // placed nothing; no leaf
                            break;
                        }
                    } else if let Some(o2) = out2.as_deref_mut() {
                        // we already have a leaf!
                        o2[rep as usize] = item;
                    }
                }
                // out?
                if itemtype == Some(0) && is_out(p.weight, item, x) {
                    break;
                }
                // yay!
                out[rep as usize] = item;
                left -= 1;
                break;
            }
        }
        ftotal += 1;
    }
    for rep in outpos..endpos {
        if out[rep as usize] == CRUSH_ITEM_UNDEF {
            out[rep as usize] = CRUSH_ITEM_NONE;
        }
        if let Some(o2) = out2.as_deref_mut()
            && o2[rep as usize] == CRUSH_ITEM_UNDEF
        {
            o2[rep as usize] = CRUSH_ITEM_NONE;
        }
    }
}

/// `crush_do_rule_no_retry`: the classic (non-MSR) rule interpreter.
fn crush_do_rule_no_retry(
    map: &CrushMap,
    rule: &Rule,
    x: i32,
    result_max: usize,
    weight: &[u32],
    choose_args: Option<&[ChooseArg]>,
) -> Vec<i32> {
    let mut work = Work::new(map);
    let p = FirstnParams { weight, x, choose_args };
    let mut result = Vec::with_capacity(result_max);
    let mut w = vec![0i32; result_max];
    let mut o = vec![0i32; result_max];
    let mut c = vec![0i32; result_max];
    let mut wsize = 0usize;
    let t = &map.tunables;
    // the original choose_total_tries value was off by one (it counted "retries" and not
    // "tries").  add one.
    let mut choose_tries: i32 = (t.choose_total_tries as i32).wrapping_add(1);
    let mut choose_leaf_tries: i32 = 0;
    // the local tries values were counted as "retries", though, and need no adjustment
    let mut choose_local_retries: i32 = t.choose_local_tries as i32;
    let mut choose_local_fallback_retries: i32 = t.choose_local_fallback_tries as i32;
    let mut vary_r: i32 = i32::from(t.chooseleaf_vary_r);
    let mut stable: i32 = i32::from(t.chooseleaf_stable);

    for step in &rule.steps {
        match step.op {
            CRUSH_RULE_TAKE => {
                let a = step.arg1;
                let is_dev = a >= 0 && a < map.max_devices;
                let bidx = -1i64 - i64::from(a);
                let is_bucket = bidx >= 0 && bidx < i64::from(map.max_buckets()) && map.buckets[bidx as usize].is_some();
                if (is_dev || is_bucket) && result_max > 0 {
                    w[0] = a;
                    wsize = 1;
                }
            }
            CRUSH_RULE_SET_CHOOSE_TRIES => {
                if step.arg1 > 0 {
                    choose_tries = step.arg1;
                }
            }
            CRUSH_RULE_SET_CHOOSELEAF_TRIES => {
                if step.arg1 > 0 {
                    choose_leaf_tries = step.arg1;
                }
            }
            CRUSH_RULE_SET_CHOOSE_LOCAL_TRIES => {
                if step.arg1 >= 0 {
                    choose_local_retries = step.arg1;
                }
            }
            CRUSH_RULE_SET_CHOOSE_LOCAL_FALLBACK_TRIES => {
                if step.arg1 >= 0 {
                    choose_local_fallback_retries = step.arg1;
                }
            }
            CRUSH_RULE_SET_CHOOSELEAF_VARY_R => {
                if step.arg1 >= 0 {
                    vary_r = step.arg1;
                }
            }
            CRUSH_RULE_SET_CHOOSELEAF_STABLE => {
                if step.arg1 >= 0 {
                    stable = step.arg1;
                }
            }
            CRUSH_RULE_CHOOSELEAF_FIRSTN | CRUSH_RULE_CHOOSE_FIRSTN | CRUSH_RULE_CHOOSELEAF_INDEP | CRUSH_RULE_CHOOSE_INDEP => {
                let firstn = step.op == CRUSH_RULE_CHOOSELEAF_FIRSTN || step.op == CRUSH_RULE_CHOOSE_FIRSTN;
                if wsize == 0 {
                    continue;
                }
                let recurse_to_leaf = step.op == CRUSH_RULE_CHOOSELEAF_FIRSTN || step.op == CRUSH_RULE_CHOOSELEAF_INDEP;
                // reset output
                let mut osize = 0usize;
                for &wi in w.iter().take(wsize) {
                    let mut numrep = step.arg1;
                    if numrep <= 0 {
                        numrep += result_max as i32;
                        if numrep <= 0 {
                            continue;
                        }
                    }
                    // make sure bucket id is valid
                    let bno = -1i64 - i64::from(wi);
                    if bno < 0 || bno >= i64::from(map.max_buckets()) {
                        // w[i] is probably CRUSH_ITEM_NONE
                        continue;
                    }
                    let Some(bucket) = map.buckets[bno as usize].as_ref() else {
                        continue;
                    };
                    if firstn {
                        let recurse_tries = if choose_leaf_tries != 0 {
                            choose_leaf_tries
                        } else if t.chooseleaf_descend_once != 0 {
                            1
                        } else {
                            choose_tries
                        };
                        let got = crush_choose_firstn(
                            map,
                            &mut work,
                            &p,
                            bucket,
                            numrep,
                            step.arg2,
                            &mut o[osize..],
                            0,
                            (result_max - osize) as i32,
                            choose_tries as u32,
                            recurse_tries as u32,
                            choose_local_retries as u32,
                            choose_local_fallback_retries as u32,
                            recurse_to_leaf,
                            vary_r as u32,
                            stable as u32,
                            Some(&mut c[osize..]),
                            0,
                        );
                        osize += got as usize;
                    } else {
                        let out_size = (numrep as usize).min(result_max - osize);
                        crush_choose_indep(
                            map,
                            &mut work,
                            &p,
                            bucket,
                            out_size as i32,
                            numrep,
                            step.arg2,
                            &mut o[osize..],
                            0,
                            choose_tries as u32,
                            if choose_leaf_tries != 0 { choose_leaf_tries as u32 } else { 1 },
                            recurse_to_leaf,
                            Some(&mut c[osize..]),
                            0,
                        );
                        osize += out_size;
                    }
                }
                if recurse_to_leaf {
                    // copy final _leaf_ values to output set
                    o[..osize].copy_from_slice(&c[..osize]);
                }
                // swap o and w arrays
                std::mem::swap(&mut o, &mut w);
                wsize = osize;
            }
            CRUSH_RULE_EMIT => {
                for &wi in w.iter().take(wsize) {
                    if result.len() >= result_max {
                        break;
                    }
                    result.push(wi);
                }
                wsize = 0;
            }
            _ => {}
        }
    }
    result
}

// ---------------------------------------------------------------------------------------------
// MSR rules (crush_msr_*): multi-step-retry rules of type msr_firstn / msr_indep.

struct MsrInput<'a> {
    map: &'a CrushMap,
    rule: &'a Rule,
    result_max: u32,
    weights: &'a [u32],
    map_input: i32,
    choose_args: Option<&'a [ChooseArg]>,
    msr_descents: u32,
    msr_collision_tries: u32,
}

/// Work space of one EMIT block: one vector of `result_len` per choose step.
struct MsrWorkspace {
    start_stepno: u32,
    end_stepno: u32,
    step_vecs: Vec<Vec<i32>>,
}

struct MsrOutput {
    returned_so_far: u32,
    out: Vec<i32>,
}

/// `crush_msr_scan_config_steps`.
fn msr_scan_config_steps(steps: &[RuleStep], descents: &mut u32, collision_tries: &mut u32) -> u32 {
    let mut stepno = 0u32;
    for s in steps {
        match s.op {
            CRUSH_RULE_SET_MSR_DESCENTS => *descents = s.arg1 as u32,
            CRUSH_RULE_SET_MSR_COLLISION_TRIES => *collision_tries = s.arg1 as u32,
            _ => return stepno,
        }
        stepno += 1;
    }
    stepno
}

/// `crush_msr_scan_next`: validate `TAKE CHOOSE_MSR* EMIT` at `stepno`; returns the total fan
/// out and the index of the EMIT step.
fn msr_scan_next(rule: &Rule, result_max: u32, stepno: u32) -> Option<(u32, u32)> {
    let len = rule.steps.len() as u32;
    if stepno + 1 >= len {
        return None;
    }
    if rule.steps[stepno as usize].op != CRUSH_RULE_TAKE {
        return None;
    }
    let mut total_children: u32 = 1;
    let mut s = stepno + 1;
    while s < len {
        let cur = &rule.steps[s as usize];
        if cur.op == CRUSH_RULE_EMIT {
            break;
        }
        if cur.op != CRUSH_RULE_CHOOSE_MSR {
            return None;
        }
        total_children = total_children.wrapping_mul(if cur.arg1 != 0 { cur.arg1 as u32 } else { result_max });
        s += 1;
    }
    if s >= len {
        return None;
    }
    Some((total_children, s))
}

impl MsrWorkspace {
    fn vec(&self, stepno: u32) -> &Vec<i32> {
        &self.step_vecs[(stepno - self.start_stepno) as usize]
    }
    fn vec_mut(&mut self, stepno: u32) -> &mut Vec<i32> {
        &mut self.step_vecs[(stepno - self.start_stepno) as usize]
    }

    /// `crush_msr_leaf_vec_populated`: all leaf slots in `[start, end)` mapped.
    fn leaf_vec_populated(&self, start: u32, end: u32) -> bool {
        let leaf = &self.step_vecs[(self.end_stepno - self.start_stepno - 1) as usize];
        leaf[start as usize..end as usize].iter().all(|&v| v != CRUSH_ITEM_UNDEF)
    }

    /// `crush_msr_valid_candidate`.
    fn valid_candidate(&self, stepno: u32, exclude: (u32, u32), include: (u32, u32), candidate: i32) -> bool {
        let v = self.vec(stepno);
        for i in exclude.0..exclude.1 {
            if v[i as usize] == candidate {
                return i >= include.0 && i < include.1;
            }
        }
        true
    }

    /// `crush_msr_push_used`.
    fn push_used(&mut self, stepno: u32, stride_start: u32, stride_end: u32, candidate: i32) -> bool {
        let v = self.vec_mut(stepno);
        for i in stride_start..stride_end {
            if v[i as usize] == candidate {
                return false;
            } else if v[i as usize] == CRUSH_ITEM_UNDEF {
                v[i as usize] = candidate;
                return true;
            }
        }
        false
    }

    /// `crush_msr_pop_used`.
    fn pop_used(&mut self, stepno: u32, stride_start: u32, stride_end: u32) {
        let v = self.vec_mut(stepno);
        for i in (stride_start..stride_end).rev() {
            if v[i as usize] != CRUSH_ITEM_UNDEF {
                v[i as usize] = CRUSH_ITEM_UNDEF;
                return;
            }
        }
    }
}

/// `crush_msr_get_retry_value`.
fn msr_retry_value(result_max: u32, index: u32, tryno: u32, local_tryno: u32) -> u32 {
    let total_index = tryno.wrapping_mul(result_max).wrapping_add(index);
    (total_index << 16).wrapping_add(local_tryno)
}

/// `crush_msr_emit_result`.
fn msr_emit_result(output: &mut MsrOutput, rule_type: u8, position: u32, result: i32) {
    let len = output.out.len() as u32;
    if position >= len || output.returned_so_far >= len {
        return;
    }
    if rule_type == CRUSH_RULE_TYPE_MSR_FIRSTN {
        output.out[output.returned_so_far as usize] = result;
        output.returned_so_far += 1;
    } else {
        output.out[position as usize] = result;
        output.returned_so_far += 1;
    }
}

/// `crush_msr_descend`: descend from `bucket` until a device or a bucket of type `ty`.
fn msr_descend<'a>(input: &MsrInput<'a>, work: &mut Work, mut bucket: &'a Bucket, ty: i32, tryno: u32, local_tryno: u32, index: u32) -> i32 {
    loop {
        let cand = crush_bucket_choose(
            bucket,
            work,
            input.map_input,
            msr_retry_value(input.result_max, index, tryno, local_tryno) as i32,
            choose_arg_for(input.choose_args, bucket),
            index as i32,
        );
        if cand >= 0 {
            return cand;
        }
        let Some(b) = input.map.bucket(cand) else {
            return CRUSH_ITEM_NONE;
        };
        bucket = b;
        if i32::from(bucket.bucket_type) == ty {
            return cand;
        }
    }
}

/// `crush_msr_choose`: map one EMIT block's choose steps `[current_stepno, end_stepno)` into
/// mapping indices `[start_index, end_index)`.
#[allow(clippy::too_many_arguments)]
fn msr_choose<'a>(
    input: &MsrInput<'a>,
    work: &mut Work,
    ws: &mut MsrWorkspace,
    output: &mut MsrOutput,
    bucket: &'a Bucket,
    total_descendants: u32,
    start_index: u32,
    end_index: u32,
    current_stepno: u32,
    end_stepno: u32,
    tryno: u32,
) -> u32 {
    let curstep = input.rule.steps[current_stepno as usize];
    // choosemsr 0 host should select result_max hosts
    let num_strides = if curstep.arg1 != 0 { curstep.arg1 as u32 } else { input.result_max };
    if num_strides == 0 || !total_descendants.is_multiple_of(num_strides) {
        return 0;
    }
    let stride_length = total_descendants / num_strides;
    if stride_length == 0 {
        return 0;
    }
    let mut undo = vec![CRUSH_ITEM_UNDEF; num_strides as usize];
    let mut mapped = 0u32;
    let mut stride_index = 0usize;
    let mut stride_start = start_index;
    while stride_start < end_index {
        let stride_end = (stride_start + stride_length).min(end_index);
        'stride: {
            // all descendants for this stride have been mapped already
            if ws.leaf_vec_populated(stride_start, stride_end) {
                break 'stride;
            }
            let mut found = false;
            let mut child = 0;
            for local_tryno in 0..input.msr_collision_tries {
                child = msr_descend(input, work, bucket, curstep.arg2, tryno, local_tryno, stride_index as u32);
                // candidate is valid if we already chose it for this stride, or it hasn't
                // been chosen for any stride
                if ws.valid_candidate(current_stepno, (start_index, end_index), (stride_start, stride_end), child) {
                    found = true;
                    break;
                }
            }
            // failed to find non-colliding choice after msr_collision_tries attempts
            if !found {
                break 'stride;
            }
            if curstep.arg2 == 0 {
                // leaf
                if stride_length != 1 || current_stepno + 1 != end_stepno {
                    // another step after a choosemsr step for the leaf type: malformed
                    break 'stride;
                }
                if is_out(input.weights, child, input.map_input) {
                    // crush_msr_do_rule will try again, msr_descents permitting
                    break 'stride;
                }
                // for collision detection
                ws.push_used(current_stepno, stride_start, stride_end, child);
                // final output, ordering depending on the rule type
                msr_emit_result(output, input.rule.rule_type, stride_start, child);
                mapped += 1;
            } else {
                if current_stepno + 1 >= end_stepno {
                    // type isn't leaf, rule is malformed since there isn't another step
                    break 'stride;
                }
                let Some(child_bucket) = input.map.bucket(child) else {
                    break 'stride;
                };
                let child_mapped = msr_choose(
                    input,
                    work,
                    ws,
                    output,
                    child_bucket,
                    stride_length,
                    stride_start,
                    stride_end,
                    current_stepno + 1,
                    end_stepno,
                    tryno,
                );
                let pushed = ws.push_used(current_stepno, stride_start, stride_end, child);
                // pushed may be false if we already chose this bucket for this stride
                if pushed && child_mapped == 0 {
                    undo[stride_index] = child;
                } else {
                    mapped += child_mapped;
                }
            }
        }
        stride_start += stride_length;
        stride_index += 1;
    }
    // pop unused buckets
    let mut stride_index = 0usize;
    let mut stride_start = start_index;
    while stride_start < end_index {
        if undo[stride_index] != CRUSH_ITEM_UNDEF {
            let stride_end = (stride_start + stride_length).min(end_index);
            ws.pop_used(current_stepno, stride_start, stride_end);
        }
        stride_start += stride_length;
        stride_index += 1;
    }
    mapped
}

/// `crush_msr_do_rule`.
fn crush_msr_do_rule(
    map: &CrushMap,
    rule: &Rule,
    map_input: i32,
    result_max: usize,
    weight: &[u32],
    choose_args: Option<&[ChooseArg]>,
) -> Vec<i32> {
    let mut msr_descents = map.tunables.msr_descents;
    let mut msr_collision_tries = map.tunables.msr_collision_tries;
    let mut start_stepno = msr_scan_config_steps(&rule.steps, &mut msr_descents, &mut msr_collision_tries);
    let input = MsrInput {
        map,
        rule,
        result_max: result_max as u32,
        weights: weight,
        map_input,
        choose_args,
        msr_descents,
        msr_collision_tries,
    };
    let mut output = MsrOutput { returned_so_far: 0, out: vec![CRUSH_ITEM_NONE; result_max] };
    let mut work = Work::new(map);
    let len = rule.steps.len() as u32;
    let mut start_index = 0u32;
    while start_stepno < len {
        let Some((total_children, emit_stepno)) = msr_scan_next(rule, input.result_max, start_stepno) else {
            // invalid rule, return whatever we have
            return Vec::new();
        };
        let take = rule.steps[start_stepno as usize];
        if take.arg1 >= 0 {
            if start_stepno + 1 != emit_stepno {
                // invalid rule: take step specifies osd, but there are subsequent choose steps
                return Vec::new();
            }
            msr_emit_result(&mut output, rule.rule_type, start_index, take.arg1);
            // The C loop does not advance here (it would assert on the next emit); move on to
            // the next block instead.
            start_stepno = emit_stepno + 1;
            continue;
        }
        let Some(root_bucket) = map.bucket(take.arg1) else {
            return Vec::new();
        };
        start_stepno += 1;
        let nvec = (emit_stepno - start_stepno) as usize;
        let mut ws = MsrWorkspace {
            start_stepno,
            end_stepno: emit_stepno,
            step_vecs: vec![vec![CRUSH_ITEM_UNDEF; result_max]; nvec.max(1)],
        };
        if nvec == 0 {
            // TAKE bucket directly followed by EMIT: nothing to choose.
            start_stepno = emit_stepno + 1;
            continue;
        }
        let mut tries_so_far = 0u32;
        let end_index = (start_index.wrapping_add(total_children)).min(input.result_max);
        let return_limit_for_block = output.returned_so_far + (end_index.saturating_sub(start_index));
        while tries_so_far < input.msr_descents && output.returned_so_far < return_limit_for_block {
            msr_choose(
                &input,
                &mut work,
                &mut ws,
                &mut output,
                root_bucket,
                total_children,
                start_index,
                end_index,
                start_stepno,
                emit_stepno,
                tries_so_far,
            );
            tries_so_far += 1;
        }
        start_index = end_index;
        start_stepno = emit_stepno + 1;
    }
    if rule.rule_type == CRUSH_RULE_TYPE_MSR_FIRSTN {
        output.out.truncate(output.returned_so_far as usize);
    }
    output.out
}

/// `crush_do_rule(map, ruleno, x, result, result_max, weight, weight_max, cwin, choose_args)`:
/// map input `x` with rule `ruleno` to at most `result_max` items. `weight[i]` is the 16.16
/// reweight of device `i` (`osd_weight` in the OSDMap; devices beyond it are out).
pub fn crush_do_rule(
    map: &CrushMap,
    ruleno: i32,
    x: i32,
    result_max: usize,
    weight: &[u32],
    choose_args: Option<&[ChooseArg]>,
) -> Vec<i32> {
    if (ruleno as u32) >= map.max_rules() {
        return Vec::new();
    }
    let Some(rule) = map.rules[ruleno as usize].as_ref() else {
        return Vec::new();
    };
    if rule.rule_type == CRUSH_RULE_TYPE_MSR_FIRSTN || rule.rule_type == CRUSH_RULE_TYPE_MSR_INDEP {
        crush_msr_do_rule(map, rule, x, result_max, weight, choose_args)
    } else {
        crush_do_rule_no_retry(map, rule, x, result_max, weight, choose_args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values from compiling `src/crush/mapper.c`'s `crush_ln` (note that it is not monotonic
    /// at the very top of the range).
    #[test]
    fn crush_ln_reference_vectors() {
        let v: &[(u32, u64)] = &[
            (0, 0),
            (1, 17592186044416),
            (2, 27882955186109),
            (3, 35184372088832),
            (100, 117132495127636),
            (255, 140737488355328),
            (256, 140836779814266),
            (32766, 263881692995889),
            (32767, 263882790666240),
            (32768, 263883565195424),
            (65534, 281474932780304),
            (65535, 281474708275200),
            (12345, 239108530962749),
            (54321, 276712071671411),
        ];
        for (x, want) in v {
            assert_eq!(crush_ln(*x), *want, "crush_ln({x})");
        }
        let sum = (0..0x10000u64).fold(0u64, |s, u| s.wrapping_add(crush_ln(u as u32).wrapping_mul(u + 1)));
        assert_eq!(sum, 3236266311848051351);
    }
}
