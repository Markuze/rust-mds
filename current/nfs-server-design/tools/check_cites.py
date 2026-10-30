import re, os, sys
doc = open(sys.argv[1], encoding="utf-8").read()
roots = {"": "/mnt/workspace/ceph-client", "ceph:": "/home/ubuntu/ceph", "rmds:": "/mnt/workspace/rust-mds"}
profile = "Documentation/filesystems/nfs/nfsd-maintainer-entry-profile.rst"
bad = []
count = 0
for tag in re.findall(r"\[src: ([^\]]+)\]", doc):
    for part in re.split(r";\s*", tag):
        part = part.strip()
        m = re.match(r"^(ceph:|rmds:)?([A-Za-z0-9_./\-]+\.[A-Za-z0-9]+|MAINTAINERS|profile):([0-9,\- ]+)$", part)
        if not m:
            continue
        prefix, path, lines = m.group(1) or "", m.group(2), m.group(3)
        if path == "profile":
            path = profile
        full = os.path.join(roots[prefix], path)
        count += 1
        if not os.path.isfile(full):
            bad.append(f"missing file: {prefix}{path}")
            continue
        n = sum(1 for _ in open(full, encoding="utf-8", errors="replace"))
        for rng in lines.replace(" ", "").split(","):
            if not rng:
                continue
            hi = int(rng.split("-")[-1])
            if hi > n:
                bad.append(f"line {rng} beyond {n} in {prefix}{path}")
print(f"checked {count} path citations")
print("\n".join(bad) if bad else "all citations resolve")
