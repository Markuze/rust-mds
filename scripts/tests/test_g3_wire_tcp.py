"""Only synthetic packets. Native process/signal/network guards precede import."""
import os
import signal
import socket
import struct
import subprocess
import unittest
from unittest.mock import patch


def forbidden(*args, **kwargs):
    raise RuntimeError('native action forbidden in offline wire tests')


_guard_targets = (
    (os, 'kill'), (os, 'killpg'), (os, 'system'), (subprocess, 'Popen'),
    (signal, 'raise_signal'), (socket, 'socket'), (socket, 'create_connection'),
) + tuple((os, name) for name in ('fork', 'forkpty', 'posix_spawn', 'posix_spawnp',
    'execv', 'execve', 'execl', 'execle', 'execlp', 'execlpe', 'execvp', 'execvpe',
    'spawnl', 'spawnle', 'spawnlp', 'spawnlpe', 'spawnv', 'spawnve', 'spawnvp', 'spawnvpe'))
_guard_targets += tuple((signal, name) for name in ('pthread_kill', 'pidfd_send_signal', 'alarm', 'setitimer'))
_guards = [patch.object(module, name, forbidden) for module, name in _guard_targets if hasattr(module, name)]
for guard in _guards:
    guard.start()

from g3_wire_tcp import PacketMeta, TcpReader, WireError, Budget


def packet(seq=100, ack=0, flags=2, data=b'', reverse=False, raw=False, ports=(3000, 7402)):
    src, dst = (b'\x7f\0\0\x01', b'\x7f\0\0\x02')
    a, b = ports
    if reverse: src, dst, a, b = dst, src, b, a
    ip = struct.pack('!BBHHHBBH4s4s', 0x45, 0, 40+len(data), 0, 0x4000, 64, 6, 0, src, dst)
    tcp = struct.pack('!HHIIBBHHH', a, b, seq & 0xffffffff, ack & 0xffffffff, 0x50, flags, 4096, 0, 0)
    return (b'' if raw else bytes(12)+b'\x08\x00') + ip + tcp + data


class Harness:
    def __init__(self, **limits):
        self.reader = TcpReader(**limits)
        self.n = 0

    def send(self, *, link='ethernet-ipv4', wire_len=None, interface='lo', **kwargs):
        data = packet(raw=link == 'raw-ipv4', **kwargs)
        self.n += 1
        meta = PacketMeta('fixture', 'boot', 'netns', interface, self.n, self.n*100000,
                          len(data) if wire_len is None else wire_len, self.n*1000, link)
        return self.reader.feed(meta, data)

    def establish(self, isn=100, server=500, **kwargs):
        spans = self.send(seq=isn, **kwargs)
        spans += self.send(seq=server, ack=isn+1, flags=18, reverse=True, **kwargs)
        spans += self.send(seq=isn+1, ack=server+1, flags=16, **kwargs)
        for span in spans: span.release()

    def finish(self):
        return self.reader.finish(dict(drained=True, packets=self.n, dropped=0))


