#!/usr/bin/env python3
"""Split msgr1 (Ceph "v027" legacy messenger) TCP streams in a pcap into messages.

    python3 -I msgr1_split.py <capture.pcap> <outdir> [--no-dencoder]

For every TCP connection whose SYN is in the capture and that starts with the
msgr1 banner "ceph v027", the handshake and every tagged frame are parsed:

  client -> server: banner(9) + ceph_entity_addr(136)
                    + N x (ceph_msg_connect(33) + authorizer)       [+ u64 in_seq if TAG_SEQ]
  server -> client: banner(9) + ceph_entity_addr(136) + peer addr(136)
                    + N x (ceph_msg_connect_reply(26) + authorizer_reply [+ u64 seq if TAG_SEQ])
  then frames:      tag(1) + payload; TAG_MSG = ceph_msg_header(53) + front + middle
                    + data + ceph_msg_footer(21; 13 without CEPH_FEATURE_MSG_AUTH)

(include/msgr.h, msg/async/ProtocolV1.cc.)  Output in <outdir>:

  index.tsv                      one line per frame (time, connection, direction,
                                 tag, seq, tid, type, version, sizes, src, file)
  connections.tsv                one line per connection (ports, entity types,
                                 connect/connect_reply fields of the last round)
  NNNN-cC-DIR-TYPE.front.bin     front payload (what Message::decode_payload reads);
                                 .middle.bin / .data.bin when non-empty
  NNNN-cC-DIR-TYPE.dencoder.bin  the message in encode_message() layout
                                 (header + old footer + u32-length-prefixed front,
                                 middle, data), for
                                 ceph-dencoder type TYPE import F decode dump_json
  NNNN-cC-DIR-TYPE.json          that ceph-dencoder dump (if ceph-dencoder is
                                 installed and knows TYPE; .json.err otherwise)
  cC-DIR.authorizerK.bin         authorizer / authorizer_reply bytes of connect round K

DIR is c2s (from the connection's initiator) or s2c. Only the Python standard
library is used.
"""
import os
import struct
import subprocess
import sys

