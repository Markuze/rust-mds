"""Independent synthetic packed layouts; no raw captures, credentials or daemon."""
import ast
import hashlib
import json
from pathlib import Path
import struct
import unittest
from unittest.mock import patch

# This owned module installs real-signal/process/socket guards before provider import.
from test_g3_wire_tcp import Harness, forbidden
from g3_wire_tcp import WireError
from g3_wire_ceph import CephReader, decode_front, crc32c, covers_flush_ack, HEADER

SALT = bytes(range(32))
FEATURES = (1 << 9) | (1 << 13)


def u(fmt, *values): return struct.pack('<'+fmt, *values)
def blob(b=b''): return u('I', len(b)) + b
def framed(b, version=1, compat=1): return u('BBI', version, compat, len(b)) + b


def legacy(nonce=7, ipv6=False):
    ss = bytearray(128)
    ss[:4] = struct.pack('!HH', 10 if ipv6 else 2, 7402)
    if ipv6:
        ss[8:24] = bytes.fromhex('20010db8000000000000000000000001')
        ss[24:28] = u('I', 9)
    else: ss[4:8] = b'\x7f\0\0\x01'
    return bytes(4) + u('I', nonce) + ss


def addr2(nonce=8, ipv6=False):
    raw = legacy(nonce, ipv6)[8:]
    size = 28 if ipv6 else 16
    raw = u('H', 10 if ipv6 else 2) + raw[2:size]
    return b'\x01' + framed(u('III', 2, nonce, size) + raw)


def session(v=7, op=0, feature=FEATURES, metadata=None):
    b = u('IQIIII', op, 555, 0, 0, 7, 8)
    if v >= 2:
        metadata = metadata if metadata is not None else {b'uuid': b'not-a-real-uuid', b'timeout': b'300', b'auth': b'do-not-export'}
        b += u('I', len(metadata)) + b''.join(blob(k)+blob(value) for k, value in metadata.items())
    if v >= 3: b += blob(u('Q', feature))
    if v >= 4: b += framed(blob(u('Q', 0xffff)))
    if v >= 5: b += u('I', 0)
    if v >= 6:
        match = framed(u('qI', -1, 0)+blob(b'/private')+blob(b'fs')+b'\0')
        b += u('I', 1)+framed(match+b'\x01\x01')
    if v >= 7: b += u('Q', 123)
    return bytes(b)


def request(v=6, hv=3, op=0x1301, p=b'\xff/new', p2=b'', retry=300, flags=2, args=None):
    a = args if args is not None else u('IIIIiiIQ', 0o1100, 0o644, 0, 0, 0, -1, 0xab, 77) + bytes(20)
    b = u('HQIIBBHIIIQ', hv, 4, 2, flags, retry & 255, 3, 1, op, 1000, 1001, 99) + a
    if hv >= 2: b += u('II', retry, 3)
    if hv >= 3: b += u('III', 118, 123, 124)
    b += b'\x01'+u('Q', 10)+blob(p) + b'\x01'+u('Q', 20)+blob(p2)
    b += bytes(40)+u('I', 3)+b'old'  # one 44-byte release plus raw name
    b += bytes(8)+u('IQQ', 2, 1, 2)
    if v >= 5: b += blob(b'opaque-alt')
    if v >= 6: b += blob(b'opaque-auth')+blob(b'opaque-file')
    return b


def caps(v=14, op=8, ino=1234, cap=10, seq=30, dirty=9, xattr=0):
    b = u('IQQQ', op, ino, 1, cap)+u('IIIIII', seq, 31, 7, 8, dirty, 34)
    b += u('QI', 0, 0)+bytes(16)+u('IQ', xattr, 2)
    b += (u('QQQI', 123, 456, 111, 42)+bytes(56)) if op != 3 else bytes(84)
    b += blob()
    if op == 4: b += bytes(21)
    b += u('Q', 9)+blob(b'inline-private')+u('IQ', 88, 12)+bytes(8)+blob(b'ns')+bytes(16)+u('I', 1)
    if v >= 11: b += bytes(16)
    if v >= 12: b += blob()+blob()
    if v >= 13: b += u('Q', 101)
    if v >= 14: b += u('i', -5)
    return b


def inodestat(ino=555, v=9, caps_value=0, maximum=0, splits=(), optional=()):
    b = u('QQIQQ', ino, (1 << 64)-2, 0, 2, 0)
    b += u('IIQIIQB', caps_value, 0, 88 if caps_value else 0, 1, 0, 1, 1)
    b += bytes(28+24+4)+u('QQQI', 123, maximum, 100, 42)+u('IIII', 0o100644, 0, 0, 1)+bytes(48)
    b += u('I', len(splits))+b''.join(u('Ii', *split) for split in splits)
    b += blob()+bytes(8)+blob()+u('Q', (1 << 64)-1)+blob()+framed(bytes(16))+blob()+bytes(16)
    if v >= 2: b += u('i', -1)
    if v >= 3: b += bytes(8)
    if v >= 4: b += bytes(8)
    if v >= 5: b += u('I', 0)
    if v >= 6: b += b'\0'
    if v >= 7: b += blob()+blob()
    if v >= 8: b += framed(u('I', len(optional))+b''.join(u('Q', kind)+value for kind, value in optional))
    if v >= 9: b += u('Q', 1)
    return framed(b, v)


def reply(result=-4, target=True, dentry=False, created=False, inode_v=9, extra=None):
    trace = b''
    if dentry: trace = inodestat(77, inode_v)+framed(u('IiI', 0, -1, 0))+blob(b'\xff')+framed(bytes(10)+blob(), 2)
    if target: trace += inodestat(v=inode_v)
    if extra is None: extra = framed(u('QI', 777, 0)) if created else b''
    return u('IiIBBB', 0x1301, result, 9, 1, int(dentry), int(target))+blob(trace)+blob(extra)+blob()


def reconnect(v=5, more=False):
    lock = u('IQQQQQB', 1, 4, 0, 8, 9, 10, 2)+u('I', 0)
    cap = framed(blob(b'\xff')+u('QIIQQI', 3, 4, 5, 1, 2, len(lock))+lock+u('Q', 6), 2)
    b = u('IQ', 1, 123)+cap
    realm = u('QQQ', 1, 2, 3)
    b += realm if v == 3 else u('I', 1)+framed(realm)
    if v == 5: b += bytes([more])
    return b


def ref_crc(data):
    # Independent bit-stream polynomial long division, not provider.crc32c.
    result = 0
    for b in data:
        for bit in range(8):
            carry = (result ^ (b >> bit)) & 1
            result >>= 1
            if carry: result ^= 0x82f63b78
    return result


