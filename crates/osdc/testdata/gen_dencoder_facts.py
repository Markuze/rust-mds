#!/usr/bin/env python3
"""Extract facts from `ceph-dencoder type OSDMap import <map> decode dump_json` into a
line-based file the Rust tests can compare against without a JSON parser.

    python3 -I gen_dencoder_facts.py <map> <out.facts>
    python3 -I gen_dencoder_facts.py --all        (regenerates testdata/dencoder/)

ceph-dencoder 20.2 segfaults dumping some maps (privmon epochs >= 64), so the selection
avoids them.
"""
import json
import subprocess
import sys

if sys.argv[1] == "--all":
    import os
    here = os.path.dirname(os.path.abspath(__file__))
    os.makedirs(os.path.join(here, "dencoder"), exist_ok=True)
    for st, epochs in (("privmon", [33, 45, 56, 60]), ("live", [19, 40, 56])):
        for e in epochs:
            subprocess.check_call([sys.executable, "-I", os.path.abspath(__file__), os.path.join(here, st, f"full.{e}"),
                                   os.path.join(here, "dencoder", f"{st}.{e}.facts")])
    sys.exit(0)
src, out = sys.argv[1], sys.argv[2]
j = json.loads(subprocess.check_output(["ceph-dencoder", "type", "OSDMap", "import", src, "decode", "dump_json"]))
lines = []
w = lines.append
for k in ["epoch", "flags_num", "crush_version", "pool_max", "max_osd"]:
    w(f"{k} {j[k]}")
w(f"fsid {j['fsid']}")
for k in ["full_ratio", "backfillfull_ratio", "nearfull_ratio"]:
    w(f"{k} {j[k]}")
w(f"require_osd_release {j['require_osd_release']}")
w(f"require_min_compat_client {j['require_min_compat_client']}")
for p in j["pools"]:
    apps = ",".join(sorted(p.get("application_metadata", {}).keys()))
    w("pool {pool} name={pool_name} type={type} size={size} min_size={min_size} crush_rule={crush_rule} "
      "object_hash={object_hash} pg_num={pg_num} pgp_num={pg_placement_num} pg_num_target={pg_num_target} "
      "pgp_num_target={pg_placement_num_target} pg_num_pending={pg_num_pending} last_change={last_change} "
      "last_force_op_resend={last_force_op_resend} flags={flags} snap_seq={snap_seq} snap_epoch={snap_epoch} "
      "quota_max_bytes={quota_max_bytes} quota_max_objects={quota_max_objects} tier_of={tier_of} "
      "read_tier={read_tier} write_tier={write_tier} stripe_width={stripe_width} "
      "expected_num_objects={expected_num_objects} fast_read={fast_read} "
      "cache_target_dirty_ratio_micro={cache_target_dirty_ratio_micro} "
      "cache_target_full_ratio_micro={cache_target_full_ratio_micro} "
      "min_read_recency_for_promote={min_read_recency_for_promote} "
      "erasure_code_profile={ecp} snaps={nsnaps} apps={apps}".format(
          ecp=p["erasure_code_profile"] or "-", nsnaps=len(p["pool_snaps"]), apps=apps or "-", **p))
for o in j["osds"]:
    pub = ",".join(f"{a['type']}:{a['addr']}/{a['nonce']}" for a in o["public_addrs"]["addrvec"])
    w(f"osd {o['osd']} up={o['up']} in={o['in']} weight={o['weight']} primary_affinity={o['primary_affinity']} "
      f"up_from={o['up_from']} up_thru={o['up_thru']} down_at={o['down_at']} lost_at={o['lost_at']} "
      f"last_clean_begin={o['last_clean_begin']} last_clean_end={o['last_clean_end']} uuid={o['uuid']} public={pub or '-'}")
for x in j["osd_xinfo"]:
    w(f"xinfo {x['osd']} features={x['features']} old_weight={x['old_weight']} dead_epoch={x['dead_epoch']} "
      f"laggy_interval={x['laggy_interval']} laggy_probability={x['laggy_probability']}")
for a, t in sorted(j["blocklist"].items()):
    w(f"blocklist {a}")
for a, t in sorted(j.get("range_blocklist", {}).items()):
    w(f"range_blocklist {a}")
for t in j["pg_temp"]:
    w(f"pg_temp {t['pgid']} {','.join(map(str, t['osds']))}")
# dumped as a flat array of (pgid, osd) pairs
pt = j["primary_temp"]
for i in range(0, len(pt), 2):
    w(f"primary_temp {pt[i]} {pt[i + 1]}")
for t in j["pg_upmap"]:
    w(f"pg_upmap {t['pgid']} {','.join(map(str, t['osds']))}")
for t in j["pg_upmap_items"]:
    w(f"pg_upmap_items {t['pgid']} {','.join(str(m['from']) + '>' + str(m['to']) for m in t['mappings'])}")
for t in j.get("pg_upmap_primaries", []):
    w(f"pg_upmap_primary {t['pgid']} {t['primary_osd']}")
for name, prof in sorted(j["erasure_code_profiles"].items()):
    w(f"ec_profile {name} {','.join(k + '=' + v for k, v in sorted(prof.items()))}")
for q in j["removed_snaps_queue"]:
    w(f"removed_snaps_queue {q['pool']} {','.join(str(i['begin']) + '~' + str(i['length']) for i in q['snaps'])}")
open(out, "w").write("\n".join(lines) + "\n")