TYPES = {
    1: ("CEPH_MSG_SHUTDOWN", "MGenericMessage"), 2: ("CEPH_MSG_PING", "MPing"), 4: ("CEPH_MSG_MON_MAP", "MMonMap"),
    5: ("CEPH_MSG_MON_GET_MAP", "MMonGetMap"), 6: ("CEPH_MSG_MON_GET_OSDMAP", "MMonGetOSDMap"), 13: ("CEPH_MSG_STATFS", "MStatfs"),
    14: ("CEPH_MSG_STATFS_REPLY", "MStatfsReply"), 15: ("CEPH_MSG_MON_SUBSCRIBE", "MMonSubscribe"), 16: ("CEPH_MSG_MON_SUBSCRIBE_ACK", "MMonSubscribeAck"),
    17: ("CEPH_MSG_AUTH", "MAuth"), 18: ("CEPH_MSG_AUTH_REPLY", "MAuthReply"), 19: ("CEPH_MSG_MON_GET_VERSION", "MMonGetVersion"),
    20: ("CEPH_MSG_MON_GET_VERSION_REPLY", "MMonGetVersionReply"), 21: ("CEPH_MSG_MDS_MAP", "MMDSMap"), 22: ("CEPH_MSG_CLIENT_SESSION", "MClientSession"),
    23: ("CEPH_MSG_CLIENT_RECONNECT", "MClientReconnect"), 24: ("CEPH_MSG_CLIENT_REQUEST", "MClientRequest"), 25: ("CEPH_MSG_CLIENT_REQUEST_FORWARD", "MClientRequestForward"),
    26: ("CEPH_MSG_CLIENT_REPLY", "MClientReply"), 27: ("CEPH_MSG_CLIENT_RECLAIM", "MClientReclaim"), 28: ("CEPH_MSG_CLIENT_RECLAIM_REPLY", "MClientReclaimReply"),
    29: ("CEPH_MSG_CLIENT_METRICS", "MClientMetrics"), 41: ("CEPH_MSG_OSD_MAP", "MOSDMap"), 42: ("CEPH_MSG_OSD_OP", "MOSDOp"),
    43: ("CEPH_MSG_OSD_OPREPLY", "MOSDOpReply"), 44: ("CEPH_MSG_WATCH_NOTIFY", "MWatchNotify"), 45: ("CEPH_MSG_FS_MAP", "MFSMap"),
    46: ("MSG_FORWARD", "MForward"), 47: ("MSG_ROUTE", "MRoute"), 48: ("CEPH_MSG_POOLOP_REPLY", "MPoolOpReply"),
    49: ("CEPH_MSG_POOLOP", "MPoolOp"), 50: ("MSG_MON_COMMAND", "MMonCommand"), 51: ("MSG_MON_COMMAND_ACK", "MMonCommandAck"),
    52: ("MSG_LOG", "MLog"), 53: ("MSG_LOGACK", "MLogAck"), 54: ("MSG_KV_DATA", "MKVData"),
    58: ("MSG_GETPOOLSTATS", "MGetPoolStats"), 59: ("MSG_GETPOOLSTATSREPLY", "MGetPoolStatsReply"), 60: ("MSG_MON_GLOBAL_ID", "MMonGlobalID"),
    61: ("CEPH_MSG_OSD_BACKOFF", "MOSDBackoff"), 62: ("MSG_CONFIG", "MConfig"), 63: ("MSG_GET_CONFIG", "MGetConfig"),
    64: ("MSG_MON_SCRUB", "MMonScrub"), 65: ("MSG_MON_ELECTION", "MMonElection"), 66: ("MSG_MON_PAXOS", "MMonPaxos"),
    67: ("MSG_MON_PROBE", "MMonProbe"), 68: ("MSG_MON_JOIN", "MMonJoin"), 69: ("MSG_MON_SYNC", "MMonSync"),
    70: ("MSG_OSD_PING", "MOSDPing"), 71: ("MSG_OSD_BOOT", "MOSDBoot"), 72: ("MSG_OSD_FAILURE", "MOSDFailure"),
    73: ("MSG_OSD_ALIVE", "MOSDAlive"), 74: ("MSG_OSD_MARK_ME_DOWN", "MOSDMarkMeDown"), 75: ("MSG_OSD_FULL", "MOSDFull"),
    76: ("MSG_MON_GET_PURGED_SNAPS", "MMonGetPurgedSnaps"), 77: ("MSG_MON_GET_PURGED_SNAPS_REPLY", "MMonGetPurgedSnapsReply"), 78: ("MSG_OSD_PGTEMP", "MOSDPGTemp"),
    79: ("MSG_OSD_BEACON", "MOSDBeacon"), 80: ("MSG_OSD_PG_NOTIFY", "MOSDPGNotify"), 81: ("MSG_OSD_PG_QUERY", "MOSDPGQuery"),
    83: ("MSG_OSD_PG_LOG", "MOSDPGLog"), 84: ("MSG_OSD_PG_REMOVE", "MOSDPGRemove"), 85: ("MSG_OSD_PG_INFO", "MOSDPGInfo"),
    86: ("MSG_OSD_PG_TRIM", "MOSDPGTrim"), 87: ("MSG_PGSTATS", "MPGStats"), 88: ("MSG_PGSTATSACK", "MPGStatsAck"),
    90: ("MSG_REMOVE_SNAPS", "MRemoveSnaps"), 92: ("MSG_OSD_SCRUB_RESERVE", "MOSDScrubReserve"), 93: ("MSG_OSD_REP_SCRUB", "MOSDRepScrub"),
    94: ("MSG_OSD_PG_SCAN", "MOSDPGScan"), 95: ("MSG_OSD_PG_BACKFILL", "MOSDPGBackfill"), 96: ("MSG_OSD_PG_BACKFILL_REMOVE", "MOSDPGBackfillRemove"),
    97: ("MSG_COMMAND", "MCommand"), 98: ("MSG_COMMAND_REPLY", "MCommandReply"), 99: ("MSG_OSD_BACKFILL_RESERVE", "MBackfillReserve"),
    100: ("MSG_MDS_BEACON", "MMDSBeacon"), 101: ("MSG_MDS_PEER_REQUEST", "MMDSPeerRequest"), 102: ("MSG_MDS_TABLE_REQUEST", "MMDSTableRequest"),
    103: ("CEPH_MSG_FS_MAP_USER", "MFSMapUser"), 105: ("MSG_OSD_PG_PUSH", "MOSDPGPush"), 106: ("MSG_OSD_PG_PULL", "MOSDPGPull"),
    107: ("MSG_OSD_PG_PUSH_REPLY", "MOSDPGPushReply"), 108: ("MSG_OSD_EC_WRITE", "MOSDECSubOpWrite"), 109: ("MSG_OSD_EC_WRITE_REPLY", "MOSDECSubOpWriteReply"),
    110: ("MSG_OSD_EC_READ", "MOSDECSubOpRead"), 111: ("MSG_OSD_EC_READ_REPLY", "MOSDECSubOpReadReply"), 112: ("MSG_OSD_REPOP", "MOSDRepOp"),
    113: ("MSG_OSD_REPOPREPLY", "MOSDRepOpReply"), 114: ("MSG_OSD_PG_UPDATE_LOG_MISSING", "MOSDPGUpdateLogMissing"), 115: ("MSG_OSD_PG_UPDATE_LOG_MISSING_REPLY", "MOSDPGUpdateLogMissingReply"),
    116: ("MSG_OSD_PG_CREATED", "MOSDPGCreated"), 117: ("MSG_OSD_REP_SCRUBMAP", "MOSDRepScrubMap"), 118: ("MSG_OSD_PG_RECOVERY_DELETE", "MOSDPGRecoveryDelete"),
    119: ("MSG_OSD_PG_RECOVERY_DELETE_REPLY", "MOSDPGRecoveryDeleteReply"), 120: ("MSG_OSD_PG_CREATE2", "MOSDPGCreate2"), 121: ("MSG_OSD_SCRUB2", "MOSDScrub2"),
    122: ("MSG_OSD_PG_READY_TO_MERGE", "MOSDPGReadyToMerge"), 123: ("MSG_OSD_MARK_ME_DEAD", "MOSDMarkMeDead"), 124: ("MSG_OSD_PG_STOP_MERGE", "MOSDPGStopMerge"),
    130: ("MSG_OSD_PG_NOTIFY2", "MOSDPGNotify2"), 131: ("MSG_OSD_PG_QUERY2", "MOSDPGQuery2"), 132: ("MSG_OSD_PG_INFO2", "MOSDPGInfo2"),
    133: ("MSG_OSD_PG_LEASE", "MOSDPGLease"), 134: ("MSG_OSD_PG_LEASE_ACK", "MOSDPGLeaseAck"), 135: ("MSG_MDS_SCRUB", "MMDSScrub"),
    136: ("MSG_OSD_PG_PCT", "MOSDPGPCT"), 140: ("MSG_MON_PING", "MMonPing"), 141: ("MSG_MON_USED_PENDING_KEYS", "MMonUsedPendingKeys"),
    150: ("MSG_OSD_RECOVERY_RESERVE", "MRecoveryReserve"), 151: ("MSG_OSD_FORCE_RECOVERY", "MOSDForceRecovery"), 512: ("MSG_MDS_RESOLVE", "MMDSResolve"),
    513: ("MSG_MDS_RESOLVEACK", "MMDSResolveAck"), 514: ("MSG_MDS_CACHEREJOIN", "MMDSCacheRejoin"), 515: ("MSG_MDS_DISCOVER", "MDiscover"),
    516: ("MSG_MDS_DISCOVERREPLY", "MDiscoverReply"), 517: ("MSG_MDS_INODEUPDATE", "MInodeUpdate"), 518: ("MSG_MDS_DIRUPDATE", "MDirUpdate"),
    519: ("MSG_MDS_CACHEEXPIRE", "MCacheExpire"), 520: ("MSG_MDS_DENTRYUNLINK", "MDentryUnlink"), 521: ("MSG_MDS_FRAGMENTNOTIFY", "MMDSFragmentNotify"),
    522: ("MSG_MDS_OFFLOAD_TARGETS", "MMDSLoadTargets"), 524: ("MSG_MDS_DENTRYLINK", "MDentryLink"), 525: ("MSG_MDS_FINDINO", "MMDSFindIno"),
    526: ("MSG_MDS_FINDINOREPLY", "MMDSFindInoReply"), 527: ("MSG_MDS_OPENINO", "MMDSOpenIno"), 528: ("MSG_MDS_OPENINOREPLY", "MMDSOpenInoReply"),
    529: ("MSG_MDS_SNAPUPDATE", "MMDSSnapUpdate"), 530: ("MSG_MDS_FRAGMENTNOTIFYACK", "MMDSFragmentNotifyAck"), 768: ("MSG_MDS_LOCK", "MLock"),
    769: ("MSG_MDS_INODEFILECAPS", "MInodeFileCaps"), 784: ("CEPH_MSG_CLIENT_CAPS", "MClientCaps"), 785: ("CEPH_MSG_CLIENT_LEASE", "MClientLease"),
    786: ("CEPH_MSG_CLIENT_SNAP", "MClientSnap"), 787: ("CEPH_MSG_CLIENT_CAPRELEASE", "MClientCapRelease"), 788: ("CEPH_MSG_CLIENT_QUOTA", "MClientQuota"),
    789: ("CEPH_MSG_CLIENT_QUARANTINE_DISABLE", "MQuarantineDisable"), 1097: ("MSG_MDS_EXPORTDIRDISCOVER", "MExportDirDiscover"), 1104: ("MSG_MDS_EXPORTDIRDISCOVERACK", "MExportDirDiscoverAck"),
    1105: ("MSG_MDS_EXPORTDIRCANCEL", "MExportDirCancel"), 1106: ("MSG_MDS_EXPORTDIRPREP", "MExportDirPrep"), 1107: ("MSG_MDS_EXPORTDIRPREPACK", "MExportDirPrepAck"),
    1110: ("MSG_MDS_EXPORTDIR", "MExportDir"), 1111: ("MSG_MDS_EXPORTDIRACK", "MExportDirAck"), 1112: ("MSG_MDS_EXPORTDIRNOTIFY", "MExportDirNotify"),
    1113: ("MSG_MDS_EXPORTDIRNOTIFYACK", "MExportDirNotifyAck"), 1120: ("MSG_MDS_EXPORTDIRFINISH", "MExportDirFinish"), 1136: ("MSG_MDS_EXPORTCAPS", "MExportCaps"),
    1137: ("MSG_MDS_EXPORTCAPSACK", "MExportCapsAck"), 1138: ("MSG_MDS_GATHERCAPS", "MGatherCaps"), 1280: ("MSG_MDS_HEARTBEAT", "MHeartbeat"),
    1281: ("MSG_MDS_METRICS", "MMDSMetrics"), 1282: ("MSG_MDS_PING", "MMDSPing"), 1283: ("MSG_MDS_SCRUB_STATS", "MMDSScrubStats"),
    1285: ("MSG_MDS_QUIESCE_DB_LISTING", "MMDSQuiesceDbListing"), 1286: ("MSG_MDS_QUIESCE_DB_ACK", "MMDSQuiesceDbAck"), 1536: ("MSG_TIMECHECK", "MTimeCheck"),
    1537: ("MSG_MON_HEALTH", "MMonHealth"), 1544: ("MSG_MON_HEALTH_CHECKS", "MMonHealthChecks"), 1545: ("MSG_TIMECHECK2", "MTimeCheck2"),
    1792: ("MSG_MGR_OPEN", "MMgrOpen"), 1793: ("MSG_MGR_CONFIGURE", "MMgrConfigure"), 1794: ("MSG_MGR_REPORT", "MMgrReport"),
    1795: ("MSG_MGR_BEACON", "MMgrBeacon"), 1796: ("MSG_MGR_MAP", "MMgrMap"), 1797: ("MSG_MGR_DIGEST", "MMgrDigest"),
    1798: ("MSG_MON_MGR_REPORT", "MMonMgrReport"), 1799: ("MSG_SERVICE_MAP", "MServiceMap"), 1800: ("MSG_MGR_CLOSE", "MMgrClose"),
    1801: ("MSG_MGR_COMMAND", "MMgrCommand"), 1802: ("MSG_MGR_COMMAND_REPLY", "MMgrCommandReply"), 1803: ("MSG_MGR_UPDATE", "MMgrUpdate"),
    2048: ("MSG_MNVMEOF_GW_MAP", "MNVMeofGwMap"), 2049: ("MSG_MNVMEOF_GW_BEACON", "MNVMeofGwBeacon"),
}

