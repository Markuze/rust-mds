#!/usr/bin/env python3
"""Generate CRUSH fixtures for the osdc golden tests.

For every map: <name>.txt (source, when compiled from text), <name>.bin (CrushWrapper
encoding from `crushtool -c` or `crushtool --build`) and <name>.mappings with the expected
results of `crushtool -i <name>.bin --test --show-mappings --rule R --num-rep N
--min-x 0 --max-x 1023 [--weight dev w ...]`. To keep the fixtures small, each
(weights, rule, numrep) case is stored as one digest line over all 1024 inputs plus the
explicit results for x < 16:

    D <weights-id> <rule> <numrep> <fnv1a64 of "x a,b,c\n" lines for x in 0..1023, hex>
    M <weights-id> <rule> <numrep> <x> <comma separated result>

`<name>.weights` lists the device reweight sets used (`id dev:weight,...`; set 0 = none).

Run: python3 -I gen_crush.py <outdir>   (needs crushtool on PATH)
"""
import os
import subprocess
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "crush")
MAX_X = 1023
EXPLICIT_X = 16

TYPES = """type 0 osd
type 1 host
type 2 rack
type 3 row
type 4 root
"""


def devices(n, classes=None):
    out = []
    for i in range(n):
        c = f" class {classes[i % len(classes)]}" if classes else ""
        out.append(f"device {i} osd.{i}{c}")
    return "\n".join(out) + "\n"


def bucket(kind, name, bid, alg, items):
    lines = [f"{kind} {name} {{", f"\tid {bid}", f"\talg {alg}", "\thash 0"]
    for it, w in items:
        lines.append(f"\titem {it} weight {w:.5f}")
    lines.append("}")
    return "\n".join(lines) + "\n"


def rule(name, rid, rtype, steps):
    lines = [f"rule {name} {{", f"\tid {rid}", f"\ttype {rtype}"]
    lines += [f"\tstep {s}" for s in steps]
    lines.append("}")
    return "\n".join(lines) + "\n"


TUNABLES = {
    "optimal": dict(choose_local_tries=0, choose_local_fallback_tries=0, choose_total_tries=50,
                    chooseleaf_descend_once=1, chooseleaf_vary_r=1, chooseleaf_stable=1,
                    straw_calc_version=1, allowed_bucket_algs=62),
    "argonaut": dict(choose_local_tries=2, choose_local_fallback_tries=5, choose_total_tries=19,
                     chooseleaf_descend_once=0, chooseleaf_vary_r=0, chooseleaf_stable=0,
                     straw_calc_version=0, allowed_bucket_algs=62),
    "bobtail": dict(choose_local_tries=0, choose_local_fallback_tries=0, choose_total_tries=50,
                    chooseleaf_descend_once=1, chooseleaf_vary_r=0, chooseleaf_stable=0,
                    straw_calc_version=1, allowed_bucket_algs=62),
    "firefly": dict(choose_local_tries=0, choose_local_fallback_tries=0, choose_total_tries=50,
                    chooseleaf_descend_once=1, chooseleaf_vary_r=1, chooseleaf_stable=0,
                    straw_calc_version=1, allowed_bucket_algs=62),
    "varyr3": dict(choose_local_tries=0, choose_local_fallback_tries=0, choose_total_tries=50,
                   chooseleaf_descend_once=1, chooseleaf_vary_r=3, chooseleaf_stable=0,
                   straw_calc_version=1, allowed_bucket_algs=62),
}


def tunables(name):
    return "".join(f"tunable {k} {v}\n" for k, v in TUNABLES[name].items())