class TcpTests(unittest.TestCase):
    def test_import_and_guards(self):
        with self.assertRaises(RuntimeError): os.kill(123, 0)
        with self.assertRaises(RuntimeError): subprocess.Popen(['false'])
        with self.assertRaises(RuntimeError): socket.socket()

    def test_source_refs_and_bidirectional_bytes(self):
        h = Harness()
        h.establish()
        a = h.send(seq=101, ack=501, flags=24, data=b'abc')[0]
        self.assertEqual((a.stream, a.direction, a.byte_offset, a.data), (1, 0, 0, b'abc'))
        ref = a.source_refs[0]
        self.assertEqual((ref.packet_start, ref.packet_end, ref.stream_start, ref.stream_end), (54, 57, 0, 3))
        self.assertEqual(ref.source_offset, 400000)
        self.assertEqual(a.identity['client_isn'], 100)
        a.release()
        b = h.send(seq=501, ack=104, flags=24, data=b'xy', reverse=True)[0]
        self.assertEqual((b.direction, b.data), (1, b'xy'))
        b.release()
        self.assertEqual(h.finish()['streams'], 1)

    def test_reorder_wrap_and_identical_overlaps(self):
        h = Harness()
        h.establish(isn=0xfffffffd)
        self.assertEqual(h.send(seq=1, ack=501, flags=24, data=b'def'), [])
        spans = h.send(seq=0xfffffffe, ack=501, flags=24, data=b'abcde')
        self.assertEqual(b''.join(s.data for s in spans), b'abcdef')
        for s in spans: s.release()
        duplicates = h.send(seq=0xfffffffe, ack=501, flags=24, data=b'abcdef')
        self.assertEqual([s.event for s in duplicates], ['retransmit'])
        self.assertEqual((duplicates[0].source_refs[0].stream_start, duplicates[0].source_refs[0].stream_end), (0, 6))
        for s in duplicates: s.release()
        self.assertEqual(h.finish()['duplicate_segments'], 2)

    def test_conflicting_pending_overlap_is_fatal(self):
        h = Harness()
        h.establish()
        h.send(seq=104, ack=501, flags=24, data=b'abc')
        with self.assertRaisesRegex(WireError, 'conflicting reordered'):
            h.send(seq=103, ack=501, flags=24, data=b'zz')
        with self.assertRaisesRegex(WireError, 'not active'): h.send()

    def test_conflicting_delivered_overlap(self):
        h = Harness()
        h.establish()
        h.send(seq=101, ack=501, flags=24, data=b'abc')[0].release()
        with self.assertRaisesRegex(WireError, 'conflicting retransmission'):
            h.send(seq=101, ack=501, flags=24, data=b'Xbc')

    def test_unverifiable_old_overlap(self):
        h = Harness(history_bytes=2)
        h.establish()
        h.send(seq=101, ack=501, flags=24, data=b'abc')[0].release()
        with self.assertRaisesRegex(WireError, 'unverifiable old overlap'):
            h.send(seq=101, ack=501, flags=24, data=b'abc')

    def test_hole_fin_and_rst_fail_closed(self):
        for terminal in ('finish', 'rst'):
            with self.subTest(terminal=terminal):
                h = Harness()
                h.establish()
                h.send(seq=104, ack=501, flags=24, data=b'later')
                with self.assertRaises(WireError):
                    if terminal == 'finish': h.finish()
                    else: h.send(seq=101, ack=501, flags=20)

    def test_tuple_reuse_requires_end_and_new_isns(self):
        h = Harness()
        h.establish()
        for s in h.send(seq=101, ack=501, flags=20): s.release()
        h.establish(isn=900, server=1200)
        self.assertEqual(h.finish()['streams'], 2)

    def test_fin_with_data_and_ack(self):
        h = Harness()
        h.establish()
        spans = h.send(seq=101, ack=501, flags=17, data=b'end')
        self.assertEqual([s.event for s in spans], ['data', 'fin'])
        for s in spans: s.release()
        for s in h.send(seq=501, ack=105, flags=17, reverse=True): s.release()
        self.assertEqual(h.finish()['active'], 0)

    def test_profiles_truncation_and_fragment(self):
        for change in ('vlan', 'fragment', 'ipoptions', 'snaplen'):
            with self.subTest(change=change):
                h = Harness()
                p = bytearray(packet())
                if change == 'vlan': p[12:14] = b'\x81\x00'
                if change == 'fragment': p[20:22] = b'\x20\x00'
                if change == 'ipoptions': p[14] = 0x46
                m = PacketMeta('f', 'b', 'n', 'lo', 1, 0, len(p)+(change == 'snaplen'), 1)
                with self.assertRaises(WireError): h.reader.feed(m, p)

    def test_raw_ipv4_and_scope_separation(self):
        h = Harness()
        h.establish(link='raw-ipv4')
        h.establish(isn=200, interface='other', link='raw-ipv4')
        self.assertEqual(h.finish()['streams'], 2)

    def test_stream_limit_plus_one_no_eviction(self):
        h = Harness()
        for i in range(8): h.establish(ports=(3000+i, 7402))
        with self.assertRaisesRegex(WireError, 'stream table full'):
            h.send(ports=(4000, 7402))
        self.assertEqual(len(h.reader.streams), 8)

    def test_budget_before_packet_copy(self):
        h = Harness(direction_bytes=512, history_bytes=16)
        h.establish()
        d = h.reader.streams[1].directions[0]
        with self.assertRaisesRegex(WireError, 'buffer budget'):
            h.send(seq=101, ack=501, flags=24, data=b'x'*513)
        self.assertEqual(d.pending, [])
        self.assertEqual(d.budget.used, 0)

    def test_parser_and_history_share_budget(self):
        b = Budget(10)
        b.take(8)
        with self.assertRaisesRegex(WireError, 'ObserverCapacity'): b.take(3)
        self.assertEqual((b.used, b.peak), (8, 8))

    def test_missing_syn_synack_and_bad_drain(self):
        h = Harness()
        with self.assertRaises(WireError): h.send(seq=101, flags=16)
        h = Harness()
        h.establish()
        with self.assertRaises(WireError): h.reader.finish(dict(drained=True, packets=h.n, dropped=1))

    def test_ack_or_empty_segment_proves_missing_tail(self):
        for method in ('ack', 'empty'):
            with self.subTest(method=method):
                h = Harness()
                h.establish()
                h.send(seq=501 if method == 'ack' else 106, ack=106 if method == 'ack' else 501,
                       flags=16, reverse=method == 'ack')
                with self.assertRaisesRegex(WireError, 'unobserved TCP tail'): h.finish()
                with self.assertRaisesRegex(WireError, 'not active'): h.send()

    def test_reordered_ack_can_precede_complete_data(self):
        h = Harness()
        h.establish()
        h.send(seq=501, ack=106, flags=16, reverse=True)
        for s in h.send(seq=101, ack=501, flags=24, data=b'hello'): s.release()
        self.assertTrue(h.finish()['drained'])

    def test_terminal_ack_fin_duplicates_and_rst_without_ack(self):
        h = Harness()
        h.establish()
        for kwargs in (dict(seq=101, ack=501, flags=17),
                       dict(seq=501, ack=102, flags=17, reverse=True),
                       dict(seq=102, ack=502, flags=16),
                       dict(seq=501, ack=102, flags=17, reverse=True)):
            for span in h.send(**kwargs): span.release()
        self.assertEqual(h.finish()['active'], 0)
        h = Harness()
        h.establish()
        for span in h.send(seq=501, flags=4, reverse=True): span.release()
        self.assertEqual(h.finish()['active'], 0)

    def test_tuple_reuse_does_not_accept_an_older_generation_syn(self):
        h = Harness()
        h.establish()
        for span in h.send(seq=101, ack=501, flags=20): span.release()
        h.establish(isn=900, server=1200)
        for span in h.send(seq=901, ack=1201, flags=20): span.release()
        with self.assertRaisesRegex(WireError, 'same initial sequence'): h.send(seq=100)

    def test_span_lease_copy_pressure_and_exact_limit(self):
        h = Harness(direction_bytes=512, history_bytes=16)
        h.establish()
        # pending bytes+record (356) + temporary/history copies (116) = 472.
        spans = h.send(seq=101, ack=501, flags=24, data=bytes(100))
        d = h.reader.streams[1].directions[0]
        self.assertEqual((d.budget.used, d.budget.peak), (372, 472))
        with self.assertRaisesRegex(WireError, 'buffer budget'):
            h.send(seq=201, ack=501, flags=24, data=bytes(100))
        spans[0].release()
        h = Harness(direction_bytes=472, history_bytes=16)
        h.establish()
        h.send(seq=101, ack=501, flags=24, data=bytes(100))[0].release()
        h.finish()
        h = Harness(direction_bytes=471, history_bytes=16)
        h.establish()
        with self.assertRaisesRegex(WireError, 'buffer budget'):
            h.send(seq=101, ack=501, flags=24, data=bytes(100))

    def test_tcp_interval_packet_total_stream_limits(self):
        h = Harness(max_segments=1)
        h.establish()
        h.send(seq=104, ack=501, flags=24, data=b'a')
        with self.assertRaisesRegex(WireError, 'interval count'):
            h.send(seq=106, ack=501, flags=24, data=b'b')
        h = Harness(max_packets=3)
        h.establish()
        with self.assertRaisesRegex(WireError, 'packet limit'): h.send(flags=16, seq=101, ack=501)
        h = Harness(max_total_streams=1)
        h.establish()
        for span in h.send(seq=101, ack=501, flags=20): span.release()
        with self.assertRaisesRegex(WireError, 'stream table full'): h.send(seq=900)

    def test_invalid_fin_syn_fastopen_and_strict_barriers(self):
        for kwargs in (dict(flags=3), dict(flags=6), dict(flags=2, data=b'x')):
            with self.subTest(kwargs=kwargs):
                with self.assertRaisesRegex(WireError, 'WireTCPProfile'): Harness().send(**kwargs)
        h = Harness()
        h.establish()
        with self.assertRaises(WireError): h.reader.finish(dict(drained=True, packets=h.n, dropped=False))
        with self.assertRaises(WireError): TcpReader(max_streams=True)

    def test_wcep2_deferred_fin_and_duplicate_keep_first_source(self):
        h = Harness()
        h.establish()
        self.assertEqual(h.send(seq=104, ack=501, flags=17, data=b'def'), [])
        d = h.reader.streams[1].directions[0]
        original = d.fin_source
        for span in h.send(seq=104, ack=501, flags=17, data=b'def'): span.release()
        spans = h.send(seq=101, ack=501, flags=24, data=b'abc')
        self.assertEqual([span.event for span in spans], ['data', 'data', 'fin'])
        self.assertEqual(b''.join(span.data for span in spans), b'abcdef')
        self.assertEqual(spans[-1].source_refs, (original,))
        self.assertEqual((original.packet, original.source_offset, original.observed_ns,
                          original.packet_start, original.packet_end, original.stream_start, original.stream_end),
                         (4, 400000, 4000, 0, 57, 6, 6))
        for span in spans: span.release()
        self.assertIs(d.fin_source, original)
        h.finish()
        self.assertIsNone(d.fin_source)
        self.assertEqual(d.budget.used, 0)

    def test_wcep2_empty_fin_before_gap_keeps_original_packet_range(self):
        h = Harness()
        h.establish()
        self.assertEqual(h.send(seq=104, ack=501, flags=17), [])
        self.assertEqual(h.send(seq=104, ack=501, flags=17), [])
        spans = h.send(seq=101, ack=501, flags=24, data=b'abc')
        self.assertEqual([span.event for span in spans], ['data', 'fin'])
        ref = spans[-1].source_refs[0]
        self.assertEqual((ref.packet, ref.source_offset, ref.observed_ns, ref.packet_start, ref.packet_end),
                         (4, 400000, 4000, 0, 54))
        self.assertEqual((ref.stream_start, ref.stream_end), (3, 3))
        for span in spans: span.release()
        h.finish()

    def test_wcep2_fin_anchor_charge_precedes_allocation(self):
        h = Harness(direction_bytes=256, history_bytes=1)
        h.establish()
        h.send(seq=101, ack=501, flags=17)[0].release()
        d = h.reader.streams[1].directions[0]
        anchor = d.fin_source
        self.assertEqual((d.budget.used, d.budget.peak), (256, 256))
        self.assertEqual(h.send(seq=101, ack=501, flags=17), [])
        self.assertIs(d.fin_source, anchor)
        self.assertEqual(d.budget.used, 256)
        h.finish()
        self.assertEqual(d.budget.used, 0)
        h = Harness(direction_bytes=255, history_bytes=1)
        h.establish()
        with patch('g3_wire_tcp.SourceRef') as allocate:
            with self.assertRaisesRegex(WireError, 'ObserverCapacity: direction buffer budget'):
                h.send(seq=101, ack=501, flags=17)
            allocate.assert_not_called()
        d = h.reader.streams[1].directions[0]
        self.assertIsNone(d.fin)
        self.assertIsNone(d.fin_source)
        self.assertEqual(d.budget.used, 0)

    def test_wcep2_conflicting_fin_and_unfilled_fin_hole_stay_fatal(self):
        h = Harness()
        h.establish()
        h.send(seq=104, ack=501, flags=17)
        original = h.reader.streams[1].directions[0].fin_source
        with self.assertRaisesRegex(WireError, 'conflicting FIN'):
            h.send(seq=103, ack=501, flags=17)
        self.assertIs(h.reader.streams[1].directions[0].fin_source, original)
        h = Harness()
        h.establish()
        h.send(seq=104, ack=501, flags=17)
        with self.assertRaisesRegex(WireError, 'unresolved TCP hole'): h.finish()


if __name__ == '__main__':
    unittest.main()