TAGS = {1: "READY", 2: "RESETSESSION", 3: "WAIT", 4: "RETRY_SESSION", 5: "RETRY_GLOBAL",
        6: "CLOSE", 7: "MSG", 8: "ACK", 9: "KEEPALIVE", 10: "BADPROTOVER",
        11: "BADAUTHORIZER", 12: "FEATURES", 13: "SEQ", 14: "KEEPALIVE2",
        15: "KEEPALIVE2_ACK", 16: "CHALLENGE_AUTHORIZER"}
ENTITY = {0: "?", 1: "mon", 2: "mds", 4: "osd", 8: "client", 0x10: "mgr", 0x20: "auth"}
BANNER = b"ceph v027"
ADDR_LEN = 136            # struct ceph_entity_addr: type, nonce, sockaddr_storage
CONNECT_LEN = 33          # struct ceph_msg_connect
REPLY_LEN = 26            # struct ceph_msg_connect_reply
HEADER_LEN = 53           # struct ceph_msg_header
FEATURE_MSG_AUTH = 1 << 23


def read_pcap(path):
    """Yield (time, src, sport, dst, dport, flags, seq, payload) for IPv4 TCP packets."""
    with open(path, "rb") as f:
        gh = f.read(24)
        magic = gh[:4]
        if magic in (b"\xd4\xc3\xb2\xa1", b"\x4d\x3c\xb2\xa1"):
            e = "<"
        elif magic in (b"\xa1\xb2\xc3\xd4", b"\xa1\xb2\x3c\x4d"):
            e = ">"
        else:
            raise SystemExit("not a classic pcap file (pcapng is not supported)")
        nsec = magic in (b"\x4d\x3c\xb2\xa1", b"\xa1\xb2\x3c\x4d")
        linktype = struct.unpack(e + "I", gh[20:24])[0]
        skip = {1: 14, 113: 16, 276: 20, 0: 4, 101: 0}.get(linktype)
        if skip is None:
            raise SystemExit(f"unsupported link type {linktype}")
        while True:
            rh = f.read(16)
            if len(rh) < 16:
                return
            ts, tf, incl, _ = struct.unpack(e + "IIII", rh)
            pkt = f.read(incl)
            t = ts + tf / (1e9 if nsec else 1e6)
            ip = pkt[skip:]
            if len(ip) < 20 or ip[0] >> 4 != 4 or ip[9] != 6:
                continue
            ihl = (ip[0] & 15) * 4
            total = struct.unpack(">H", ip[2:4])[0]
            ip = ip[:total]
            tcp = ip[ihl:]
            sport, dport, seq = struct.unpack(">HHI", tcp[:8])
            off = (tcp[12] >> 4) * 4
            flags = tcp[13]
            src = ".".join(map(str, ip[12:16]))
            dst = ".".join(map(str, ip[16:20]))
            yield t, src, sport, dst, dport, flags, seq, tcp[off:]


