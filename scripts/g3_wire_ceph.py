"""Offline WireCeph1 metadata parser. No native collection or authentication claim.

References/pins live in tests/fixtures/g3-wire-ceph/README.md. All mutable buffers
are charged to the input Span's shared per-direction Budget before copying.
Unknown messages are CRC-validated and stream-skipped. Required metadata versions
outside the explicit profile fail WireCephFields, never silently disappear.
"""
from collections import deque
from dataclasses import asdict
import hashlib
import struct

from g3_wire_tcp import SourceRef, Span, WireError, require

BANNER = b"ceph v027"
HEADER = struct.Struct('<QQHHHIIIHBQHHI')
MSG_AUTH = 1 << 23
RELEVANT = {22, 23, 24, 26, 27, 28, 0x310}


def crc32c(data, crc=0):
    """Ceph reflected Castagnoli, seed 0, no pre/post complement."""
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82f63b78 if crc & 1 else 0)
    return crc


class Fields:
    """Zero-copy, checked cursor. Length/count admission precedes slicing/loops."""
    def __init__(self, data, max_count=1024, items=None):
        view = memoryview(data)
        require(view.ndim == 1 and view.itemsize == 1 and view.c_contiguous,
                'WireCephFields', 'contiguous field bytes required')
        self.data = view.cast('B')
        self.pos = 0
        self.max_count = max_count
        self.items = [4096] if items is None else items

    def take(self, n):
        require(type(n) is int and 0 <= n <= len(self.data) - self.pos,
                'WireCephFields', 'truncated field')
        p = self.pos
        self.pos += n
        return self.data[p:p+n]

    def num(self, fmt):
        return struct.unpack('<' + fmt, self.take(struct.calcsize('<' + fmt)))[0]

    def boolean(self):
        n = self.num('B')
        require(n in (0, 1), 'WireCephFields', 'invalid boolean')
        return bool(n)

    def blob(self, limit=1024*1024):
        n = self.num('I')
        require(n <= limit, 'ObserverCapacity', 'field byte limit')
        return self.take(n)

    def count(self, minimum=1):
        n = self.num('I')
        self.admit(n)
        require(n * minimum <= len(self.data) - self.pos,
                'WireCephFields', 'truncated counted field')
        return n

    def admit(self, n):
        require(n <= self.max_count and n <= self.items[0],
                'ObserverCapacity', 'field count/aggregate limit')
        self.items[0] -= n

    def cursor(self, data):
        return Fields(data, self.max_count, self.items)

    def frame(self, versions, compat=1):
        v, c, n = self.num('B'), self.num('B'), self.num('I')
        require(v in versions and c == compat, 'WireCephFields', 'unsupported nested version/compat')
        return v, self.cursor(self.take(n))

    def end(self):
        require(self.pos == len(self.data), 'WireCephFields', 'surplus field bytes')

    def skip_map(self):
        for _ in range(self.count(8)):
            self.blob()
            self.blob()


def fingerprint(salt, raw):
    h = hashlib.sha256(salt)
    h.update(raw)
    return h.hexdigest()


def path(c, salt, field):
    require(c.num('B') == 1, 'WireCephFields', 'filepath version')
    base = c.num('Q')
    b = c.blob()
    return dict(field=field, base=base, len=len(b), hash=fingerprint(salt, b))


def address(c, legacy=False):
    """entity_addr_t: legacy BE family; ADDR2 LE family. Ports always BE."""
    marker = c.num('B')
    if legacy or marker == 0:
        require(marker == 0 and c.take(3) == b'\0'*3, 'WireCephFields', 'legacy address marker')
        nonce = c.num('I')
        raw = c.take(128)
        family = int.from_bytes(raw[:2], 'big')
        ty = 1 if family else 0
    else:
        require(marker == 1, 'WireCephFields', 'address marker')
        _, f = c.frame({1})
        ty, nonce = f.num('I'), f.num('I')
        size = f.num('I')
        require(size in (0, 16, 28), 'WireCephFields', 'sockaddr size')
        raw = f.take(size)
        family = int.from_bytes(raw[:2], 'little') if size else 0
        f.end()
    require(family in (0, 2, 10), 'WireCephFields', 'address family')
    if family == 0:
        return dict(type=ty, nonce=nonce, family=None, ip=None, port=0, flowinfo=0, scope=0)
    size = 16 if family == 2 else 28
    require(len(raw) >= size, 'WireCephFields', 'short sockaddr')
    require(legacy or marker == 0 or len(raw) == size, 'WireCephFields', 'sockaddr family/size mismatch')
    ip = b'\0'*12 + bytes(raw[4:8]) if family == 2 else bytes(raw[8:24])
    return dict(type=ty, nonce=nonce, family='IPv4' if family == 2 else 'IPv6',
                ip=ip.hex(), port=int.from_bytes(raw[2:4], 'big'),
                flowinfo=0 if family == 2 else int.from_bytes(raw[4:8], 'big'),
                scope=0 if family == 2 else int.from_bytes(raw[24:28], 'little'))


def locks(c):
    """Locker::encode_file_locks: two counted ceph_filelock arrays."""
    result = []
    if len(c.data) == 0:
        return result
    for rule in ('fcntl', 'flock'):
        for _ in range(c.count(41)):
            start, length, client, owner, pid = struct.unpack('<QQQQQ', c.take(40))
            ty = c.num('B')
            result.append(dict(rule=rule, start=start, length=length, to_eof=length == 0,
                               client=client, owner=owner, pid=pid, type=ty))
    c.end()
    return result