def algs_map(tun):
    """Every bucket algorithm, several rule shapes."""
    ndev = 26
    s = "# begin crush map\n" + tunables(tun) + "\n" + devices(ndev) + "\n" + TYPES + "\n"
    hosts = [
        ("h-uniform", -11, "uniform", [(f"osd.{i}", 1.0) for i in range(0, 4)]),
        ("h-list", -12, "list", [("osd.4", 1.0), ("osd.5", 2.0), ("osd.6", 0.5), ("osd.7", 1.5)]),
        ("h-tree", -13, "tree", [("osd.8", 1.0), ("osd.9", 0.75), ("osd.10", 2.0), ("osd.11", 1.25), ("osd.12", 0.5)]),
        ("h-straw", -14, "straw", [("osd.13", 1.0), ("osd.14", 3.0), ("osd.15", 0.25), ("osd.16", 1.0)]),
        ("h-straw2a", -15, "straw2", [("osd.17", 1.0), ("osd.18", 2.0), ("osd.19", 1.0), ("osd.20", 0.5)]),
        ("h-straw2b", -16, "straw2", [("osd.21", 1.5), ("osd.22", 0.0), ("osd.23", 1.0)]),
        ("h-uniform2", -17, "uniform", [("osd.24", 2.0), ("osd.25", 2.0)]),
    ]
    hw = {}
    for name, bid, alg, items in hosts:
        s += bucket("host", name, bid, alg, items)
        hw[name] = sum(w for _, w in items)
    racks = [
        ("r1", -21, "straw2", ["h-uniform", "h-list", "h-tree"]),
        ("r2", -22, "list", ["h-straw", "h-straw2a"]),
        ("r3", -23, "tree", ["h-straw2b", "h-uniform2"]),
    ]
    rw = {}
    for name, bid, alg, hs in racks:
        s += bucket("rack", name, bid, alg, [(h, hw[h]) for h in hs])
        rw[name] = sum(hw[h] for h in hs)
    s += bucket("root", "default", -1, "straw", [(r, rw[r]) for r, *_ in racks])
    s += bucket("root", "flat", -2, "straw2", [(f"osd.{i}", 1.0 + (i % 3) * 0.5) for i in range(ndev)])
    s += "\n"
    s += rule("rep_host", 0, "replicated", ["take default", "chooseleaf firstn 0 type host", "emit"])
    s += rule("rep_rack_host", 1, "replicated", ["take default", "choose firstn 2 type rack", "chooseleaf firstn 2 type host", "emit"])
    s += rule("ec_host", 2, "erasure", ["set_chooseleaf_tries 5", "set_choose_tries 100", "take default", "chooseleaf indep 0 type host", "emit"])
    s += rule("ec_rack_osd", 3, "erasure", ["take default", "choose indep 3 type rack", "choose indep 2 type osd", "emit"])
    s += rule("rep_osd", 4, "replicated", ["take default", "choose firstn 0 type osd", "emit"])
    s += rule("rep_tree", 5, "replicated", ["take h-tree", "choose firstn 0 type osd", "emit"])
    s += rule("multi_emit", 6, "replicated", ["take r1", "chooseleaf firstn 2 type host", "emit", "take r2", "chooseleaf firstn 1 type host", "emit"])
    s += rule("local_tries", 7, "replicated", ["set_choose_local_tries 2", "set_choose_local_fallback_tries 5", "take default", "chooseleaf firstn 0 type host", "emit"])
    s += rule("vary_stable", 8, "replicated", ["set_chooseleaf_vary_r 0", "set_chooseleaf_stable 0", "take default", "chooseleaf firstn 0 type host", "emit"])
    s += rule("flat_osd", 9, "replicated", ["take flat", "chooseleaf firstn 0 type osd", "emit"])
    s += rule("ec_flat", 10, "erasure", ["take flat", "choose indep 0 type osd", "emit"])
    s += rule("uniform_indep", 11, "erasure", ["take h-uniform", "choose indep 0 type osd", "emit"])
    s += rule("rep_minus", 12, "replicated", ["take default", "chooseleaf firstn -1 type host", "emit"])
    s += "\n# end crush map\n"
    return s, list(range(13))


def classes_map():
    ndev = 18
    s = "# begin crush map\n" + tunables("optimal") + "\n" + devices(ndev, ["hdd", "ssd", "hdd", "nvme"]) + "\n" + TYPES + "\n"
    hosts = []
    for h in range(6):
        items = [(f"osd.{h * 3 + j}", 1.0 + 0.25 * j) for j in range(3)]
        hosts.append((f"node{h}", -2 - h, items))
    for name, bid, items in hosts:
        s += bucket("host", name, bid, "straw2", items)
    s += bucket("root", "default", -1, "straw2", [(n, sum(w for _, w in it)) for n, _, it in hosts])
    s += "\n"
    s += rule("rep", 0, "replicated", ["take default", "chooseleaf firstn 0 type host", "emit"])
    s += rule("rep_ssd", 1, "replicated", ["take default class ssd", "chooseleaf firstn 0 type host", "emit"])
    s += rule("rep_hdd", 2, "replicated", ["take default class hdd", "chooseleaf firstn 0 type host", "emit"])
    s += rule("ec_hdd", 3, "erasure", ["set_chooseleaf_tries 5", "take default class hdd", "chooseleaf indep 0 type host", "emit"])
    s += "\n# end crush map\n"
    return s, [0, 1, 2, 3]