class Stream:
    """One direction of a TCP connection, reassembled in sequence order."""

    def __init__(self):
        self.isn = None
        self.next = None
        self.buf = bytearray()
        self.times = []          # (offset, time) of each appended segment
        self.pending = {}

    def add(self, seq, data, t):
        if self.next is None:
            return
        if seq != self.next:
            delta = (seq - self.next) & 0xFFFFFFFF
            if delta >= 0x80000000:          # old data, maybe partly new
                old = (self.next - seq) & 0xFFFFFFFF
                if old >= len(data):
                    return
                data, seq = data[old:], self.next
            else:
                self.pending[seq] = (data, t)
                return
        self.times.append((len(self.buf), t))
        self.buf += data
        self.next = (self.next + len(data)) & 0xFFFFFFFF
        while self.next in self.pending:
            d, tt = self.pending.pop(self.next)
            self.add(self.next, d, tt)

    def time_at(self, off):
        t = None
        for o, tt in self.times:
            if o > off:
                break
            t = tt
        return t


class Short(Exception):
    pass


class Reader:
    def __init__(self, buf):
        self.b = bytes(buf)
        self.p = 0

    def take(self, n):
        if self.p + n > len(self.b):
            raise Short()
        r = self.b[self.p:self.p + n]
        self.p += n
        return r


