Golden msgr1 data from a real Linux 7.0 kernel client talking to the Ceph 20.2.0 C++ MDS
(dev cluster fsid 3ceca2c6-851f-41cf-b538-1cfded4f1e82, 2026-10-11T19:06:19Z).

Source: testdata/captures/kernel-v1.pcap (env agent), split with testdata/msgr1_split.py;
connection 1 = kernel (client.admin, global_id 14202) -> mds.a v1:127.0.0.1:7401.

  kernel_authorizer0.bin        first cephx authorizer in ceph_msg_connect (no challenge answer)
  cpp_challenge.bin             the C++ MDS's CHALLENGE_AUTHORIZER payload (raw ciphertext)
  kernel_authorizer1.bin        the kernel's second authorizer, answering the challenge
  cpp_authorizer_reply.bin      the C++ MDS's authorizer reply in the SEQ connect reply
  NNNN-c1-DIR-TYPE.frame.bin    complete MSG frames (tag, header, front, middle, data,
                                21-byte footer with the cephx signature); c2s = kernel-signed,
                                s2c = C++-signed

The tests also need the cluster's rotating "mds" service secret with id 2 (the ticket's
secret_id), fetched at capture time by authenticating as mds.rust. It is not committed: it is
RMDS_TEST_MDS_ROTATING_SECRET_2 in the workspace's git-ignored .env (see .env.example).

Negotiated features: kernel 0x2f018fb87aa4aafe & C++ MDS 0x3f07fffffffdffff = 0x2f018fb87aa4aafe.
