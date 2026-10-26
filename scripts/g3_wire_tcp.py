"""Offline bounded Ethernet-II/raw IPv4 TCP reassembly for WireCeph1.

No capture, socket, filesystem, clock, or process operation occurs here. PacketMeta
is supplied by a separately qualified acquisition provider. Packet offsets refer to
that provider's immutable source, not a new normalized packet counter.
"""
from dataclasses import dataclass, field
import struct


class WireError(ValueError):
    """Fatal provider failure. Messages contain no input payload bytes."""

    def __init__(self, code, reason):
        self.code = code
        self.reason = reason
        super().__init__(f"{code}: {reason}")


def require(ok, code, reason):
    if not ok:
        raise WireError(code, reason)


@dataclass
class Budget:
    """Shared TCP history/reassembly/span/parser copy budget, per direction."""
    limit: int = 4 * 1024 * 1024
    used: int = 0
    peak: int = 0

    def take(self, size):
        require(type(size) is int and size >= 0 and type(self.limit) is int
                and 0 < self.limit <= 4 * 1024 * 1024 and self.used + size <= self.limit,
                "ObserverCapacity", "direction buffer budget")
        self.used += size
        self.peak = max(self.peak, self.used)

    def drop(self, size):
        require(0 <= size <= self.used, "WireTCP", "buffer ownership mismatch")
        self.used -= size


@dataclass(frozen=True)
class PacketMeta:
    capture: str
    boot: str
    netns: str
    interface: str
    packet: int
    source_offset: int
    wire_len: int
    observed_ns: int
    link: str = "ethernet-ipv4"


@dataclass(frozen=True)
class SourceRef:
    capture: str
    packet: int
    source_offset: int
    packet_start: int
    packet_end: int
    observed_ns: int
    stream_start: int
    stream_end: int


@dataclass
class Span:
    """Transient owned bytes. Feed once into CephReader, which releases the lease.

    identity is sanitized connection metadata. source refs cover exact byte ranges.
    SYN/FIN/RST events carry no application bytes. Never serialize Span.data.
    """
    stream: int
    direction: int
    byte_offset: int
    data: bytes
    source_refs: tuple
    identity: dict
    budget: Budget
    event: str = "data"
    _released: bool = False
    _metadata_charge: int = 0

    def release(self):
        require(not self._released, "WireTCP", "span already released")
        self.budget.drop(len(self.data) + self._metadata_charge)
        self.data = b""
        self._released = True


@dataclass
class _Direction:
    isn: int
    budget: Budget
    next: int = 0
    history_start: int = 0
    history: bytes = b""
    pending: list = field(default_factory=list)
    fin: object = None
    fin_source: object = None
    ended: bool = False
    observed_seq: int = 0
    acknowledged: int = 0


@dataclass
class _Stream:
    number: int
    identity: dict
    client: tuple
    server: tuple
    directions: list
    established: bool = False
    ended: bool = False


