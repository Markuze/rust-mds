Front payloads of msgr1 messages between the Ceph 20.2.0 C++ MDS (mds.a, bound to msgr1
only) and the monitor during boot, from testdata/captures/mds-v1-boot.pcap (split with
testdata/msgr1_split.py; NNNN = message number in that capture's index.tsv).
Cluster fsid 3ceca2c6-851f-41cf-b538-1cfded4f1e82, 2026-10-11T19:11Z.

The cephx exchange (0002-0012) can be replayed with mds.a's key in force at capture time,
RMDS_TEST_MDS_A_KEY in the workspace's git-ignored .env (see .env.example); keys are never
committed.
Expected MDSMap values come from `ceph fs dump <epoch> -f json` on the same cluster.