def message(front=b'', *, ty=22, version=7, compat=None, tid=23, seq=1,
            middle=b'', data=b'', auth=True, flags=5, data_off=4095, source_type=None, source_num=None):
    if compat is None: compat = 0 if ty == 26 or (ty in (22, 23) and version in (0, 3)) else 4 if ty == 23 else 1
    if source_type is None:
        source_type = 2 if ty in (26, 28) or (ty == 22 and front[:4] == u('I', 1)) else 8
    if source_num is None: source_num = 0 if source_type == 2 else 4242
    h = u('QQHHHIIIHBQHH', seq, tid, ty, 127, version, len(front), len(middle), len(data), data_off, source_type, source_num, compat, 0)
    h += u('I', ref_crc(h))
    footer = u('III', ref_crc(front), ref_crc(middle), ref_crc(data))
    if auth: footer += bytes(8)  # invented opaque signature; never authentication proof
    return b'\x07'+h+front+middle+data+footer+bytes([flags])


class Peer:
    def __init__(self, *, auth=True, ready=1, limits=None, retries=(), fragmentation=None,
                 reply_features=None, reply_flags=0, tcp=None, reader=None,
                 ports=(3000, 7402), server_nonce=9, global_seq=5, next_global=None):
        self.tcp = tcp or Harness()
        self.reader = reader or CephReader(SALT, **(limits or {}))
        self.ports = ports
        self.server_nonce = server_nonce
        self.seq = [101, 501]
        self.records = []
        self.fragmentation = fragmentation
        for kwargs in [dict(seq=100), dict(seq=500, ack=101, flags=18, reverse=True), dict(seq=101, ack=501, flags=16)]:
            for s in self.tcp.send(ports=self.ports, **kwargs): self.records += self.reader.feed(s)
        self.push(0, b'ceph v027'+legacy())
        self.push(1, b'ceph v027'+legacy(server_nonce)+legacy())
        features = (1 << 23 if auth else 0) | (1 << 6) | (1 << 59)
        server_features = features if reply_features is None else reply_features
        self.features = features
        self.push(0, u('QIIIIIIB', features, 8, global_seq, 0, 32, 2, 3, 0)+b'xyz')
        g, c = global_seq, 0
        for tag, rg, rc in retries:
            self.push(1, u('BQIIIIB', tag, server_features, rg, rc, 32, 2, 0)+b'ab')
            if tag == 4: c = rc
            if tag == 5: g = rg + 1 if next_global is None else next_global
            if tag == 2: c = 0
            self.push(0, u('QIIIIIIB', features, 8, g, c, 32, 2, 3, 0)+b'xyz')
        self.push(1, u('BQIIIIB', ready, server_features, 7, c+1, 32, 2, reply_flags)+b'ab')
        if ready == 13:
            self.push(1, u('Q', 0))
            self.push(0, u('Q', 0))

    def push(self, direction, data, fragment=None):
        out = []
        step = fragment or self.fragmentation or max(1, len(data))
        for i in range(0, len(data), step):
            part = data[i:i+step]
            for span in self.tcp.send(seq=self.seq[direction], ack=self.seq[1-direction],
                                      flags=24, data=part, reverse=bool(direction), ports=self.ports):
                out += self.reader.feed(span)
            self.seq[direction] += len(part)
        self.records += out
        return out

    def negotiate_session(self):
        self.push(0, message(session()))
        self.push(1, message(session(1, 1), version=1))

    def finish(self):
        result = self.reader.finish(dict(drained=True, parser_errors=0))
        self.tcp.finish()
        return result

    def reconnect_transport(self, reset=False, server_nonce=None, seq_ack=0, connect_seq=None, global_seq=8):
        for s in self.tcp.send(seq=self.seq[0], ack=self.seq[1], flags=20, ports=self.ports):
            self.records += self.reader.feed(s)
        initial = 1000*len(self.tcp.reader.streams)
        self.seq = [initial+1, initial+501]
        for kwargs in [dict(seq=initial), dict(seq=initial+500, ack=initial+1, flags=18, reverse=True), dict(seq=initial+1, ack=initial+501, flags=16)]:
            for s in self.tcp.send(ports=self.ports, **kwargs): self.records += self.reader.feed(s)
        self.push(0, b'ceph v027'+legacy())
        if server_nonce is None: server_nonce = self.server_nonce
        self.server_nonce = server_nonce
        self.push(1, b'ceph v027'+legacy(server_nonce)+legacy())
        c = len(self.tcp.reader.streams)-1 if connect_seq is None else connect_seq
        self.push(0, u('QIIIIIIB', self.features, 8, global_seq, c, 32, 2, 0, 0))
        if reset:
            self.push(1, u('BQIIIIB', 2, self.features, 9, 0, 32, 0, 0))
            self.push(0, u('QIIIIIIB', self.features, 8, global_seq, 0, 32, 2, 0, 0))
            self.push(1, u('BQIIIIB', 1, self.features, 9, 1, 32, 0, 0))
        else:
            self.push(1, u('BQIIIIB', 13, self.features, 9, c+1, 32, 0, 0)+u('Q', seq_ack))
            self.push(0, u('Q', 0))