class TcpReader:
    """feed(PacketMeta, complete_frame) -> list[Span]; finish(barrier) -> receipt.

    A source/capture is a single observation point. Duplicate loopback copies must
    have the same capture/netns/interface tuple; bytes/sequence, not timing, dedupe
    them. A second observation interface is a different explicit stream domain.
    TCP options are bounded/opaque; IP options, VLAN, IPv6, fragments, SYN data,
    simultaneous open and captures without both SYNs are unsupported, never guessed.
    """

    def __init__(self, *, max_streams=8, max_total_streams=128, max_packets=100000,
                 direction_bytes=4 * 1024 * 1024, history_bytes=65536,
                 max_segments=4096):
        require(all(type(x) is int for x in (max_streams, max_total_streams, max_packets,
                                           direction_bytes, history_bytes, max_segments))
                and 1 <= max_streams <= 8 and 1 <= max_total_streams <= 128
                and 1 <= direction_bytes <= 4 * 1024 * 1024
                and 1 <= history_bytes <= min(direction_bytes, 65536)
                and 1 <= max_segments <= 4096 and 1 <= max_packets <= 1000000,
                "ObserverCapacity", "invalid TCP limits")
        self.limits = (max_streams, max_total_streams, max_packets, direction_bytes,
                       history_bytes, max_segments)
        self.streams = {}
        self.tuples = {}
        self.sources = {}
        self.packets = 0
        self.duplicates = 0
        self.failed = False
        self.finished = False

    def feed(self, packet_meta, packet_bytes):
        require(not self.failed and not self.finished, "WireTCP", "reader not active")
        try:
            return self._feed(packet_meta, packet_bytes)
        except (WireError, struct.error):
            self.failed = True
            raise

    def _feed(self, m, packet):
        require(isinstance(m, PacketMeta), "WireTCP", "PacketMeta required")
        require(all(type(x) is str and 0 < len(x) <= 128 for x in
                    (m.capture, m.boot, m.netns, m.interface)), "WireTCP", "source identity")
        require(all(type(x) is int and 0 <= x < 1 << 64 for x in
                    (m.packet, m.source_offset, m.wire_len, m.observed_ns)), "WireTCP", "packet metadata")
        require(isinstance(packet, (bytes, bytearray, memoryview)) and len(packet) <= 65549,
                "ObserverCapacity", "packet size")
        require(m.wire_len == len(packet), "WireTCP", "snaplen truncation")
        require(m.source_offset + m.wire_len < 1 << 64, "WireTCP", "source offset overflow")
        self.packets += 1
        require(self.packets <= self.limits[2], "ObserverCapacity", "packet limit")
        p = memoryview(packet)
        require(p.ndim == 1 and p.itemsize == 1 and p.c_contiguous,
                "WireTCPProfile", "contiguous packet bytes required")
        p = p.cast('B')
        if m.link == "ethernet-ipv4":
            require(len(p) >= 14 and p[12:14] == b"\x08\x00", "WireTCPProfile", "Ethernet-II IPv4 required")
            ip = 14
        else:
            require(m.link == "raw-ipv4", "WireTCPProfile", "unsupported link type")
            ip = 0
        require(len(p) >= ip + 20 and p[ip] == 0x45 and p[ip + 9] == 6,
                "WireTCPProfile", "IPv4 without options, TCP required")
        total, frag = struct.unpack_from("!HH", p, ip + 2)[0], struct.unpack_from("!H", p, ip + 6)[0]
        require(frag & 0xbfff == 0, "WireTCPProfile", "IP fragmentation/reserved flag")
        require(total >= 40 and ip + total <= len(p), "WireTCP", "incomplete IP packet")
        # Ethernet minimum-frame padding is outside the IPv4 total length.
        require(len(p) == ip + total or (ip == 14 and len(p) <= 60), "WireTCPProfile", "unexpected packet tail")
        tcp = ip + 20
        src, dst, seq, ack = struct.unpack_from("!HHII", p, tcp)
        hlen = (p[tcp + 12] >> 4) * 4
        flags = p[tcp + 13]
        require(20 <= hlen <= 60 and tcp + hlen <= ip + total,
                "WireTCP", "TCP header length")
        require(p[tcp + 12] & 15 == 0 and flags & 0xc0 == 0,
                "WireTCPProfile", "reserved/ECN TCP profile")
        a = (bytes(p[ip + 12:ip + 16]).hex(), src)
        b = (bytes(p[ip + 16:ip + 20]).hex(), dst)
        domain = (m.capture, m.boot, m.netns, m.interface)
        source = self.sources.get(domain)
        if source is None:
            require(len(self.sources) < 128, 'ObserverCapacity', 'source table full')
            source = dict(first_packet=m.packet, first_offset=m.source_offset,
                          last_packet=m.packet, end_offset=m.source_offset, packets=0)
            self.sources[domain] = source
        require(source['packets'] == 0 or (m.packet > source['last_packet']
                and m.source_offset >= source['end_offset']), 'WireTCP', 'reused/reordered source identity')
        source.update(last_packet=m.packet, end_offset=m.source_offset+m.wire_len,
                      packets=source['packets']+1)
        key = (domain, tuple(sorted((a, b))))
        s = self.streams.get(self.tuples.get(key))
        syn, acked, fin, rst = bool(flags & 2), bool(flags & 16), bool(flags & 1), bool(flags & 4)
        require(not (syn and (fin or rst)) and not (fin and rst) and not flags & 32,
                "WireTCPProfile", "unsupported TCP flag combination/urgent data")
        data_start = tcp + hlen
        payload = p[data_start:ip + total]
        require(not syn or not payload, "WireTCPProfile", "TCP fast-open data unsupported")
        result = []
        if syn and not acked:
            if s is not None and not s.ended:
                require(a == s.client and seq == s.directions[0].isn,
                        "WireTCP", "tuple reuse without terminal transport")
                self.duplicates += 1
                return []
            require(not any(old.identity['capture'] == m.capture
                            and old.identity['boot'] == m.boot
                            and old.identity['netns'] == m.netns
                            and old.identity['interface'] == m.interface
                            and old.client == a and old.server == b
                            and old.directions[0].isn == seq for old in self.streams.values()),
                    'WireTCPProfile', 'ambiguous tuple reuse with same initial sequence')
            require(sum(not x.ended for x in self.streams.values()) < self.limits[0]
                    and len(self.streams) < self.limits[1], "ObserverCapacity", "stream table full")
            number = len(self.streams) + 1
            identity = dict(capture=m.capture, boot=m.boot, netns=m.netns,
                            interface=m.interface, client=a, server=b, generation=number,
                            client_isn=seq, server_isn=None,
                            syn_source=dict(packet=m.packet, source_offset=m.source_offset,
                                            observed_ns=m.observed_ns), synack_source=None)
            s = _Stream(number, identity, a, b,
                        [_Direction(seq, Budget(self.limits[3])), None])
            self.streams[number] = s
            self.tuples[key] = number
            return [self._event(s, 0, m, "syn")]
        require(s is not None, "WireTCP", "missing SYN")
        direction = 0 if a == s.client and b == s.server else 1
        if syn:
            require(not s.ended and direction == 1 and acked
                    and ack == ((s.directions[0].isn + 1) & 0xffffffff),
                    "WireTCP", "invalid SYN-ACK")
            if s.directions[1] is not None:
                require(seq == s.directions[1].isn, "WireTCP", "conflicting SYN-ACK")
                self.duplicates += 1
                return []
            s.directions[1] = _Direction(seq, Budget(self.limits[3]))
            s.identity['server_isn'] = seq
            s.identity['synack_source'] = dict(packet=m.packet, source_offset=m.source_offset,
                                               observed_ns=m.observed_ns)
            return [self._event(s, 1, m, "synack")]
        require(s.directions[1] is not None and (acked or rst), "WireTCP", "missing SYN-ACK/ACK")
        if not s.established:
            require(direction == 0 and acked and seq == ((s.directions[0].isn + 1) & 0xffffffff)
                    and ack == ((s.directions[1].isn + 1) & 0xffffffff),
                    "WireTCP", "missing final handshake ACK")
            s.established = True
        d = s.directions[direction]
        offset = self._offset(d, seq)
        require(offset >= 0 and offset <= d.next + self.limits[3], "WireTCP", "unverifiable sequence window")
        d.observed_seq = max(d.observed_seq, offset)
        if acked:
            peer = s.directions[1-direction]
            peer_ack = self._offset(peer, ack)
            require(0 <= peer_ack <= peer.next + self.limits[3] + 1,
                    "WireTCP", "unverifiable ACK window")
            peer.acknowledged = max(peer.acknowledged, peer_ack)
        end = offset + len(payload)
        require(not s.ended or end <= d.next + int(not payload and d.fin is not None),
                "WireTCP", "new bytes after terminal stream")
        require(d.fin is None or end <= d.fin or (not payload and end == d.fin + 1),
                "WireTCP", "payload past FIN")
        if payload:
            start = offset
            overlaps = []
            if start < d.next:
                overlap = min(end, d.next) - start
                require(start >= d.history_start, "WireTCP", "unverifiable old overlap")
                require(payload[:overlap] == memoryview(d.history)[start - d.history_start:start - d.history_start + overlap],
                        "WireTCP", "conflicting retransmission")
                overlaps.append((start, start + overlap))
                start += overlap
            if end > d.next:
                # Compare every pending overlap before making any copy.
                for lo, data, _ in d.pending:
                    hi = lo + len(data)
                    l, r = max(start, lo), min(end, hi)
                    if l < r:
                        require(payload[l - offset:r - offset] == memoryview(data)[l - lo:r - lo],
                                "WireTCP", "conflicting reordered overlap")
                        require(len(overlaps) < self.limits[5] + 1,
                                "ObserverCapacity", "overlap range count")
                        overlaps.append((l, r))
                gaps = [(start, end)]
                for lo, data, _ in d.pending:
                    hi = lo + len(data)
                    remaining = []
                    for x, y in gaps:
                        for l, r in ((x, min(y, lo)), (max(x, hi), y)):
                            if l < r:
                                require(len(remaining) < self.limits[5],
                                        "ObserverCapacity", "reassembly gap count")
                                remaining.append((l, r))
                    gaps = remaining
                require(len(d.pending) + len(gaps) <= self.limits[5], "ObserverCapacity", "reassembly interval count")
                for lo, hi in gaps:
                    d.budget.take(hi - lo + 256)
                    ref = SourceRef(m.capture, m.packet, m.source_offset,
                                    data_start + lo - offset, data_start + hi - offset,
                                    m.observed_ns, lo, hi)
                    d.pending.append((lo, bytes(payload[lo - offset:hi - offset]), ref))
                d.pending.sort(key=lambda item: item[0])
            if overlaps:
                result.append(self._duplicate(s, direction, m, data_start, offset, overlaps))
        while d.pending and d.pending[0][0] == d.next:
            lo, data, ref = d.pending.pop(0)
            # The pending data lease transfers into Span; history is another copy.
            old_len = len(d.history)
            keep_len = min(old_len + len(data), self.limits[4])
            d.budget.take(old_len + len(data) + keep_len)
            keep = (d.history + data)[-keep_len:]
            d.history = keep
            d.budget.drop(2 * old_len + len(data))
            d.next += len(data)
            d.history_start = d.next - len(keep)
            result.append(Span(s.number, direction, lo, data, (ref,), dict(s.identity),
                               d.budget, _metadata_charge=256))
        if fin:
            require(not s.ended or d.fin == end, "WireTCP", "new FIN after terminal stream")
            require(d.fin is None or d.fin == end, "WireTCP", "conflicting FIN")
            if d.fin is None:
                d.budget.take(256)
                d.fin_source = SourceRef(m.capture, m.packet, m.source_offset, 0, m.wire_len,
                                         m.observed_ns, end, end)
                d.fin = end
        if d.fin == d.next and not d.ended:
            require(not d.pending, "WireTCP", "data beyond FIN")
            d.ended = True
            result.append(Span(s.number, direction, d.fin, b"", (d.fin_source,),
                               dict(s.identity), d.budget, 'fin'))
        if rst:
            require(not any(x.pending for x in s.directions), "WireTCP", "RST with unresolved hole")
            require(all(x.observed_seq <= x.next + int(x.ended and x.fin is not None)
                        and x.acknowledged <= x.next + int(x.ended and x.fin is not None)
                        for x in s.directions), "WireTCP", "RST with unobserved TCP bytes")
            for i, peer in enumerate(s.directions):
                if not peer.ended:
                    peer.ended = True
                    result.append(self._event(s, i, m, "rst"))
        if all(x.ended for x in s.directions):
            s.ended = True
        return result

    @staticmethod
    def _offset(direction, sequence):
        return direction.next + (((sequence - (direction.isn + 1 + direction.next)
                                  + (1 << 31)) & 0xffffffff) - (1 << 31))

    def _duplicate(self, s, direction, m, packet_start, offset, ranges):
        self.duplicates += 1
        budget = s.directions[direction].budget
        charge = 256 * len(ranges)
        budget.take(charge)
        refs = tuple(SourceRef(m.capture, m.packet, m.source_offset, packet_start + start-offset,
                               packet_start + end-offset, m.observed_ns, start, end) for start, end in ranges)
        return Span(s.number, direction, ranges[0][0], b"", refs, dict(s.identity),
                    budget, 'retransmit', _metadata_charge=charge)

    @staticmethod
    def _event(s, direction, m, event):
        d = s.directions[direction]
        ref = SourceRef(m.capture, m.packet, m.source_offset, 0, m.wire_len,
                        m.observed_ns, d.next, d.next)
        return Span(s.number, direction, d.next, b"", (ref,), dict(s.identity), d.budget, event)

    def finish(self, barrier):
        require(not self.failed and not self.finished, "WireTCP", "reader not active")
        try:
            return self._finish(barrier)
        except WireError:
            self.failed = True
            raise

    def _finish(self, barrier):
        require(type(barrier) is dict and barrier.get('drained') is True
                and type(barrier.get('packets')) is int and barrier['packets'] == self.packets
                and type(barrier.get('dropped')) is int and barrier['dropped'] == 0,
                "WireTCP", "unproved source drain/loss counters")
        for s in self.streams.values():
            require(s.established and s.directions[1] is not None,
                    "WireTCP", "incomplete TCP establishment")
            for d in s.directions:
                require(not d.pending and (d.fin is None or d.fin == d.next),
                        "WireTCP", "unresolved TCP hole")
                require(d.observed_seq <= d.next + int(d.ended and d.fin is not None)
                        and d.acknowledged <= d.next + int(d.ended and d.fin is not None),
                        "WireTCP", "unobserved TCP tail")
                require(d.budget.used == len(d.history) + 256 * int(d.fin_source is not None),
                        "WireTCP", "unconsumed span/parser buffer")
        for s in self.streams.values():
            for d in s.directions:
                d.budget.drop(len(d.history) + 256 * int(d.fin_source is not None))
                d.history = b""
                d.fin_source = None
        self.finished = True
        return dict(profile='ethernet-or-raw-ipv4-tcp-v1', packets=self.packets,
                    duplicate_segments=self.duplicates, streams=len(self.streams),
                    active=sum(not s.ended for s in self.streams.values()), drained=True,
                    sources=[dict(capture=k[0], boot=k[1], netns=k[2], interface=k[3], **v)
                             for k, v in self.sources.items()],
                    identities=[dict(s.identity) for s in self.streams.values()])