def parse_addr(b):
    typ, nonce, fam = struct.unpack("<IIH", b[:10])
    port = struct.unpack(">H", b[10:12])[0]
    ip = ".".join(map(str, b[12:16]))
    return f"{ip}:{port}/{nonce}"


def parse_handshake(c2s, s2c):
    """Returns (c2s Reader, s2c Reader, info dict) positioned at the first frame."""
    rc, rs = Reader(c2s.buf), Reader(s2c.buf)
    info = {"rounds": []}
    if rc.take(9) != BANNER or rs.take(9) != BANNER:
        raise ValueError("no msgr1 banner")
    info["client_addr"] = parse_addr(rc.take(ADDR_LEN))
    info["server_addr"] = parse_addr(rs.take(ADDR_LEN))
    info["client_addr_seen_by_server"] = parse_addr(rs.take(ADDR_LEN))
    while True:
        c = struct.unpack("<QIIIIIIB", rc.take(CONNECT_LEN))
        cauth = rc.take(c[6])
        r = struct.unpack("<BQIIIIB", rs.take(REPLY_LEN))
        rauth = rs.take(r[5])
        rnd = {"connect": dict(zip(("features", "host_type", "global_seq", "connect_seq",
                                    "protocol_version", "authorizer_protocol",
                                    "authorizer_len", "flags"), c)),
               "reply": dict(zip(("tag", "features", "global_seq", "connect_seq",
                                  "protocol_version", "authorizer_len", "flags"), r)),
               "authorizer": cauth, "authorizer_reply": rauth}
        info["rounds"].append(rnd)
        tag = r[0]
        if tag == 13:                       # SEQ: server acked seq, client in_seq
            rnd["server_acked_seq"] = struct.unpack("<Q", rs.take(8))[0]
            rnd["client_in_seq"] = struct.unpack("<Q", rc.take(8))[0]
        if tag in (1, 13):
            info["features"] = c[0] & r[1]
            info["client_type"] = c[1]
            return rc, rs, info
        if tag not in (2, 3, 4, 5, 16):     # failure tags: the connection ends
            raise ValueError(f"handshake ended with tag {TAGS.get(tag, tag)}")