def choose_args_map(index):
    ndev = 12
    s = "# begin crush map\n" + tunables("optimal") + "\n" + devices(ndev) + "\n" + TYPES + "\n"
    hosts = []
    for h in range(4):
        items = [(f"osd.{h * 3 + j}", 1.0) for j in range(3)]
        hosts.append((f"host{h}", -2 - h, items))
    for name, bid, items in hosts:
        s += bucket("host", name, bid, "straw2", items)
    s += bucket("root", "default", -1, "straw2", [(n, 3.0) for n, _, _ in hosts])
    s += "\n"
    s += rule("rep", 0, "replicated", ["take default", "chooseleaf firstn 0 type host", "emit"])
    s += rule("ec", 1, "erasure", ["take default", "chooseleaf indep 0 type host", "emit"])
    s += "\n"
    # positional weight sets on the root and two hosts, replacement ids on host0
    s += f"choose_args {index} {{\n"
    s += "  {\n    bucket_id -1\n    weight_set [\n      [ 1.000 3.000 0.500 2.000 ]\n      [ 2.000 0.500 3.000 1.000 ]\n    ]\n  }\n"
    s += "  {\n    bucket_id -2\n    weight_set [\n      [ 0.100 1.000 2.000 ]\n      [ 1.000 1.000 0.000 ]\n    ]\n    ids [ -100 -101 -102 ]\n  }\n"
    s += "  {\n    bucket_id -3\n    weight_set [\n      [ 1.000 0.000 1.000 ]\n      [ 0.500 1.500 1.000 ]\n    ]\n  }\n"
    s += "}\n"
    s += "\n# end crush map\n"
    return s, [0, 1]


def msr_map():
    ndev = 24
    s = "# begin crush map\n" + tunables("optimal") + "tunable msr_descents 100\ntunable msr_collision_tries 100\n\n" + devices(ndev) + "\n" + TYPES + "\n"
    hosts = []
    for h in range(6):
        items = [(f"osd.{h * 4 + j}", 1.0) for j in range(4)]
        hosts.append((f"host{h}", -2 - h, items))
    for name, bid, items in hosts:
        s += bucket("host", name, bid, "straw2", items)
    s += bucket("root", "default", -1, "straw2", [(n, 4.0) for n, _, _ in hosts])
    s += "\n"
    s += rule("msr_indep_4x2", 0, "msr_indep", ["take default", "choosemsr 4 type host", "choosemsr 2 type osd", "emit"])
    s += rule("msr_firstn_3x2", 1, "msr_firstn", ["take default", "choosemsr 3 type host", "choosemsr 2 type osd", "emit"])
    s += rule("msr_indep_hosts", 2, "msr_indep", ["set_msr_descents 20", "set_msr_collision_tries 10", "take default", "choosemsr 0 type host", "choosemsr 1 type osd", "emit"])
    s += rule("msr_firstn_osd", 3, "msr_firstn", ["take default", "choosemsr 0 type osd", "emit"])
    s += "\n# end crush map\n"
    return s, [0, 1, 2, 3]


def run(*args, check=True):
    r = subprocess.run(list(args), capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"{' '.join(args)} failed: {r.stderr}")
    return r.stdout + r.stderr


WEIGHT_SETS = {
    0: [],
    1: [(1, 0.0), (5, 0.5), (9, 0.25), (14, 0.0), (18, 0.75), (21, 0.1)],
    2: [(0, 0.0), (2, 0.0), (3, 0.3), (17, 0.0), (19, 0.9)],
}