class FrontTests(unittest.TestCase):
    def decode(self, ty, v, b, data=b'', middle=0, compat=None, features=FEATURES, server=FEATURES):
        if compat is None: compat = 0 if ty == 26 or (ty in (22, 23) and v in (0, 3)) else 4 if ty == 23 else 1
        return decode_front(ty, v, compat, b, middle, len(data), data, SALT, features, server)

    def test_literal_vectors(self):
        vectors = json.loads((Path(__file__).parent/'fixtures/g3-wire-ceph/vectors.json').read_text())
        for v in vectors['fronts']:
            with self.subTest(name=v['name']):
                f = self.decode(v['type'], v['version'], bytes.fromhex(v['hex']),
                                data=bytes.fromhex(v.get('data_hex', '')), compat=v['compat'])
                for k, value in v['expected'].items(): self.assertEqual(f[k], value)
        self.assertEqual(crc32c(b'123456789'), 0x58e3fa20)
        self.assertEqual(HEADER.size, 53)

    def test_requests_all_versions_heads_binary_empty_paths(self):
        for v in (4, 5, 6):
            for hv in (1, 2, 3):
                with self.subTest(v=v, hv=hv):
                    r = self.decode(24, v, request(v, hv))
                    self.assertEqual(r['retry'], 44 if hv == 1 else 300)
                    self.assertEqual(r['path'], dict(field='Path', base=10, len=5, hash=hashlib.sha256(SALT+b'\xff/new').hexdigest()))
                    self.assertEqual(r['path2']['hash'], hashlib.sha256(SALT).hexdigest())
                    self.assertEqual(r['path2']['base'], 20)
                    self.assertEqual(r['open']['flags'], 0o1100)
                    self.assertFalse(r['replay'])

    def test_request_replay_openflags_and_n4_filelock(self):
        r = self.decode(24, 6, request(flags=1))
        self.assertTrue(r['replay'])
        args = u('BBQQQQB', 1, 2, 77, 99, 123, 0, 1)+bytes(21)
        for op in (0x1109, 0x110):
            r = self.decode(24, 6, request(op=op, args=args))
            self.assertIsNone(r['open'])
            self.assertEqual(r['filelock'], dict(rule=1, type=2, owner=77, pid=99, start=123, length=0, to_eof=True, wait=True))

    def test_each_required_suffix_truncation(self):
        for ty, versions, maker in [(24, (4, 5, 6), request), (0x310, range(10, 15), caps), (22, range(8), session)]:
            for v in versions:
                b = maker(v)
                for n in range(len(b)):
                    with self.subTest(ty=ty, v=v, at=n):
                        with self.assertRaises(WireError): self.decode(ty, v, b[:n], compat=0 if v == 0 else 1)
        for v in (3, 4, 5):
            b = reconnect(v)
            # v3 can validly end before its optional raw realm array.
            for n in range(len(b)):
                if v == 3 and n == len(b)-24: continue
                with self.subTest(reconnect=v, at=n):
                    with self.assertRaises(WireError): self.decode(23, v, b'', b[:n], compat=0 if v == 3 else 4)

    def test_caps_versions_scalars_and_segments(self):
        for v in range(10, 15):
            f = self.decode(0x310, v, caps(v))
            self.assertEqual(f['range'], dict(size=123, max_size=456, truncate_size=111, truncate_seq=42))
            self.assertEqual((f['osd_barrier'], f['oldest_flush_tid'], f['flags']), (88, 12, 1))
            self.assertEqual(f['oserrno'], -5 if v == 14 else None)
        self.assertIsNone(self.decode(0x310, 14, caps(op=3))['range'])
        self.assertEqual(self.decode(0x310, 12, caps(12, op=4))['op'], 4)
        with self.assertRaisesRegex(WireError, 'middle mismatch'): self.decode(0x310, 14, caps(xattr=1))

    def test_covering_ack_metadata_does_not_narrow(self):
        for op in (8, 10):
            f = self.decode(0x310, 14, caps(op=op, cap=999, seq=991, dirty=0))
            f['tid'] = 100
            self.assertEqual(covers_flush_ack(f, 1234, 99), op == 8)
            self.assertFalse(covers_flush_ack(f, 1235, 99))
            self.assertFalse(covers_flush_ack(f, 1234, 101))

    def test_session_versions_whitelist_and_decimal_timeout(self):
        for v in range(8):
            f = self.decode(22, v, session(v), compat=0 if v == 0 else 1)
            self.assertEqual((f['op'], f['seq']), (0, 555))
            self.assertNotIn('do-not-export', repr(f))
            self.assertNotIn('/private', repr(f))
            self.assertEqual(f['features'], FEATURES if v >= 3 else None)
        for raw in (b'-1', b'300 ', b'4294967296', b'', b'\xff', b'+1'):
            f = self.decode(22, 7, session(metadata={b'timeout': raw}))
            self.assertEqual(f['timeout']['kind'], 'Malformed')
        self.assertEqual(self.decode(22, 7, session(metadata={b'timeout': b'0'}))['timeout'], dict(kind='Valid', seconds=0))

    def test_reply_inodes_modern_versions_and_distinct_created(self):
        for v in range(1, 10):
            f = self.decode(26, 1, reply(target=True, dentry=True, created=False, inode_v=v))
            self.assertEqual(f['result'], -4)
            self.assertTrue(f['safe'])
            self.assertEqual((f['target']['ino'], f['dentry_parent']['ino']), (555, 77))
            self.assertEqual((f['target']['caps'], f['target']['max_size']), (0, 0))
            self.assertIsNone(f['created_ino'])
        f = self.decode(26, 1, reply(result=0, created=True))
        self.assertEqual((f['target']['ino'], f['created_ino']), (555, 777))
        self.assertIsNone(self.decode(26, 1, reply(target=False))['target'])
        with self.assertRaisesRegex(WireError, 'SESSION'): self.decode(26, 1, reply(), features=None)
        with self.assertRaisesRegex(WireError, 'legacy trace'): self.decode(26, 1, reply(), features=0)

    def test_reply_truncation_and_counts(self):
        for v in range(1, 10):
            b = reply(dentry=True, created=True, inode_v=v)
            for n in range(len(b)):
                with self.subTest(v=v, at=n):
                    with self.assertRaises(WireError): self.decode(26, 1, b[:n])
        with self.assertRaises(WireError): self.decode(26, 1, b+bytes(1))

    def test_reconnect_mixed_chunks_locks_and_final(self):
        for v in (3, 4, 5):
            f = self.decode(23, v, b'', reconnect(v, v == 5), compat=0 if v == 3 else 4)
            self.assertEqual((len(f['caps']), len(f['realms'])), (1, 1))
            self.assertEqual(f['caps'][0]['locks'][0]['owner'], 9)
            self.assertEqual(f['caps'][0]['path']['hash'], hashlib.sha256(SALT+b'\xff').hexdigest())
            self.assertEqual(f['final'], v != 5)
        self.assertTrue(self.decode(23, 5, b'', u('IIB', 0, 0, 0), compat=4)['final'])
        with self.assertRaises(WireError): self.decode(23, 5, b'x', reconnect(), compat=4)

    def test_reclaim_full_address_forms(self):
        f = self.decode(27, 1, blob(b'uuid-\xff')+u('I', 0x80000000))
        self.assertEqual(f['uuid'], hashlib.sha256(SALT+b'uuid-\xff').hexdigest())
        self.assertEqual(f['flags'], 1 << 31)
        for a in (legacy(), legacy(ipv6=True), b'\x02'+u('I', 2)+addr2()+addr2(9, True)):
            f = self.decode(28, 1, u('iI', -2, 99)+a)
            self.assertEqual(f['result'], -2)
            self.assertEqual(f['addresses'][0]['port'], 7402)
            self.assertEqual(f['addresses'][0]['nonce'], 8 if a[0] == 2 else 7)
            self.assertEqual(f['addresses'][-1]['scope'], 9 if len(f['addresses']) == 2 or len(a) == 136 and a[8:10] == b'\0\x0a' else 0)
        self.assertEqual(self.decode(28, 1, bytes(8)+b'\x02'+bytes(4))['addresses'], [])

    def test_relevant_version_count_and_size_failures(self):
        for ty in (22, 23, 24, 26, 27, 28, 0x310):
            with self.assertRaisesRegex(WireError, 'WireCephFields'): self.decode(ty, 99, b'')
        with self.assertRaisesRegex(WireError, 'ObserverCapacity'):
            self.decode(22, 7, bytes(28)+u('I', 1025))
        with self.assertRaisesRegex(WireError, 'ObserverCapacity'):
            self.decode(28, 1, bytes(8)+b'\x02'+u('I', 65)+bytes(65))
        for payload in (blob(b'uuid')+u('I', 1), u('iI', -2, 99)+b'\x02'+u('I', 2)+addr2()+addr2(9, True)):
            ty = 27 if payload[:4] == u('I', 4) else 28
            for cut in range(len(payload)):
                with self.subTest(ty=ty, cut=cut):
                    with self.assertRaises(WireError): self.decode(ty, 1, payload[:cut])

    def test_request_head_sizes_struct_length_and_every_head_cut(self):
        for hv, size in ((1, 98), (2, 106), (3, 118)):
            raw = request(v=4, hv=hv)
            self.assertEqual(raw[size:size+13], b'\x01'+u('QI', 10, 5))
            for cut in range(len(raw)):
                with self.subTest(hv=hv, cut=cut):
                    with self.assertRaises(WireError): self.decode(24, 4, raw[:cut])
        raw = bytearray(request())
        raw[106:110] = u('I', 117)
        with self.assertRaisesRegex(WireError, 'struct_len'): self.decode(24, 6, raw)

    def test_inode_nonempty_split_map_and_optional_payload_lengths(self):
        target = inodestat(splits=((0, 1), (0x1000000, 2)),
                           optional=((1, framed(blob(b'private-charmap'))), (2, framed(b'\x01'+bytes(12)))))
        raw = u('IiIBBB', 0x302, 0, 1, 1, 0, 1)+blob(target)+blob()+blob()
        self.assertEqual(self.decode(26, 1, raw)['target']['ino'], 555)
        self.assertNotIn('private-charmap', repr(self.decode(26, 1, raw)))
        for cut in range(len(target)):
            with self.subTest(cut=cut):
                truncated = u('IiIBBB', 0x302, 0, 1, 1, 0, 1)+blob(target[:cut])+blob()+blob()
                with self.assertRaises(WireError): self.decode(26, 1, truncated)

    def test_reply_extra_selects_server_features_not_requesting_client(self):
        legacy_extra = reply(result=0, extra=u('Q', 777))
        self.assertEqual(self.decode(26, 1, legacy_extra, server=0)['created_ino'], 777)
        self.assertEqual(self.decode(26, 1, reply(created=True), server=FEATURES)['created_ino'], 777)
        with self.assertRaises(WireError): self.decode(26, 1, reply(created=True), server=0)
        with self.assertRaisesRegex(WireError, 'server OPEN'): self.decode(26, 1, reply(created=True), server=None)

    def test_readdir_extra_lease_versions_and_truncation(self):
        for v in (1, 2):
            extra = framed(u('IiI', 0, -1, 0))+u('IH', 1, 3)+blob(b'\xff')+framed(bytes(10)+(blob() if v == 2 else b''), v)+inodestat(v=8)
            front = u('IiIBBB', 0x305, 0, 1, 1, 0, 0)+blob()+blob(extra)+blob()
            self.assertEqual(self.decode(26, 1, front)['op'], 0x305)
            for cut in range(len(extra)):
                broken = u('IiIBBB', 0x305, 0, 1, 1, 0, 0)+blob()+blob(extra[:cut])+blob()
                with self.subTest(lease=v, cut=cut):
                    # Empty extra is valid on an unsuccessful/empty reply.
                    if cut:
                        with self.assertRaises(WireError): self.decode(26, 1, broken)
            with self.assertRaisesRegex(WireError, 'legacy directory'): self.decode(26, 1, front, features=0)

    def test_message_compat_allowlists_and_nested_relevant_versions(self):
        for ty, v, raw, comp in ((26, 1, reply(), 1), (27, 1, blob()+bytes(4), 0),
                                  (28, 1, bytes(8)+b'\x02'+bytes(4), 0), (22, 0, session(0), 1)):
            with self.subTest(ty=ty, comp=comp):
                with self.assertRaisesRegex(WireError, 'WireCephFields'): self.decode(ty, v, raw, compat=comp)
        raw = bytearray(reply())
        raw[19] = 10  # InodeStat frame version.
        with self.assertRaisesRegex(WireError, 'nested version'): self.decode(26, 1, raw)
        for value in (True, -1, 1 << 32):
            with self.assertRaises(WireError): self.decode(22, value, session(1))

    def test_aggregate_count_limit_is_shared_by_nested_cursors(self):
        # 5 cap-auth records each contain 900 gids. Each vector is below 1024,
        # but the aggregate exceeds the admitted 4096 items.
        match = framed(u('qI', -1, 900)+bytes(4*900)+blob()+blob()+b'\0')
        auth = framed(match+b'\x01\x01')
        front = session(5)+u('I', 5)+auth*5
        with self.assertRaisesRegex(WireError, 'aggregate limit'): self.decode(22, 6, front)

    def test_full_address_union_limit_and_ipv6_byte_orders(self):
        union = b'\x02'+u('I', 64)+b''.join(addr2(i) for i in range(64))
        self.assertEqual(len(self.decode(28, 1, bytes(8)+union)['addresses']), 64)
        raw = bytearray(addr2(ipv6=True))
        raw[1+6+12+4:1+6+12+8] = bytes.fromhex('12345678')
        f = self.decode(28, 1, bytes(8)+raw)['addresses'][0]
        self.assertEqual((f['flowinfo'], f['scope'], f['port']), (0x12345678, 9, 7402))

    def test_session_uuid_binary_empty_and_salt_separation(self):
        for value in (b'', b'\0\xff'):
            f = self.decode(22, 2, session(2, metadata={b'uuid': value}))
            self.assertEqual(f['uuid'], hashlib.sha256(SALT+value).hexdigest())
            other = decode_front(22, 2, 1, session(2, metadata={b'uuid': value}), 0, 0, b'', bytes(32))
            self.assertNotEqual(other['uuid'], f['uuid'])

    def test_mixed_legacy_addr2_union_and_finish_flags(self):
        data = bytes(8)+b'\x02'+u('I', 3)+legacy()+addr2(8, True)+addr2(9)
        addresses = self.decode(28, 1, data)['addresses']
        self.assertEqual([(a['type'], a['nonce'], a['family']) for a in addresses],
                         [(1, 7, 'IPv4'), (2, 8, 'IPv6'), (2, 9, 'IPv4')])
        with self.assertRaisesRegex(WireError, 'FINISH must be alone'):
            self.decode(27, 1, blob()+u('I', (1 << 31) | 1))