def parse_frames(r, footer_len):
    """Yield (offset, tag, fields) until the stream ends."""
    while True:
        start = r.p
        try:
            tag = r.take(1)[0]
            if tag == 7:
                h = r.take(HEADER_LEN)
                (seq, tid, typ, prio, ver, front_len, middle_len, data_len, data_off,
                 src_type, src_num, compat, _res, crc) = struct.unpack("<QQHHHIIIHBQHHI", h)
                front = r.take(front_len)
                middle = r.take(middle_len)
                data = r.take(data_len)
                footer = r.take(footer_len)
                yield start, tag, dict(header=h, seq=seq, tid=tid, type=typ, version=ver,
                                       compat=compat, front=front, middle=middle, data=data,
                                       footer=footer, src=f"{ENTITY.get(src_type, src_type)}.{src_num}",
                                       priority=prio, data_off=data_off)
            elif tag in (8, 14, 15):
                yield start, tag, dict(value=r.take(8))
            elif tag in (6, 9):
                yield start, tag, {}
            else:
                yield start, tag, dict(error=f"unexpected tag {tag}")
                return
        except Short:
            return


def dencoder_blob(m):
    """encode_message() layout: header, ceph_msg_footer_old, front, middle, data."""
    f = m["footer"]
    old_footer = f[:12] + f[-1:]           # front_crc, middle_crc, data_crc, flags
    out = m["header"] + old_footer
    for part in (m["front"], m["middle"], m["data"]):
        out += struct.pack("<I", len(part)) + part
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    use_dencoder = "--no-dencoder" not in sys.argv
    if len(args) != 2:
        raise SystemExit(__doc__)
    pcap, outdir = args
    os.makedirs(outdir, exist_ok=True)
    have_dencoder = use_dencoder and subprocess.run(
        ["sh", "-c", "command -v ceph-dencoder"], capture_output=True).returncode == 0

    streams, conns, order = {}, {}, []
    for t, src, sp, dst, dp, flags, seq, payload in read_pcap(pcap):
        key = (src, sp, dst, dp)
        st = streams.setdefault(key, Stream())
        if flags & 0x02:                    # SYN
            st.isn = seq
            st.next = (seq + 1) & 0xFFFFFFFF
            if not flags & 0x10:            # SYN without ACK: initiator
                ck = (src, sp, dst, dp)
                if ck not in conns:
                    conns[ck] = len(order)
                    order.append((ck, t))
            continue
        if payload:
            st.add(seq, payload, t)

    frames, conn_rows = [], []
    for cid, (ck, t0) in enumerate(order):
        src, sp, dst, dp = ck
        c2s, s2c = streams.get(ck), streams.get((dst, dp, src, sp))
        if not c2s or not s2c or not c2s.buf.startswith(BANNER):
            continue
        try:
            rc, rs, info = parse_handshake(c2s, s2c)
        except (Short, ValueError) as ex:
            conn_rows.append((cid, sp, dp, f"handshake not parsed: {ex}"))
            continue
        footer_len = 21 if info["features"] & FEATURE_MSG_AUTH else 13
        last = info["rounds"][-1]
        conn_rows.append((cid, sp, dp,
                          f"client={ENTITY.get(info['client_type'], info['client_type'])} "
                          f"client_addr={info['client_addr']} server_addr={info['server_addr']} "
                          f"rounds={len(info['rounds'])} "
                          f"connect.features=0x{last['connect']['features']:x} "
                          f"reply.features=0x{last['reply']['features']:x} "
                          f"reply.tag={TAGS.get(last['reply']['tag'])} "
                          f"connect.flags={last['connect']['flags']} "
                          f"authorizer_protocol={last['connect']['authorizer_protocol']}"))
        for k, rnd in enumerate(info["rounds"]):
            for d, blob in (("c2s", rnd["authorizer"]), ("s2c", rnd["authorizer_reply"])):
                if blob:
                    with open(os.path.join(outdir, f"c{cid}-{d}.authorizer{k}.bin"), "wb") as f:
                        f.write(blob)
        for d, r, st in (("c2s", rc, c2s), ("s2c", rs, s2c)):
            for off, tag, fr in parse_frames(r, footer_len):
                frames.append((st.time_at(off) or t0, cid, d, sp, dp, tag, fr))

    frames.sort(key=lambda x: (x[0], x[1]))
    with open(os.path.join(outdir, "index.tsv"), "w") as idx:
        idx.write("n\ttime\tconn\tdir\tclient_port\tserver_port\ttag\tseq\ttid\ttype\tname"
                  "\tversion\tcompat\tfront\tmiddle\tdata\tsrc\tfile\tdecode\n")
        n = 0
        for t, cid, d, sp, dp, tag, fr in frames:
            if tag != 7:
                v = struct.unpack("<Q", fr["value"])[0] if "value" in fr else ""
                idx.write(f"-\t{t:.6f}\t{cid}\t{d}\t{sp}\t{dp}\t{TAGS.get(tag, tag)}\t{v}"
                          f"\t\t\t\t\t\t\t\t\t\t\t{fr.get('error', '')}\n")
                continue
            n += 1
            const, cls = TYPES.get(fr["type"], (f"type{fr['type']}", None))
            base = f"{n:04d}-c{cid}-{d}-{cls or const}"
            for part in ("front", "middle", "data"):
                if fr[part]:
                    with open(os.path.join(outdir, f"{base}.{part}.bin"), "wb") as f:
                        f.write(fr[part])
            with open(os.path.join(outdir, f"{base}.dencoder.bin"), "wb") as f:
                f.write(dencoder_blob(fr))
            decode = ""
            if have_dencoder and cls:
                p = subprocess.run(["ceph-dencoder", "type", cls, "import",
                                    os.path.join(outdir, f"{base}.dencoder.bin"),
                                    "decode", "dump_json"], capture_output=True, text=True)
                if p.returncode == 0:
                    decode = "ok"
                    with open(os.path.join(outdir, f"{base}.json"), "w") as f:
                        f.write(p.stdout)
                else:
                    decode = "error"
                    with open(os.path.join(outdir, f"{base}.json.err"), "w") as f:
                        f.write(p.stdout + p.stderr)
            idx.write(f"{n}\t{t:.6f}\t{cid}\t{d}\t{sp}\t{dp}\tMSG\t{fr['seq']}\t{fr['tid']}"
                      f"\t{fr['type']}\t{const}\t{fr['version']}\t{fr['compat']}"
                      f"\t{len(fr['front'])}\t{len(fr['middle'])}\t{len(fr['data'])}"
                      f"\t{fr['src']}\t{base}\t{decode}\n")
    with open(os.path.join(outdir, "connections.tsv"), "w") as f:
        f.write("conn\tclient_port\tserver_port\tinfo\n")
        for row in conn_rows:
            f.write("\t".join(map(str, row)) + "\n")
    print(f"{len(conn_rows)} connections, {n} messages -> {outdir}")


if __name__ == "__main__":
    main()