def mappings(binf, name, rules, numreps, weight_sets):
    lines = []
    for wid in weight_sets:
        wargs = []
        for dev, w in WEIGHT_SETS[wid]:
            wargs += ["--weight", str(dev), str(w)]
        for r in rules:
            for n in numreps:
                out = run("crushtool", "-i", binf, "--test", "--show-mappings", "--rule", str(r), "--num-rep", str(n),
                          "--min-x", "0", "--max-x", str(MAX_X), *wargs)
                h = 0xcbf29ce484222325
                count = 0
                for line in out.splitlines():
                    if not line.startswith("CRUSH rule "):
                        continue
                    parts = line.split(" ", 5)
                    # CRUSH rule R x X [..]
                    vec = parts[5].strip("[]").replace(" ", "")
                    x = int(parts[4])
                    for b in f"{x} {vec}\n".encode():
                        h ^= b
                        h = (h * 0x100000001b3) & 0xffffffffffffffff
                    count += 1
                    if x < EXPLICIT_X:
                        lines.append(f"M {wid} {parts[2]} {n} {x} {vec}")
                if count != MAX_X + 1:
                    raise SystemExit(f"{name}: rule {r} numrep {n}: {count} mappings")
                lines.append(f"D {wid} {r} {n} {h:016x}")
    with open(os.path.join(OUT, name + ".mappings"), "w") as f:
        f.write("\n".join(lines) + "\n")
    with open(os.path.join(OUT, name + ".weights"), "w") as f:
        for wid in weight_sets:
            f.write(f"{wid} " + ",".join(f"{d}:{w}" for d, w in WEIGHT_SETS[wid]) + "\n")
    return len(lines)


def compile_text(name, text, unsafe=False):
    txt = os.path.join(OUT, name + ".txt")
    binf = os.path.join(OUT, name + ".bin")
    with open(txt, "w") as f:
        f.write(text)
    args = ["crushtool", "-c", txt, "-o", binf]
    if unsafe:
        args.append("--enable-unsafe-tunables")
    run(*args)
    return binf


def main():
    os.makedirs(OUT, exist_ok=True)
    total = 0
    for tun in ["optimal", "argonaut", "bobtail", "firefly", "varyr3"]:
        text, rules = algs_map(tun)
        binf = compile_text(f"algs-{tun}", text, unsafe=True)
        total += mappings(binf, f"algs-{tun}", rules, [1, 3, 5], [0, 1, 2] if tun == "optimal" else [0, 1])
    text, rules = classes_map()
    binf = compile_text("classes", text)
    total += mappings(binf, "classes", rules, [2, 3, 4], [0, 1])
    # crushtool --test uses choose_args index 0 (with fallback to -1, which the text compiler
    # cannot express); index 7 checks that other indexes are ignored.
    for idx in [0, 7]:
        text, rules = choose_args_map(idx)
        nm = f"choose-args-{idx}"
        binf = compile_text(nm, text)
        total += mappings(binf, nm, rules, [1, 2, 3], [0])
    text, rules = msr_map()
    binf = compile_text("msr", text)
    total += mappings(binf, "msr", rules, [3, 6, 8], [0, 1])
    # built hierarchies with every algorithm at various levels
    builds = {
        "build-straw2": ["--num_osds", "120", "host", "straw2", "5", "rack", "straw2", "4", "row", "straw2", "3", "root", "straw2", "0"],
        "build-mixed": ["--num_osds", "64", "host", "uniform", "4", "rack", "list", "4", "root", "straw", "0"],
        "build-tree": ["--num_osds", "40", "host", "tree", "5", "root", "tree", "0"],
    }
    for nm, layers in builds.items():
        binf = os.path.join(OUT, nm + ".bin")
        run("crushtool", "-o", binf, "--build", *layers)
        # add an indep rule across hosts and a firstn rule across the second layer
        layer2 = layers[5]
        run("crushtool", "-i", binf, "-o", binf, "--create-simple-rule", "ec_host", "root", "host", "indep")
        run("crushtool", "-i", binf, "-o", binf, "--create-replicated-rule", "rep_l2", "root", layer2)
        rules = [0, 1, 2]
        total += mappings(binf, nm, rules, [2, 3, 6], [0, 1])
    print(f"wrote {total} mappings to {OUT}")


if __name__ == "__main__":
    main()