class FramingTests(unittest.TestCase):
    def test_literal_full_header_and_footer_frame(self):
        vectors = json.loads((Path(__file__).parent/'fixtures/g3-wire-ceph/vectors.json').read_text())
        for vector in vectors['frames']:
            p = Peer(auth=vector['auth'])
            record = p.push(vector['direction'], bytes.fromhex(vector['hex']))[0]
            self.assertEqual(record['header'], vector['expected_header'])
            self.assertEqual((record['fields']['op'], record['fields']['seq']), (2, 2387509390608836392))
            self.assertFalse(record['signature_verified'])
            p.finish()

    def test_one_byte_handshake_challenge_and_feature_intersection(self):
        p = Peer(fragmentation=1, retries=((16, 7, 0),), reply_features=1 << 6)
        success = [r for r in p.records if r['kind'] == 'ConnectReply'][-1]
        self.assertEqual(success['accepted_features'], 1 << 6)
        self.assertEqual(p.push(0, message(session(), auth=False, flags=1))[0]['data_crc'], 'checked')
        for r in p.records:
            self.assertEqual(sum(ref['stream_end']-ref['stream_start'] for ref in r['source_refs']), r['frame_end']-r['frame_offset'])
        p.finish()
        with self.assertRaisesRegex(WireError, 'SEQ without RECONNECT_SEQ'): Peer(ready=13, reply_features=1 << 23)

    def test_reply_requires_completed_observed_session_negotiation(self):
        p = Peer()
        p.push(0, message(session()))
        with self.assertRaisesRegex(WireError, 'SESSION'):
            p.push(1, message(reply(), ty=26, version=1))
        p = Peer()
        p.negotiate_session()
        r = p.push(1, message(reply(extra=u('Q', 777)), ty=26, version=1, seq=2))[0]
        self.assertEqual(r['fields']['created_ino'], 777)  # head-only OPEN advertises no DELEG_INO
        self.assertEqual(r['client_features_record'], p.records[4]['record'])
        self.assertIsNotNone(r['server_features_record'])
        p.finish()

    def test_challenge_retry_and_reset_negotiation_rounds(self):
        p = Peer(retries=((16, 7, 0), (4, 7, 3), (5, 9, 3), (2, 9, 0)))
        replies = [r['tag'] for r in p.records if r['kind'] == 'ConnectReply']
        self.assertEqual(replies, [16, 4, 5, 2, 1])
        self.assertTrue(all(r['authentication_verified'] is False for r in p.records if 'authentication_verified' in r))
        p.finish()

    def test_session_epoch_seq_reconnect_and_random_reset(self):
        p = Peer()
        first = p.push(0, message(session(), seq=100))[0]
        p.reconnect_transport()
        duplicate = p.push(0, message(session(), seq=100))[0]
        self.assertTrue(duplicate['transport_replay'])
        self.assertEqual(duplicate['messenger_epoch'], first['messenger_epoch'])
        self.assertNotEqual(duplicate['stream'], first['stream'])
        p.reconnect_transport(reset=True)
        new = p.push(0, message(session(), seq=1234567))[0]
        self.assertFalse(new['transport_replay'])
        self.assertNotEqual(new['messenger_epoch'], first['messenger_epoch'])
        p.finish()

    def test_negotiation_round_limit_and_bad_transitions(self):
        with self.assertRaisesRegex(WireError, 'negotiation rounds'):
            Peer(limits=dict(max_rounds=1), retries=((16, 7, 0),))
        with self.assertRaisesRegex(WireError, 'negotiation failed'):
            Peer(ready=10)
        with self.assertRaisesRegex(WireError, 'RETRY_SESSION'):
            Peer(retries=((4, 7, 0),))
        with self.assertRaisesRegex(WireError, 'lossy messenger'): Peer(reply_flags=1)

    def test_fragmentation_one_byte_coalescing_dataoff(self):
        p = Peer()
        a = message(session())
        out = p.push(0, a+a.replace(u('Q', 1), u('Q', 1), 0), fragment=1)
        msgs = [r for r in out if r['kind'] == 'Message']
        self.assertEqual(len(msgs), 2)
        self.assertFalse(msgs[0]['transport_replay'])
        self.assertTrue(msgs[1]['transport_replay'])
        self.assertEqual(msgs[0]['header']['data_off'], 4095)
        self.assertEqual(sum(r['stream_end']-r['stream_start'] for r in msgs[0]['source_refs']), len(a))
        p.finish()
        p = Peer()
        out = p.push(0, message(session(), seq=1)+message(session(), seq=2))
        self.assertEqual([r['header']['seq'] for r in out], [1, 2])
        p.finish()

    def test_seq_gap_random_start_and_same_tid_new_instance(self):
        p = Peer(ready=13)
        out = p.push(0, message(session(), seq=0x7ffff000, tid=77)+message(session(), seq=0x7ffff005, tid=77))
        self.assertTrue(all(m['sequence_gap'] for m in out))
        self.assertTrue(all(not m['transport_replay'] for m in out))
        self.assertNotEqual(out[0]['identity'], out[1]['identity'])
        p.finish()

    def test_nonauth_footer_and_all_ready_controls(self):
        p = Peer(auth=False)
        p.push(1, message(session(1, 1), version=1, seq=5, auth=False, flags=1))
        out = p.push(0, b'\x08'+u('Q', 5)+b'\x09'+b'\x0e'+u('II', 1, 2)+b'\x0f'+u('II', 1, 2))
        self.assertEqual([r['tag'] for r in out], [8, 9, 14, 15])
        self.assertEqual(p.push(0, message(session(), auth=False, flags=1))[0]['signature_verified'], False)
        p.push(0, b'\x06')
        p.finish()

    def test_bad_header_front_middle_data_crc_and_complete(self):
        for where in ('header', 'front', 'middle', 'data', 'complete'):
            with self.subTest(where=where):
                p = Peer()
                b = bytearray(message(b'abc', ty=999, version=1, middle=b'd', data=b'ef'))
                at = {'header': 10, 'front': 54, 'middle': 57, 'data': 58, 'complete': len(b)-1}[where]
                b[at] ^= 1
                with self.assertRaisesRegex(WireError, 'WireCephCRC'): p.push(0, b)

    def test_nocrc_omits_only_data_not_front(self):
        p = Peer()
        b = bytearray(message(b'abc', ty=999, version=1, data=b'ef', flags=3))
        b[57] ^= 1
        r = p.push(0, b)[0]
        self.assertEqual((r['fields']['kind'], r['data_crc']), ('Skipped', 'omitted'))
        p.finish()
        p = Peer()
        b[54] ^= 1
        with self.assertRaises(WireError): p.push(0, b)

    def test_actual_session_then_capless_safe_reply(self):
        p = Peer()
        p.negotiate_session()
        out = p.push(1, message(reply(), ty=26, version=1, seq=2))
        self.assertEqual(out[0]['fields']['target']['ino'], 555)
        self.assertIsNone(out[0]['fields']['created_ino'])
        self.assertNotIn('inline-private', repr(out))
        p.finish()

    def test_caps_tid_and_reconnect_data_not_skipped(self):
        p = Peer()
        r = p.push(0, message(caps(), ty=0x310, version=14, tid=999))[0]
        self.assertEqual(r['fields']['tid'], 999)
        r = p.push(0, message(b'', ty=23, version=5, compat=4, data=reconnect(), seq=2))[0]
        self.assertTrue(r['fields']['final'])
        self.assertEqual(len(r['fields']['caps']), 1)
        p.finish()

    def test_mixed_reconnect_chunk_chain(self):
        p = Peer()
        one = p.push(0, message(ty=23, version=5, compat=4, data=reconnect(more=True)))[0]
        two = p.push(0, message(ty=23, version=5, compat=4, data=u('IIB', 0, 0, 0), seq=2))[0]
        self.assertEqual((one['fields']['chunk'], two['fields']['chunk']), (1, 2))
        self.assertEqual(one['fields']['series'], two['fields']['series'])
        self.assertTrue(two['fields']['final'])
        p.finish()

    def test_reclaim_single_outstanding_exchange_not_header_tid(self):
        p = Peer()
        request_record = p.push(0, message(blob(b'uuid')+u('I', 1), ty=27, version=1, tid=999))[0]
        answer = p.push(1, message(bytes(8)+b'\x02'+bytes(4), ty=28, version=1, tid=0))[0]
        self.assertEqual(request_record['fields']['exchange'], answer['fields']['exchange'])
        self.assertEqual(answer['fields']['addresses'], [])
        p.finish()
        p = Peer()
        with self.assertRaisesRegex(WireError, 'without observed exchange'):
            p.push(1, message(bytes(8)+b'\x02'+bytes(4), ty=28, version=1))

    def test_true_tcp_retransmission_does_not_create_message(self):
        p = Peer()
        data = message(session())
        start = p.seq[0]
        records = p.push(0, data)
        spans = p.tcp.send(seq=start, ack=p.seq[1], flags=24, data=data)
        duplicates = [r for s in spans for r in p.reader.feed(s)]
        self.assertEqual([r['kind'] for r in duplicates], ['TransportDuplicate'])
        self.assertEqual(len(records), 1)
        p.finish()

    def test_transient_signature_auth_file_xattr_bytes_not_in_outputs(self):
        p = Peer()
        r = p.push(0, message(request(p2=b'private-symlink'), ty=24, version=6, data=b'private-xattr-value'))[0]
        self.assertNotIn('private-symlink', repr(r))
        self.assertNotIn('private-xattr-value', repr(r))
        self.assertNotIn('opaque-auth', repr(r))
        self.assertFalse(r['signature_verified'])
        self.assertTrue(r['signed'])
        p.finish()

    def test_incomplete_frames_controls_and_unsupported_tag(self):
        for tail in (b'\x07', b'\x08\0', b'\x0e\0', message(session())[:-1]):
            with self.subTest(tail=len(tail)):
                p = Peer()
                p.push(0, tail)
                with self.assertRaises(WireError): p.finish()
        p = Peer()
        with self.assertRaises(WireError): p.push(0, b'\x0d')

    def test_front_and_shared_parser_budget_fail_before_body(self):
        p = Peer(limits=dict(max_front=32))
        with self.assertRaisesRegex(WireError, 'segment length'):
            p.push(0, message(bytes(33), ty=999)[:54])
        p = Peer()
        budget = p.tcp.reader.streams[1].directions[0].budget
        budget.limit = budget.used + 512
        with self.assertRaisesRegex(WireError, 'ObserverCapacity'):
            p.push(0, message(bytes(513), ty=999))

    def test_parser_copy_limit_precedes_front_allocation(self):
        p = Peer()
        budget = p.tcp.reader.streams[1].directions[0].budget
        budget.limit = 1600
        allocations = []
        def allocate(n=0):
            allocations.append(n)
            return bytearray(n)
        with patch('g3_wire_ceph.bytearray', allocate, create=True):
            with self.assertRaisesRegex(WireError, 'direction buffer budget'):
                p.push(0, message(bytes(600))[:54])
        self.assertTrue(p.reader.failed)
        self.assertIn(53, allocations)
        self.assertNotIn(600, allocations)
        self.assertIsNone(p.reader.streams[1]['payload'][0])
        self.assertLessEqual(budget.peak, 1600)

    def test_fin_rst_inside_frame_and_failed_finish_are_terminal(self):
        for event in (17, 20):
            p = Peer()
            p.push(0, message(session())[:-1])
            with self.assertRaisesRegex(WireError, 'transport ended inside'):
                for span in p.tcp.send(seq=p.seq[0], ack=p.seq[1], flags=event): p.reader.feed(span)
        p = Peer()
        with self.assertRaises(WireError): p.reader.finish(dict(drained=True, parser_errors=False))
        self.assertTrue(p.reader.failed)
        with self.assertRaisesRegex(WireError, 'not active'): p.push(0, b'\x09')

    def test_frame_offsets_and_original_ranges_include_negotiation(self):
        p = Peer()
        for direction in (0, 1):
            records = [r for r in p.records if r['direction'] == direction]
            at = 0
            for record in records:
                self.assertEqual(record['frame_offset'], at)
                self.assertEqual(sum(ref['packet_end']-ref['packet_start'] for ref in record['source_refs']), record['frame_end']-at)
                at = record['frame_end']
        before = p.seq[0]-101
        r = p.push(0, message(session()))[0]
        self.assertEqual(r['frame_offset'], before)
        self.assertEqual(r['source_refs'][0]['packet_start'], 54)
        p.finish()
        self.assertTrue(all(d.budget.used == 0 for s in p.tcp.reader.streams.values() for d in s.directions))

    def test_full_frame_reorder_partial_overlap_and_packet_refs(self):
        p = Peer()
        frame = message(request(), ty=24, version=6)
        start = p.seq[0]
        self.assertEqual(p.tcp.send(seq=start+100, ack=p.seq[1], flags=24, data=frame[100:]), [])
        spans = p.tcp.send(seq=start, ack=p.seq[1], flags=24, data=frame[:120])
        out = [record for span in spans for record in p.reader.feed(span)]
        self.assertEqual([r['kind'] for r in out], ['TransportDuplicate', 'Message'])
        self.assertEqual(out[-1]['fields']['path2']['len'], 0)
        self.assertEqual(sum(r['stream_end']-r['stream_start'] for r in out[-1]['source_refs']), len(frame))
        self.assertEqual(len({r['packet'] for r in out[-1]['source_refs']}), 2)
        p.seq[0] += len(frame)
        p.finish()

    def test_reconnect_to_new_server_preserves_only_observed_client_features(self):
        p = Peer()
        p.negotiate_session()
        first = p.records[-1]
        p.reconnect_transport(reset=True, server_nonce=99)
        p.push(0, message(ty=23, version=5, data=reconnect(), seq=100))
        p.push(1, message(session(7, 1), seq=100))
        record = p.push(1, message(reply(created=True), ty=26, version=1, seq=101))[0]
        self.assertEqual(record['client_entity'], 4242)
        self.assertEqual(record['fields']['created_ino'], 777)
        self.assertEqual(record['client_features_record'], first['client_features_record'])
        self.assertNotEqual(record['messenger_epoch'], first['messenger_epoch'])
        p.finish()

    def test_ack_and_seq_do_not_manufacture_unobserved_messages(self):
        p = Peer()
        p.push(0, message(session(), seq=50))
        r = p.push(1, b'\x08'+u('Q', 50))[0]
        self.assertEqual(r['kind'], 'Control')
        with self.assertRaisesRegex(WireError, 'ACK acknowledges unobserved'): p.push(1, b'\x08'+u('Q', 51))
        p = Peer()
        p.push(0, message(session(), seq=50))
        with self.assertRaisesRegex(WireError, 'SEQ acknowledges unobserved'): p.reconnect_transport(seq_ack=51)

    def test_stale_generation_and_source_entity_fail(self):
        for options in (dict(connect_seq=0), dict(global_seq=4)):
            with self.subTest(options=options):
                p = Peer()
                with self.assertRaisesRegex(WireError, 'stale messenger'): p.reconnect_transport(**options)
        p = Peer()
        p.push(0, message(session()))
        with self.assertRaisesRegex(WireError, 'client entity changed'):
            p.push(0, message(session(), seq=2, source_num=4243))

    def test_server_reply_after_seq_uses_original_client_not_mds_source(self):
        p = Peer()
        p.negotiate_session()
        before = p.push(1, message(reply(), ty=26, version=1, seq=2))[0]
        p.reconnect_transport()
        after = p.push(1, message(reply(), ty=26, version=1, seq=3))[0]
        self.assertEqual(after['header']['source_num'], 0)
        self.assertEqual(after['client_entity'], 4242)
        self.assertEqual(after['client_features_record'], before['client_features_record'])
        self.assertEqual(after['messenger_epoch'], before['messenger_epoch'])
        p.finish()

    def test_caps_middle_skipped_but_crc_and_size_validated(self):
        p = Peer()
        r = p.push(0, message(caps(12, xattr=7), ty=0x310, version=12, middle=b'private'))[0]
        self.assertEqual(r['fields']['range']['max_size'], 456)
        self.assertNotIn('private', repr(r))
        p.finish()

    def test_reclaim_pending_survives_seq_reconnect_and_finish_is_not_exchange(self):
        p = Peer()
        req = p.push(0, message(blob(b'uuid')+u('I', 1), ty=27, version=1))[0]
        p.reconnect_transport()
        answer = p.push(1, message(bytes(8)+b'\x02'+bytes(4), ty=28, version=1))[0]
        self.assertEqual(req['fields']['exchange'], answer['fields']['exchange'])
        finish = p.push(0, message(blob()+u('I', 1 << 31), ty=27, version=1, seq=2))[0]
        self.assertIsNone(finish['fields']['exchange'])
        p.finish()

    def test_reclaim_multiple_outstanding_rejected(self):
        p = Peer()
        p.push(0, message(blob(b'uuid')+u('I', 1), ty=27, version=1))
        with self.assertRaisesRegex(WireError, 'ambiguous outstanding'):
            p.push(0, message(blob(b'uuid2')+u('I', 1), ty=27, version=1, seq=2))

    def test_footer_flags_and_unknown_message_lengths(self):
        for auth, flags in ((False, 5), (True, 0), (True, 9)):
            with self.subTest(auth=auth, flags=flags):
                p = Peer(auth=auth)
                with self.assertRaises(WireError): p.push(0, message(b'', ty=999, auth=auth, flags=flags))
        p = Peer(limits=dict(max_payload=8))
        with self.assertRaisesRegex(WireError, 'segment length'):
            p.push(0, message(b'', ty=999, data=bytes(9))[:54])

    def test_request_head_selection_requires_negotiated_fs_btime(self):
        p = Peer(reply_features=(1 << 23) | (1 << 6))
        with self.assertRaisesRegex(WireError, 'REQUEST legacy head without FS_BTIME'):
            p.push(0, message(request(), ty=24, version=6))

    def test_record_and_source_range_limit_before_growth(self):
        p = Peer(limits=dict(max_records=4))
        with self.assertRaisesRegex(WireError, 'record count'): p.push(0, b'\x09')
        self.assertEqual(p.reader.records, 4)
        p = Peer()
        with self.assertRaisesRegex(WireError, 'source-range count'):
            p.push(0, message(bytes(4100), ty=999), fragment=1)
        queue = p.reader.streams[1]['q'][0]
        self.assertEqual(len(queue.refs), 4096)
        self.assertLessEqual(queue.budget.peak, queue.budget.limit)

    def test_reconnect_series_chunk_and_cap_limit_plus_one(self):
        p = Peer()
        empty_more = u('IIB', 0, 0, 1)
        for seq in range(1, 65): p.push(0, message(ty=23, version=5, data=empty_more, seq=seq))
        with self.assertRaisesRegex(WireError, 'reconnect series bound'):
            p.push(0, message(ty=23, version=5, data=empty_more, seq=65))
        p = Peer()
        cap = framed(blob()+u('QIIQQI', 1, 0, 0, 1, 1, 0)+u('Q', 0), 2)
        for seq in range(1, 6):
            count = 1024 if seq < 5 else 1
            data = u('I', count)+b''.join(u('Q', ino)+cap for ino in range(count))+u('IB', 0, 1)
            if seq < 5: p.push(0, message(ty=23, version=5, data=data, seq=seq))
            else:
                with self.assertRaisesRegex(WireError, 'reconnect series bound'):
                    p.push(0, message(ty=23, version=5, data=data, seq=seq))

    def test_wcep1_reconnect_servers_interleave_independently(self):
        for nonce in (9, 10):
            with self.subTest(server_nonce=nonce):
                a = Peer()
                b = Peer(tcp=a.tcp, reader=a.reader, ports=(3001, 7403), server_nonce=nonce)
                a1 = a.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
                b1 = b.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
                a2 = a.push(0, message(ty=23, version=5, data=reconnect(), seq=2))[0]
                b2 = b.push(0, message(ty=23, version=5, data=reconnect(), seq=2))[0]
                self.assertEqual(a1['client_entity'], b1['client_entity'])
                self.assertNotEqual(a1['messenger_epoch'], b1['messenger_epoch'])
                for first, last in ((a1, a2), (b1, b2)):
                    self.assertEqual((first['fields']['series'], last['fields']['series']), (1, 1))
                    self.assertEqual((first['fields']['chunk'], last['fields']['chunk']), (1, 2))
                    self.assertTrue(last['fields']['final'])
                    self.assertFalse(first['fields']['previous_incomplete'])
                    self.assertFalse(last['fields']['previous_incomplete'])
                self.assertEqual(len(a.reader.reconnect_chains), 2)
                a.finish()

    def test_wcep1_per_server_chunk_cap_and_realm_limits(self):
        def data(kind, count, offset):
            if kind == 'caps':
                records = b''.join(u('Q', ino)+framed(blob()+u('QIIQQIQ', ino+1, 0, 0, 1, 1, 0, 0), 2)
                                   for ino in range(4096+offset, 4096+offset+count))
                return u('I', count)+records+u('IB', 0, 1)
            if kind == 'realms':
                records = b''.join(framed(u('QQQ', ino, 1, 0))
                                   for ino in range(4096+offset, 4096+offset+count))
                return u('II', 0, count)+records+b'\x01'
            return u('IIB', 0, 0, 1)

        for kind in ('chunks', 'caps', 'realms'):
            with self.subTest(bound=kind):
                a = Peer()
                b = Peer(tcp=a.tcp, reader=a.reader, ports=(3001, 7403), server_nonce=10)
                chunks = 64 if kind == 'chunks' else 4
                count = 0 if kind == 'chunks' else 1024
                for seq in range(1, chunks+1):
                    af = a.push(0, message(ty=23, version=5, seq=seq,
                                          data=data(kind, count, (seq-1)*count)))[0]['fields']
                    bf = b.push(0, message(ty=23, version=5, seq=seq,
                                          data=data(kind, 0 if kind == 'chunks' else 1, 65536+seq)))[0]['fields']
                    self.assertEqual((af['series'], bf['series'], af['chunk'], bf['chunk']), (1, 1, seq, seq))
                    if kind != 'chunks': self.assertEqual(len(af[kind]), 1024)
                with self.assertRaisesRegex(WireError, 'ObserverCapacity: reconnect series bound'):
                    a.push(0, message(ty=23, version=5, seq=chunks+1,
                                      data=data(kind, 0 if kind == 'chunks' else 1, 8192)))
                with self.assertRaisesRegex(WireError, 'not active'):
                    a.reader.finish(dict(drained=True, parser_errors=0))

    def test_wcep1_chain_row_admission_and_lifetime(self):
        a = Peer(limits=dict(max_reconnect_chains=1))
        b = Peer(tcp=a.tcp, reader=a.reader, ports=(3001, 7403), server_nonce=10)
        first = a.push(0, message(ty=23, version=5, data=reconnect()))[0]['fields']
        second = a.push(0, message(ty=23, version=5, data=reconnect(), seq=2))[0]['fields']
        self.assertEqual((first['series'], second['series'], second['chunk']), (1, 2, 1))
        retained = {key: dict(chain) for key, chain in a.reader.reconnect_chains.items()}
        self.assertEqual(len(retained), 1)
        with self.assertRaisesRegex(WireError, 'ObserverCapacity: reconnect chain table'):
            b.push(0, message(ty=23, version=5, data=reconnect()))
        self.assertEqual(a.reader.reconnect_chains, retained)
        for value in (0, 129, True, 1.0):
            with self.assertRaisesRegex(WireError, 'ObserverCapacity: Ceph limits'):
                CephReader(SALT, max_reconnect_chains=value)

    def test_wcep1_seq_and_interruption_are_server_local(self):
        for reset in (False, True):
            with self.subTest(reset=reset):
                a = Peer()
                b = Peer(tcp=a.tcp, reader=a.reader, ports=(3001, 7403), server_nonce=10)
                a1 = a.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
                b1 = b.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
                a.reconnect_transport(reset=reset)
                a2 = a.push(0, message(ty=23, version=5, seq=2, data=reconnect()))[0]
                b2 = b.push(0, message(ty=23, version=5, seq=2, data=reconnect()))[0]
                self.assertEqual(a1['messenger_epoch'] == a2['messenger_epoch'], not reset)
                self.assertEqual((a2['fields']['series'], a2['fields']['chunk']), (2, 1) if reset else (1, 2))
                self.assertEqual(a2['fields']['previous_incomplete'], reset)
                self.assertEqual((b1['fields']['series'], b2['fields']['series'], b2['fields']['chunk']), (1, 1, 2))
                self.assertFalse(b2['fields']['previous_incomplete'])
                self.assertTrue(a2['fields']['final'] and b2['fields']['final'])
                a.finish()

    def test_wcep1_obsolete_epoch_cannot_reset_chain(self):
        a = Peer()
        old = a.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
        successor = Peer(tcp=a.tcp, reader=a.reader, ports=(3001, 7402))
        new = successor.push(0, message(ty=23, version=5, data=reconnect(more=True)))[0]
        self.assertGreater(new['messenger_epoch'], old['messenger_epoch'])
        self.assertEqual((new['fields']['series'], new['fields']['chunk'], new['fields']['previous_incomplete']), (2, 1, True))
        retained = {key: dict(chain) for key, chain in a.reader.reconnect_chains.items()}
        with self.assertRaisesRegex(WireError, 'WireCephFields: obsolete reconnect epoch'):
            a.push(0, message(ty=23, version=5, seq=2, data=reconnect(more=True)))
        self.assertEqual(a.reader.reconnect_chains, retained)

    def test_wcep2_deferred_fin_keeps_packet8_through_packet9(self):
        p = Peer()
        frame = message(session(1, op=2), version=1)
        start = p.seq[0]
        self.assertEqual(p.tcp.n, 7)
        self.assertEqual(p.tcp.send(seq=start+20, ack=p.seq[1], flags=17, data=frame[20:]), [])
        rows = [r for span in p.tcp.send(seq=start, ack=p.seq[1], flags=24, data=frame[:20])
                for r in p.reader.feed(span)]
        self.assertEqual(p.tcp.n, 9)
        self.assertEqual([r['kind'] for r in rows], ['Message', 'TransportEnd'])
        self.assertEqual(rows[0]['fields']['op'], 2)
        ref = rows[1]['source_refs'][0]
        self.assertEqual((ref['packet'], ref['observed_ns'], ref['source_offset'], ref['packet_start'], ref['packet_end']),
                         (8, 8000, 800000, 0, 54+len(frame)-20))
        self.assertEqual(ref['stream_start'], ref['stream_end'])
        p.seq[0] += len(frame)
        p.finish()

    def test_wcep3_retry_global_advances_both_and_rejects_rollover(self):
        for hint, value in ((90, 101), (101, 102), (90, 150), (0xfffffffe, 0xffffffff)):
            with self.subTest(valid_hint=hint, new=value):
                p = Peer(global_seq=100, retries=((5, hint, 0),), next_global=value)
                connects = [r['global_seq'] for r in p.records if r['kind'] == 'Connect']
                self.assertEqual(connects, [100, value])
                self.assertTrue(p.finish()['drained'])
        for hint, value in ((90, 91), (90, 100), (90, 90), (101, 101), (101, 100)):
            with self.subTest(invalid_hint=hint, new=value):
                with self.assertRaisesRegex(WireError, 'WireCeph1: RETRY_GLOBAL generation did not advance'):
                    Peer(global_seq=100, retries=((5, hint, 0),), next_global=value)
        for previous, hint in ((100, 0xffffffff), (0xffffffff, 90), (0xffffffff, 0xffffffff)):
            with self.subTest(rollover_previous=previous, hint=hint):
                # The wrapped zero is encoded as a valid u32 CONNECT. A pack or
                # parser setup error cannot satisfy this named protocol rejection.
                with self.assertRaisesRegex(WireError, 'WireCeph1: RETRY_GLOBAL u32 rollover unsupported'):
                    Peer(global_seq=previous, retries=((5, hint, 0),), next_global=0)


class StaticTests(unittest.TestCase):
    def test_owned_modules_have_no_runtime_assert_or_io_dependency(self):
        root = Path(__file__).parent.parent
        allowed = {'collections', 'dataclasses', 'hashlib', 'struct', 'g3_wire_tcp'}
        for name in ('g3_wire_tcp.py', 'g3_wire_ceph.py'):
            tree = ast.parse((root/name).read_text())
            for node in ast.walk(tree):
                self.assertNotIsInstance(node, ast.Assert)
                if isinstance(node, ast.Import):
                    self.assertTrue(all(alias.name in allowed for alias in node.names))
                if isinstance(node, ast.ImportFrom): self.assertIn(node.module, allowed)
                if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                    self.assertNotIn(node.func.id, ('open', 'eval', 'exec', 'compile', '__import__', 'input'))


if __name__ == '__main__':
    unittest.main()