def inode(c):
    """MClientReply.h::InodeStat / CInode::encode_inodestat, modern v1–9."""
    v, b = c.frame(set(range(1, 10)))
    ino, snap = b.num('Q'), b.num('Q')
    b.take(4+8+8)
    caps, wanted, cap_id, seq, mseq, realm, flags = struct.unpack('<IIQIIQB', b.take(33))
    b.take(28+24+4)
    size, maximum, trunc_size = b.num('Q'), b.num('Q'), b.num('Q')
    trunc_seq = b.num('I')
    b.take(16+40+8)
    b.take(8*b.count(8))  # fragtree map: u32 frag + i32 split bits
    b.blob()  # symlink content: never export
    b.take(8)
    b.blob()  # xattrs
    b.take(8)
    b.blob()  # inline data
    _, quota = b.frame({1})
    quota.take(16)
    quota.end()
    b.blob()  # pool namespace
    b.take(16)
    if v >= 2: b.take(4)
    if v >= 3: b.take(8)
    if v >= 4: b.take(8)
    if v >= 5: b.skip_map()
    if v >= 6: b.boolean()
    if v >= 7:
        b.blob()
        b.blob()
    if v >= 8:
        _, opt = b.frame({1})
        for _ in range(opt.count(14)):
            opt.take(8)
            # Optional metadata kinds carry opaque, bounded versioned frames.
            opt.take(2)
            opt.take(opt.num('I'))
        opt.end()
    if v >= 9: b.take(8)
    b.end()
    return dict(ino=ino, snap=snap, caps=caps, wanted=wanted, cap_id=cap_id,
                seq=seq, mseq=mseq, realm=realm, flags=flags, size=size,
                max_size=maximum, truncate_size=trunc_size, truncate_seq=trunc_seq)


def dirstat(c):
    _, b = c.frame({1})
    b.take(8)
    b.take(4*b.count(4))
    b.end()


def lease(c):
    v, b = c.frame({1, 2})
    b.take(10)
    if v == 2: b.blob()
    b.end()


def decode_front(ty, version, compat, front, middle_len, data_len, data,
                 salt, session_features=None, server_features=None):
    """Pure bounded metadata decoder. Does not retain any input memoryview."""
    require(all(type(x) is int for x in (ty, version, compat, middle_len, data_len))
            and 0 <= ty <= 0xffff and 0 <= version <= 0xffff and 0 <= compat <= 0xffff
            and 0 <= middle_len <= 64*1024*1024 and 0 <= data_len <= 64*1024*1024,
            'WireCephFields', 'message scalar domain')
    require(isinstance(front, (bytes, bytearray, memoryview))
            and isinstance(data, (bytes, bytearray, memoryview)), 'WireCephFields', 'metadata bytes required')
    require(isinstance(salt, bytes) and len(salt) == 32, 'WireCephFields', 'run salt length')
    require(len(front) <= 1024*1024 and len(data) <= 1024*1024,
            'ObserverCapacity', 'metadata decoder byte limit')
    c = Fields(front)
    if ty not in RELEVANT:
        return dict(kind='Skipped')
    versions = {22: set(range(8)), 23: {3, 4, 5}, 24: {4, 5, 6}, 26: {1},
                27: {1}, 28: {1}, 0x310: set(range(10, 15))}
    allowed = {0, 1} if ty in (22, 24, 0x310) else {1}
    if ty == 22 and version == 0: allowed = {0}
    if ty == 26: allowed = {0}
    if ty == 23: allowed = {0} if version == 3 else {4}
    require(version in versions[ty] and compat in allowed and compat <= version,
            'WireCephFields', 'unsupported required message version/compat')
    if ty != 0x310:
        require(middle_len == 0, 'WireCephFields', 'unexpected middle segment')
    if ty not in (23, 24):
        require(data_len == 0, 'WireCephFields', 'unexpected data segment')
    if ty == 24:
        # MClientRequest::decode_payload; common/ceph_fs.cc::decode(request_head).
        hv = c.num('H')
        require(hv in (1, 2, 3), 'WireCephFields', 'request head version')
        oldest, epoch, flags = c.num('Q'), c.num('I'), c.num('I')
        retry, fwd, releases = c.num('B'), c.num('B'), c.num('H')
        op, uid, gid, ino = c.num('I'), c.num('I'), c.num('I'), c.num('Q')
        args = c.take(56)
        if hv >= 2: retry, fwd = c.num('I'), c.num('I')
        if hv == 3:
            length = c.num('I')
            require(length == 118, 'WireCephFields', 'request head struct_len')
            c.take(8)
        p, p2 = path(c, salt, 'Path'), path(c, salt, 'Path2')
        c.admit(releases)
        require(releases * 44 <= len(c.data)-c.pos, 'WireCephFields', 'truncated release array')
        for _ in range(releases):
            c.take(40)
            c.take(c.num('I'))
        c.take(8)
        c.take(8*c.count(8))
        if version >= 5: c.blob()
        if version >= 6:
            c.blob()
            c.blob()
        c.end()
        result = dict(kind='Request', op=op, oldest_tid=oldest, epoch=epoch,
                      head_version=hv, head_flags=flags, replay=bool(flags & 1),
                      retry=retry, forward=fwd, ino=ino, caller_uid=uid, caller_gid=gid,
                      path=p, path2=p2, open=None, filelock=None)
        if op in (0x1301, 0x302):
            result['open'] = dict(flags=struct.unpack_from('<I', args, 0)[0],
                                  mode=struct.unpack_from('<I', args, 4)[0],
                                  mask=struct.unpack_from('<I', args, 24)[0],
                                  old_size=struct.unpack_from('<Q', args, 28)[0])
        if op in (0x1109, 0x110):
            owner, pid, start, length = struct.unpack_from('<QQQQ', args, 2)
            require(args[34] in (0, 1), 'WireCephFields', 'filelock wait')
            result['filelock'] = dict(rule=args[0], type=args[1], owner=owner, pid=pid,
                                     start=start, length=length, to_eof=length == 0, wait=bool(args[34]))
        return result
    if ty == 0x310:
        # MClientCaps::decode_payload, packed ceph_mds_caps_head/non_export_body.
        op, ino, realm, cap_id = c.num('I'), c.num('Q'), c.num('Q'), c.num('Q')
        seq, issue, caps, wanted, dirty, mseq = struct.unpack('<IIIIII', c.take(24))
        follows, snap_len = c.num('Q'), c.num('I')
        c.take(16)
        xattr_len = c.num('I')
        c.take(8)
        require(xattr_len == middle_len, 'WireCephFields', 'CAPS xattr/middle mismatch')
        raw = c.take(84)
        body = None if op == 3 else dict(zip(('size', 'max_size', 'truncate_size', 'truncate_seq'),
                                             struct.unpack_from('<QQQI', raw)))
        c.take(snap_len)
        locks(c.cursor(c.blob()))
        if op == 4: c.take(21)
        c.take(8)
        c.blob()
        barrier, oldest = c.num('I'), c.num('Q')
        c.take(8)
        c.blob()
        c.take(16)
        flags = c.num('I')
        if version >= 11: c.take(16)
        if version >= 12:
            c.blob()
            c.blob()
        if version >= 13: c.take(8)
        error = c.num('i') if version >= 14 else None
        c.end()
        return dict(kind='Caps', op=op, ino=ino, realm=realm, cap_id=cap_id, seq=seq,
                    issue_seq=issue, caps=caps, wanted=wanted, dirty=dirty, mseq=mseq,
                    snap_follows=follows, flags=flags, osd_barrier=barrier,
                    oldest_flush_tid=oldest, range=body, oserrno=error)
    if ty == 22:
        # MClientSession::decode_payload; MDSCapAuth/MDSCapMatch::decode.
        op, seq = c.num('I'), c.num('Q')
        c.take(8)
        max_caps, max_leases = c.num('I'), c.num('I')
        uuid, timeout, feature_bits = None, dict(kind='Absent'), None
        seen = set()
        if version >= 2:
            for _ in range(c.count(8)):
                key, value = c.blob(4096), c.blob()
                # Only the two whitelisted keys survive. Other metadata is opaque.
                if key in (b'uuid', b'timeout'):
                    name = bytes(key)
                    require(name not in seen, 'WireCephFields', 'duplicate metadata identity key')
                    seen.add(name)
                    if key == b'uuid': uuid = fingerprint(salt, value)
                    else:
                        valid = 0 < len(value) <= 10 and all(48 <= b <= 57 for b in value)
                        n = int(bytes(value)) if valid else 0
                        timeout = dict(kind='Valid', seconds=n) if valid and n <= 0xffffffff else dict(kind='Malformed', hash=fingerprint(salt, value))
        if version >= 3:
            bits = c.blob(128)
            feature_bits = int.from_bytes(bits, 'little')
        if version >= 4:
            _, metric = c.frame({1})
            metric.blob(128)
            metric.end()
        flags = c.num('I') if version >= 5 else 0
        if version >= 6:
            for _ in range(c.count(6)):
                _, auth = c.frame({1})
                _, match = auth.frame({1})
                match.take(8)
                match.take(4*match.count(4))
                match.blob()
                match.blob()
                match.boolean()
                match.end()
                auth.boolean()
                auth.boolean()
                auth.end()
        oldest = c.num('Q') if version >= 7 else None
        c.end()
        return dict(kind='Session', op=op, seq=seq, uuid=uuid, timeout=timeout,
                    features=feature_bits, flags=flags, oldest_tid=oldest,
                    max_caps=max_caps, max_leases=max_leases)
    if ty == 23:
        # MClientReconnect::decode_payload; cap_reconnect_t::decode.
        require(not front and len(data) == data_len, 'WireCephFields', 'reconnect segment placement')
        c = c.cursor(data)
        caps, realms, seen = [], [], set()
        for _ in range(c.count(54)):
            ino = c.num('Q')
            require(ino not in seen, 'WireCephFields', 'duplicate reconnect inode')
            seen.add(ino)
            _, b = c.frame({2})
            p = b.blob()
            cap_id, wanted, issued, realm, base, flock_len = struct.unpack('<QIIQQI', b.take(36))
            lockset = locks(b.cursor(b.take(flock_len)))
            follows = b.num('Q')
            b.end()
            caps.append(dict(ino=ino, path=dict(base=base, len=len(p), hash=fingerprint(salt, p)),
                             cap_id=cap_id, wanted=wanted, issued=issued, realm=realm,
                             snap_follows=follows, locks=lockset))
        if version == 3:
            require((len(c.data)-c.pos) % 24 == 0 and (len(c.data)-c.pos)//24 <= 1024,
                    'WireCephFields', 'v3 realm array')
            c.admit((len(c.data)-c.pos)//24)
            while c.pos < len(c.data):
                realms.append(dict(zip(('ino', 'seq', 'parent'), struct.unpack('<QQQ', c.take(24)))))
        else:
            for _ in range(c.count(30)):
                _, b = c.frame({1})
                realms.append(dict(zip(('ino', 'seq', 'parent'), struct.unpack('<QQQ', b.take(24)))))
                b.end()
        more = c.boolean() if version == 5 else False
        c.end()
        return dict(kind='Reconnect', caps=caps, realms=realms, more=more, final=not more)
    if ty == 27:
        # MClientReclaim::decode_payload. FINISH alone expects no reply.
        uuid, flags = fingerprint(salt, c.blob()), c.num('I')
        require(not flags & (1 << 31) or flags == 1 << 31, 'WireCephFields', 'FINISH must be alone')
        c.end()
        return dict(kind='Reclaim', uuid=uuid, flags=flags)
    if ty == 28:
        # MClientReclaimReply::decode_payload; entity_addr{vec,_t}::decode.
        result, epoch = c.num('i'), c.num('I')
        require(c.pos < len(c.data), 'WireCephFields', 'missing address vector')
        if c.data[c.pos] == 2:
            c.take(1)
            n = c.count(1)
            require(n <= 64, 'ObserverCapacity', 'address union limit')
            addrs = [address(c) for _ in range(n)]
        else:
            addrs = [address(c)]
        c.end()
        require(len({tuple(x.values()) for x in addrs}) == len(addrs), 'WireCephFields', 'duplicate addresses')
        return dict(kind='ReclaimReply', result=result, epoch=epoch, addresses=addrs)
    # REPLY. Session feature evidence is required before selecting trace/CREATE form.
    op, result, epoch = c.num('I'), c.num('i'), c.num('I')
    safe, is_dir, is_target = c.boolean(), c.boolean(), c.boolean()
    trace, extra, snap = c.blob(), c.blob(), c.blob()
    c.end()
    require(session_features is not None, 'WireCephFields', 'reply lacks observed SESSION negotiation')
    require(server_features is not None, 'WireCephFields', 'reply lacks observed server OPEN features')
    require(not trace or session_features & (1 << 9), 'WireCephFields', 'legacy trace unsupported')
    t = c.cursor(trace)
    parent, name, target = None, None, None
    if is_dir:
        parent = inode(t)
        dirstat(t)
        raw = t.blob()
        name = dict(len=len(raw), hash=fingerprint(salt, raw))
        lease(t)
    if is_target: target = inode(t)
    t.end()
    created = None
    e = c.cursor(extra)
    if op == 0x1301 and extra:
        if server_features & (1 << 13):
            _, b = e.frame({1})
            created = b.num('Q')
            b.take(16*b.count(16))
            b.end()
        else: created = e.num('Q')
        e.end()
    elif op == 0x110 and extra:
        e.take(41)
        e.end()
    elif op in (0x305, 0x402) and extra:
        require(session_features & (1 << 9), 'WireCephFields', 'legacy directory extra unsupported')
        dirstat(e)
        n = e.count(1)
        e.take(2)
        for _ in range(n):
            e.blob()
            lease(e)
            inode(e)
        e.end()
    elif op == 0x106 and extra:
        _, b = e.frame({1})
        b.blob()
        b.end()
        e.end()
    else: e.end()
    return dict(kind='Reply', op=op, result=result, epoch=epoch, safe=safe,
                target=target, dentry_parent=parent, dentry_name=name, created_ino=created,
                trace=bool(trace), snap_trace=bool(snap))


def covers_flush_ack(caps, selected_ino, selected_tid):
    """Caller must separately join exact native client/session and rank."""
    return caps.get('kind') == 'Caps' and caps['op'] == 8 and caps['ino'] == selected_ino and caps['tid'] >= selected_tid


class _Queue:
    def __init__(self, budget):
        self.budget = budget
        self.parts = deque()
        self.size = 0
        self.offset = 0
        self.refs = []
        self.scratch = 0

    def append(self, span):
        require(span.byte_offset == self.offset + self.size, 'WireCeph1', 'noncontiguous span')
        require(span.budget is self.budget and bool(span.data), 'WireCeph1', 'span lease/direction mismatch')
        require(type(span.source_refs) is tuple and 0 < len(span.source_refs) <= 4096,
                'WireCeph1', 'span source ranges required')
        at = span.byte_offset
        for ref in span.source_refs:
            require(isinstance(ref, SourceRef) and ref.stream_start == at
                    and all(type(x) is int for x in (ref.stream_start, ref.stream_end, ref.packet_start, ref.packet_end))
                    and ref.stream_end > at and ref.packet_start >= 0
                    and ref.packet_end <= 65549
                    and ref.packet_end-ref.packet_start == ref.stream_end-ref.stream_start
                    and ref.capture == span.identity['capture'], 'WireCeph1', 'invalid span source coverage')
            at = ref.stream_end
        require(at == span.byte_offset + len(span.data), 'WireCeph1', 'incomplete span source coverage')
        self.budget.take(256)
        self.parts.append([span, 0])
        self.size += len(span.data)

    def read(self, n, collect=True):
        require(n <= self.size, 'WireCeph1', 'parser underflow')
        # Use one admitted fixed-size copy. No views retain a released Span's
        # larger backing allocation. Skipped authorizers allocate no byte copy.
        if collect: self.budget.take(n)
        result = bytearray(n) if collect else None
        left = n
        while left:
            span, at = self.parts[0]
            k = min(left, len(span.data)-at)
            if collect:
                memoryview(result)[n-left:n-left+k] = memoryview(span.data)[at:at+k]
            lo, hi = self.offset, self.offset+k
            for ref in span.source_refs:
                l, r = max(lo, ref.stream_start), min(hi, ref.stream_end)
                if l < r:
                    last = self.refs[-1] if self.refs else None
                    if (last is not None and last['capture'] == ref.capture
                            and last['packet'] == ref.packet and last['source_offset'] == ref.source_offset
                            and last['observed_ns'] == ref.observed_ns and last['stream_end'] == l
                            and last['packet_end'] == ref.packet_start + l-ref.stream_start):
                        last['stream_end'] = r
                        last['packet_end'] = ref.packet_start + r-ref.stream_start
                    else:
                        require(len(self.refs) < 4096, 'ObserverCapacity', 'frame source-range count')
                        self.budget.take(512)
                        self.refs.append(dict(capture=ref.capture, packet=ref.packet, source_offset=ref.source_offset,
                                              packet_start=ref.packet_start + l-ref.stream_start,
                                              packet_end=ref.packet_start + r-ref.stream_start,
                                              observed_ns=ref.observed_ns, stream_start=l, stream_end=r))
            self.offset += k
            self.size -= k
            left -= k
            self.parts[0][1] += k
            if self.parts[0][1] == len(span.data):
                self.parts.popleft()
                span.release()
                self.budget.drop(256)
        return result

    def fixed(self, n):
        self.budget.drop(self.scratch)
        self.scratch = 0
        b = self.read(n)
        self.scratch = n
        return b

    def receipt(self):
        refs = self.refs
        self.refs = []
        self.budget.drop(512*len(refs))
        return refs


class CephReader:
    """feed(Span) -> sanitized records; finish(barrier) -> continuity receipt.

    Feed every span exactly once in arrival order, across both directions. Spans
    share TcpReader's budget. Outputs must be drained by the caller; this reader
    bounds total records and does not retain emitted transcripts or packet bodies.
    Session epochs are passive address/nonce joins, NOT proof of authentication.
    """

    def __init__(self, salt, *, max_streams=128, max_records=100000,
                 max_front=1024*1024, max_reconnect=1024*1024,
                 max_payload=64*1024*1024, max_rounds=16, max_reconnect_chains=128):
        require(isinstance(salt, bytes) and len(salt) == 32, 'WireCephFields', 'run salt')
        require(all(type(x) is int for x in (max_streams, max_records, max_front,
                                           max_reconnect, max_payload, max_rounds, max_reconnect_chains))
                and 1 <= max_streams <= 128 and 1 <= max_records <= 1000000
                and 1 <= max_front <= 1024*1024 and 1 <= max_reconnect <= 1024*1024
                and 1 <= max_payload <= 64*1024*1024 and 1 <= max_rounds <= 16
                and 1 <= max_reconnect_chains <= 128,
                'ObserverCapacity', 'Ceph limits')
        self.salt = salt
        self.limits = (max_streams, max_records, max_front, max_reconnect, max_payload, max_rounds)
        self.streams = {}
        self.sessions = {}
        self.clients = {}
        self.max_reconnect_chains = max_reconnect_chains
        self.reconnect_chains = {}
        self.records = 0
        self.failed = False
        self.finished = False

    def _emit(self, out, s, direction, kind, **body):
        require(self.records < self.limits[1], 'ObserverCapacity', 'record count')
        self.records += 1
        q = s['q'][direction]
        refs = q.receipt()
        start = refs[0]['stream_start'] if refs else q.offset
        out.append(dict(kind=kind, record=self.records, stream=s['id'], direction=direction,
                        frame_offset=start, frame_end=q.offset, messenger_epoch=s.get('epoch'),
                        source_refs=refs, **body))

    def _tcp_event(self, s, span):
        require(self.records < self.limits[1], 'ObserverCapacity', 'record count')
        self.records += 1
        record = dict(kind='TransportDuplicate' if span.event == 'retransmit' else 'TransportEnd',
                      record=self.records,
                      stream=s['id'], direction=span.direction, event=span.event,
                      messenger_epoch=s.get('epoch'), tcp_identity=dict(s['identity']),
                      source_refs=[asdict(r) for r in span.source_refs])
        span.release()
        return [record]

    def feed(self, span):
        require(not self.failed and not self.finished, 'WireCeph1', 'reader not active')
        try:
            require(isinstance(span, Span) and not span._released, 'WireCeph1', 'owned Span required')
            require(type(span.direction) is int and span.direction in (0, 1)
                    and type(span.stream) is int and 0 < span.stream <= self.limits[0]
                    and type(span.byte_offset) is int and span.byte_offset >= 0
                    and type(span.data) is bytes and len(span.data) <= 65535
                    and type(span.identity) is dict and 'capture' in span.identity
                    and type(span.source_refs) is tuple and len(span.source_refs) <= 4097
                    and span.event in ('syn', 'synack', 'data', 'fin', 'rst', 'retransmit'),
                    'WireCeph1', 'span identity/event')
            require((span.event != 'syn' or span.direction == 0)
                    and (span.event != 'synack' or span.direction == 1)
                    and (span.event == 'data' or not span.data), 'WireCeph1', 'span event direction/bytes')
            return self._feed(span)
        except (WireError, struct.error):
            self.failed = True
            raise

    def _feed(self, span):
        out = []
        s = self.streams.get(span.stream)
        if span.event == 'syn':
            require(s is None and len(self.streams) < self.limits[0]
                    and sum(not all(x['ended']) for x in self.streams.values()) < 8,
                    'ObserverCapacity', 'Ceph stream table')
            s = dict(id=span.stream, identity=span.identity, q=[_Queue(span.budget), None],
                     phase=['banner', 'banner'], start=[0, 0], addresses=[None, None],
                     connect=None, rounds=0, accepted=None, epoch=None,
                      session=None, client=None, server_features=None, server_features_record=None,
                      pending_features=None,
                      payload=[None, None], ended=[False, False])
            self.streams[span.stream] = s
            span.release()
            return []
        require(s is not None, 'WireCeph1', 'missing TCP establishment')
        d = span.direction
        if span.event == 'synack':
            require(s['q'][1] is None, 'WireCeph1', 'duplicate SYN-ACK event')
            s['q'][1] = _Queue(span.budget)
            s['identity'] = span.identity
            span.release()
            return []
        require(s['q'][d] is not None, 'WireCeph1', 'missing direction')
        require(span.budget is s['q'][d].budget, 'WireCeph1', 'span budget changed')
        require(span.identity == s['identity'], 'WireCeph1', 'stream identity changed')
        if span.event == 'retransmit':
            return self._tcp_event(s, span)
        if span.event in ('fin', 'rst'):
            require(not s['q'][d].size and s['phase'][d] in ('tag', 'closed'),
                    'WireCeph1', 'transport ended inside required frame/negotiation')
            s['ended'][d] = True
            return self._tcp_event(s, span)
        require(not s['ended'][d], 'WireCeph1', 'data after transport end')
        s['q'][d].append(span)
        progress = True
        while progress:
            progress = False
            for direction in (0, 1):
                if s['q'][direction] is not None and self._step(s, direction, out): progress = True
        return out

    def _step(self, s, d, out):
        q, phase = s['q'][d], s['phase'][d]
        if phase == 'banner':
            size = 9 + (136 if d == 0 else 272)
            if q.size < size: return False
            raw = q.fixed(size)
            require(raw[:9] == BANNER, 'WireCeph1', 'unsupported messenger banner')
            c = Fields(memoryview(raw)[9:])
            s['addresses'][d] = address(c, True)
            observed = address(c, True) if d else None
            c.end()
            self._emit(out, s, d, 'Banner', address=s['addresses'][d], observed_peer=observed,
                       tcp_identity=dict(s['identity']), authentication_verified=False)
            s['phase'][d] = 'connect' if d == 0 else 'reply'
            return True
        if phase == 'connect':
            if s['connect'] is not None or not all(s['addresses']) or q.size < 33: return False
            raw = q.fixed(33)
            features, host, global_seq, connect_seq, protocol, auth_protocol, length, flags = struct.unpack('<QIIIIIIB', raw)
            require(length <= self.limits[4] and flags & ~1 == 0,
                    'WireCeph1', 'connect authorizer/flags')
            require(flags == 0, 'WireCeph1', 'lossy messenger outside profile')
            require(host == 8 and protocol == 32, 'WireCeph1', 'client-to-MDS protocol32 required')
            s['rounds'] += 1
            require(s['rounds'] <= self.limits[5], 'ObserverCapacity', 'negotiation rounds')
            retry = s.get('retry')
            if retry:
                tag, g, c, previous = retry
                require((tag != 2 or connect_seq == 0) and (tag != 4 or connect_seq == c),
                        'WireCeph1', 'invalid connect retry')
                if tag == 5:
                    # ProtocolV1::handle_connect_reply_2 calls AsyncMessenger::
                    # get_global_seq: increment max(current counter, peer hint).
                    # Other connections can advance that counter further. This
                    # observer supports increasing u32 generations, not rollover.
                    require(max(previous['global_seq'], g) < 0xffffffff,
                            'WireCeph1', 'RETRY_GLOBAL u32 rollover unsupported')
                    require(global_seq > max(previous['global_seq'], g),
                            'WireCeph1', 'RETRY_GLOBAL generation did not advance')
                require(features == previous['features'] and host == previous['host_type']
                        and protocol == previous['protocol'] and auth_protocol == previous['auth_protocol']
                        and (tag in (2, 4) or connect_seq == previous['connect_seq'])
                        and (tag == 5 or global_seq == previous['global_seq']),
                        'WireCeph1', 'connect retry changed unrelated negotiation fields')
            s['connect'] = dict(features=features, host_type=host, global_seq=global_seq,
                                connect_seq=connect_seq, protocol=protocol, auth_protocol=auth_protocol,
                                authorizer_len=length, flags=flags)
            s['skip0'] = length
            s['phase'][0] = 'connect-auth'
            return True
        if phase in ('connect-auth', 'reply-auth'):
            key = 'skip0' if d == 0 else 'skip1'
            n = min(q.size, s[key])
            if n:
                q.read(n, False)
                s[key] -= n
            if s[key]: return bool(n)
            if d == 0:
                self._emit(out, s, d, 'Connect', **s['connect'], authentication_verified=False)
                s['phase'][d] = 'waiting'
            else: self._accept_reply(s, out)
            return True
        if phase == 'reply':
            if s['connect'] is None or s['phase'][0] != 'waiting' or q.size < 26: return False
            raw = q.fixed(26)
            tag, features, global_seq, connect_seq, protocol, length, flags = struct.unpack('<BQIIIIB', raw)
            require(length <= self.limits[4] and flags & ~1 == 0, 'WireCeph1', 'reply authorizer/flags')
            require(flags == 0, 'WireCeph1', 'lossy messenger outside profile')
            s['reply'] = dict(tag=tag, features=features, global_seq=global_seq,
                              connect_seq=connect_seq, protocol=protocol, authorizer_len=length, flags=flags)
            s['skip1'] = length
            s['phase'][1] = 'reply-auth'
            return True
        if phase == 'seq':
            if q.size < 8: return False
            n = struct.unpack('<Q', q.fixed(8))[0]
            require(n <= s['session']['high'][1-d], 'WireCeph1', 'SEQ acknowledges unobserved message')
            self._emit(out, s, d, 'SequenceExchange', acknowledged=n)
            s['session']['acks'][d] = max(s['session']['acks'][d], n)
            s['phase'][d] = 'seq-wait'
            if s['phase'][1-d] == 'seq-wait': s['phase'] = ['tag', 'tag']
            return True
        if phase in ('waiting', 'seq-wait', 'closed'): return False
        if phase == 'header':
            if q.size < 53: return False
            raw = q.fixed(53)
            values = HEADER.unpack(raw)
            require(crc32c(raw[:49]) == values[-1], 'WireCephCRC', 'header CRC32C')
            seq, tid, ty, priority, version, front, middle, data, off, src_type, src_num, compat, reserved, _ = values
            require(front <= self.limits[2] and middle <= self.limits[4] and data <= self.limits[4],
                    'ObserverCapacity', 'message segment length')
            require(ty != 23 or data <= self.limits[3], 'ObserverCapacity', 'reconnect data length')
            # MClientRequest::encode_payload selects the legacy 88-byte head
            # without FS_BTIME, even when its message version is modern.
            require(ty != 24 or s['accepted'] & (1 << 59),
                    'WireCephFields', 'REQUEST legacy head without FS_BTIME unsupported')
            header = dict(seq=seq, tid=tid, type=ty, priority=priority, version=version,
                          front_len=front, middle_len=middle, data_len=data, data_off=off,
                          source_type=src_type, source_num=src_num, compat=compat, reserved=reserved)
            keep_front = front if ty in RELEVANT else 0
            keep_data = data if ty == 23 else 0
            # Fixed-size metadata storage avoids bytearray growth/reallocation.
            # Charge a second copy allowance before either allocation.
            charge = 2 * (keep_front + keep_data)
            q.budget.take(charge)
            s['payload'][d] = dict(header=header, stage=0, left=[front, middle, data],
                                    crcs=[0, 0, 0], buffers=[bytearray(keep_front), None, bytearray(keep_data)],
                                    charged=charge)
            s['phase'][d] = 'payload'
            return True
        if phase == 'payload':
            p = s['payload'][d]
            while p['stage'] < 3 and p['left'][p['stage']] == 0: p['stage'] += 1
            if p['stage'] == 3:
                s['phase'][d] = 'footer'
                return True
            if not q.size: return False
            i = p['stage']
            n = min(q.size, p['left'][i], 65536)
            # Read is charged before allocation; copied required metadata retains
            # another lease. Unneeded file/auth/xattr data is not kept.
            raw = q.read(n)
            p['crcs'][i] = crc32c(raw, p['crcs'][i])
            if p['header']['type'] in RELEVANT and (i == 0 or (i == 2 and p['header']['type'] == 23)):
                at = len(p['buffers'][i]) - p['left'][i]
                memoryview(p['buffers'][i])[at:at+n] = raw
            del raw
            q.budget.drop(n)
            p['left'][i] -= n
            return True
        if phase == 'footer':
            size = 21 if s['accepted'] & MSG_AUTH else 13
            if q.size < size: return False
            raw = q.fixed(size)
            front_crc, middle_crc, data_crc = struct.unpack_from('<III', raw)
            flags = raw[-1]
            require(flags & 1 and not flags & ~7, 'WireCephCRC', 'aborted/unknown footer')
            require(not flags & 4 or size == 21, 'WireCeph1', 'SIGNED without MSG_AUTH')
            p = s['payload'][d]
            require(p['crcs'][:2] == [front_crc, middle_crc], 'WireCephCRC', 'front/middle CRC32C')
            require(flags & 2 or p['crcs'][2] == data_crc, 'WireCephCRC', 'data CRC32C')
            h = p['header']
            client = self._client(s, d, h)
            format_features = client.get('features') if client is not None else None
            fields = decode_front(h['type'], h['version'], h['compat'], p['buffers'][0],
                                  h['middle_len'], h['data_len'], p['buffers'][2], self.salt,
                                  format_features, s['server_features'])
            high = s['session']['high'][d]
            replay = h['seq'] <= high
            if not replay:
                s['session']['high'][d] = h['seq']
                if fields['kind'] == 'Session':
                    self._session_fields(s, d, fields, client)
                if fields['kind'] == 'Reconnect':
                    require(d == 0 and client is not None, 'WireCephFields', 'RECONNECT direction/client')
                    # Client::send_reconnect(MetaSession*) sends all chunks to one
                    # server session. Retain bounded peer rows until finish; never
                    # let another endpoint or an obsolete epoch reset its budget.
                    key = (client['identity'], self._server_scope(s))
                    chain = self.reconnect_chains.get(key)
                    if chain is None:
                        require(len(self.reconnect_chains) < self.max_reconnect_chains,
                                'ObserverCapacity', 'reconnect chain table')
                        chain = dict(series=0, chunks=0, caps=0, realms=0, more=False, epoch=s['epoch'])
                        self.reconnect_chains[key] = chain
                    require(s['epoch'] >= chain['epoch'], 'WireCephFields', 'obsolete reconnect epoch')
                    changed = chain.get('epoch') != s['epoch']
                    fields['previous_incomplete'] = bool(chain['more'] and changed)
                    if not chain['more'] or changed:
                        chain.update(series=chain['series']+1, chunks=0, caps=0, realms=0, epoch=s['epoch'])
                    require(chain['chunks'] < 64 and chain['caps']+len(fields['caps']) <= 4096
                            and chain['realms']+len(fields['realms']) <= 4096,
                            'ObserverCapacity', 'reconnect series bound')
                    chain.update(chunks=chain['chunks']+1, caps=chain['caps']+len(fields['caps']),
                                 realms=chain['realms']+len(fields['realms']), more=fields['more'])
                    fields['series'] = chain['series']
                    fields['chunk'] = chain['chunks']
            if fields['kind'] == 'Caps': fields['tid'] = h['tid']
            if fields['kind'] in ('Request', 'Reply'):
                require(d == (0 if fields['kind'] == 'Request' else 1),
                        'WireCephFields', 'request/reply direction')
            if not replay and fields['kind'] == 'Reclaim':
                require(d == 0, 'WireCephFields', 'RECLAIM direction')
                if fields['flags'] != 1 << 31:
                    require(s['session'].get('reclaim_pending') is None, 'WireCephFields', 'ambiguous outstanding RECLAIM')
                    s['session']['reclaim_pending'] = (s['id'], s['epoch'], self.records+1)
                    fields['exchange'] = s['session']['reclaim_pending']
                else:
                    fields['exchange'] = None
            if not replay and fields['kind'] == 'ReclaimReply':
                require(d == 1 and s['session'].get('reclaim_pending') is not None,
                        'WireCephFields', 'RECLAIM_REPLY without observed exchange')
                fields['exchange'] = s['session'].pop('reclaim_pending')
            self._emit(out, s, d, 'Message', header=h, fields=fields,
                       client_entity=None if client is None else client['entity'],
                       client_features_record=None if client is None else client['feature_record'],
                       server_features_record=s['server_features_record'],
                       transport_replay=replay, sequence_gap=not replay and h['seq'] > high+1,
                       data_crc='omitted' if flags & 2 else 'checked',
                       signed=bool(flags & 4), signature_verified=False,
                       identity=(s['id'], d, s['start'][d], s['epoch'], h['seq'], h['tid'], h['type']))
            charge = p['charged']
            s['payload'][d] = None
            del p
            q.budget.drop(charge)
            s['phase'][d] = 'tag'
            s['start'][d] = q.offset
            return True
        if phase == 'control':
            tag = s['control'][d]
            size = 8
            if q.size < size: return False
            raw = q.fixed(size)
            if tag == 8:
                value = struct.unpack('<Q', raw)[0]
                require(value <= s['session']['high'][1-d], 'WireCeph1', 'ACK acknowledges unobserved message')
                s['session']['acks'][d] = max(s['session']['acks'][d], value)
            else:
                sec, nsec = struct.unpack('<II', raw)
                require(nsec < 1000000000, 'WireCeph1', 'keepalive timestamp')
                value = dict(sec=sec, nsec=nsec)
            self._emit(out, s, d, 'Control', tag=tag, value=value)
            s['phase'][d] = 'tag'
            s['start'][d] = q.offset
            return True
        require(phase == 'tag', 'WireCeph1', 'invalid state')
        if not q.size: return False
        require(self.records < self.limits[1], 'ObserverCapacity', 'record count')
        s['start'][d] = q.offset
        tag = q.fixed(1)[0]
        if tag == 7: s['phase'][d] = 'header'
        elif tag in (8, 14, 15):
            s.setdefault('control', {})[d] = tag
            s['phase'][d] = 'control'
        elif tag in (6, 9):
            self._emit(out, s, d, 'Control', tag=tag, value=None)
            s['phase'][d] = 'closed' if tag == 6 else 'tag'
        else: raise WireError('WireCeph1', 'tag is invalid in ready state')
        return True

    def _accept_reply(self, s, out):
        r, connect = s['reply'], s['connect']
        tag = r['tag']
        if tag in (2, 4, 5, 16):
            require(tag != 4 or r['connect_seq'] > connect['connect_seq'], 'WireCeph1', 'RETRY_SESSION sequence')
            require(tag != 16 or (connect['auth_protocol'] == 2 and r['authorizer_len'] > 0),
                    'WireCeph1', 'invalid authorizer challenge')
            s['retry'] = (tag, r['global_seq'], r['connect_seq'], connect)
            if tag == 2: s['reset'] = True
            self._emit(out, s, 1, 'ConnectReply', **r, authentication_verified=False)
            s['connect'] = None
            s['phase'] = ['connect', 'reply']
            return
        require(tag in (1, 13) and r['connect_seq'] == connect['connect_seq']+1
                and r['protocol'] == connect['protocol'], 'WireCeph1', 'negotiation failed/invalid ready')
        s['accepted'] = r['features'] & connect['features']
        require(tag != 13 or s['accepted'] & (1 << 6), 'WireCeph1', 'SEQ without RECONNECT_SEQ')
        domain = tuple(s['identity'][k] for k in ('capture', 'boot', 'netns', 'interface'))
        key = (domain, tuple(s['addresses'][0].values()), self._server_scope(s))
        old = self.sessions.get(key)
        reset = tag == 1 or s.get('reset', False)
        require(reset or old is not None or connect['connect_seq'] == 0,
                'WireCeph1', 'unobserved messenger session predecessor')
        if old is None or reset:
            require(len(self.sessions) < 128, 'ObserverCapacity', 'messenger session table')
            session = dict(epoch=self.records+1, high=[0, 0], acks=[0, 0],
                           server_features=None, server_features_record=None)
            self.sessions[key] = session
        else:
            require(connect['connect_seq'] >= old['connect_seq']
                    and connect['global_seq'] >= old['client_global_seq']
                    and r['global_seq'] >= old['server_global_seq'],
                    'WireCeph1', 'stale messenger reconnect generation')
            session = old
        previous_stream = session.get('stream')
        session.update(connect_seq=r['connect_seq'], client_global_seq=connect['global_seq'],
                       server_global_seq=r['global_seq'], stream=s['id'])
        s['session'], s['epoch'] = session, session['epoch']
        s['client'] = session.get('client')
        s['server_features'] = session['server_features']
        s['server_features_record'] = session['server_features_record']
        self._emit(out, s, 1, 'ConnectReply', **r, accepted_features=s['accepted'],
                   predecessor_stream=previous_stream, reset_epoch=reset,
                   authentication_verified=False)
        s['phase'] = ['seq', 'seq'] if tag == 13 else ['tag', 'tag']
        s['start'] = [q.offset for q in s['q']]

    @staticmethod
    def _server_scope(s):
        """Observed endpoint and advertisement, not a cross-address logical rank."""
        return (s['identity']['server'], tuple(s['addresses'][1].values()),
                s['connect']['host_type'], s['connect']['protocol'])

    def _client(self, s, direction, header):
        """Only a client-origin header supplies client entity, never an MDS reply."""
        if direction == 1:
            require(header['source_type'] == 2, 'WireCephFields', 'non-MDS message source')
            return s['client']
        require(header['source_type'] == 8, 'WireCephFields', 'non-client message source')
        entity = header['source_num']
        if s['client'] is not None:
            require(s['client']['entity'] == entity, 'WireCephFields', 'client entity changed on stream')
            return s['client']
        domain = tuple(s['identity'][k] for k in ('capture', 'boot', 'netns', 'interface'))
        key = (domain, tuple(s['addresses'][0].values()), entity)
        client = self.clients.get(key)
        if client is None:
            require(len(self.clients) < 128, 'ObserverCapacity', 'CephFS client table')
            client = dict(identity=key, entity=entity, features=None, feature_record=None, servers={})
            self.clients[key] = client
        s['client'] = client
        old_server = client['servers'].get(self._server_scope(s))
        if s['server_features'] is None and old_server is not None:
            s['server_features'], s['server_features_record'] = old_server
            s['session']['server_features'] = s['server_features']
            s['session']['server_features_record'] = s['server_features_record']
        s['session']['client'] = client
        return client

    def _session_fields(self, s, direction, fields, client):
        """Server::handle_client_session / CInode::encode_inodestat / Client::make_request.

        The client's REQUEST_OPEN bitmap selects server trace encoding. The MDS's
        OPEN bitmap selects CREATE extra decoding at the client. They are separate.
        Retain the observed client bitmap across messenger/MDS resets, joined by
        capture domain, advertised client address/nonce and client-origin entity.
        This passive continuity is not an authenticated SessionKey binding.
        """
        if fields['op'] == 0:
            require(direction == 0 and client is not None, 'WireCephFields', 'REQUEST_OPEN direction')
            bits = fields['features'] if fields['features'] is not None else 0
            require(client['features'] is None or client['features'] == bits,
                    'WireCephFields', 'client features changed within identity')
            require(s['pending_features'] is None or s['pending_features'][0] == bits,
                    'WireCephFields', 'ambiguous outstanding REQUEST_OPEN')
            if s['pending_features'] is None: s['pending_features'] = (bits, self.records+1)
        elif fields['op'] == 1:
            require(direction == 1, 'WireCephFields', 'OPEN direction')
            if client is not None and s['pending_features'] is not None:
                client['features'], client['feature_record'] = s['pending_features']
                s['pending_features'] = None
            s['server_features'] = fields['features'] if fields['features'] is not None else 0
            s['server_features_record'] = self.records+1
            s['session']['server_features'] = s['server_features']
            s['session']['server_features_record'] = s['server_features_record']
            if client is not None:
                key = self._server_scope(s)
                require(key in client['servers'] or len(client['servers']) < 128,
                        'ObserverCapacity', 'client server-feature table')
                client['servers'][key] = (s['server_features'], s['server_features_record'])

    def finish(self, barrier):
        require(not self.failed and not self.finished, 'WireCeph1', 'reader not active')
        try:
            return self._finish(barrier)
        except WireError:
            self.failed = True
            raise

    def _finish(self, barrier):
        require(type(barrier) is dict and barrier.get('drained') is True
                and type(barrier.get('parser_errors')) is int and barrier['parser_errors'] == 0,
                'WireCeph1', 'unproved parser drain')
        for s in self.streams.values():
            require(s['session'] is not None and all(p in ('tag', 'closed') for p in s['phase'])
                    and all(q is not None and not q.size for q in s['q']),
                    'WireCeph1', 'incomplete negotiation/message/control')
        for s in self.streams.values():
            for q in s['q']:
                q.budget.drop(q.scratch)
                q.scratch = 0
        self.finished = True
        return dict(provider='WireCeph1', origin='offline-decoder', records=self.records,
                    streams=len(self.streams), drained=True, authentication_verified=False)
