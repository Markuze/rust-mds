# WireCeph1 offline provider and fixtures

This is P1: bounded TCP/msgr1 framing **and** the required modern metadata fronts.
It is not a capture reader, installed-provider qualification, authentication proof,
peer-processing proof, native G3 result or launcher. P3 supplies acquisition,
source/clock barriers and independent identity. P4 supplies lossless evidence joins.
No existing core helper is a runtime or test dependency.

## Public API and ownership

Import from `scripts` on `PYTHONPATH`. Imports perform no collection or explicit I/O.
The only dependencies are Python's standard library and the owned `g3_wire_tcp`.

```python
from g3_wire_tcp import PacketMeta, TcpReader
from g3_wire_ceph import CephReader

tcp = TcpReader()
ceph = CephReader(run_salt)  # exactly 32 bytes; supplied by the caller
for meta, packet_bytes in injected_complete_packets:
    for span in tcp.feed(meta, packet_bytes):
        for record in ceph.feed(span):
            persist_sanitized_record(record)
ceph_receipt = ceph.finish({"drained": True, "parser_errors": 0})
tcp_receipt = tcp.finish({"drained": True, "packets": input_count, "dropped": 0})
```

The loop is illustrative. This package supplies no source, persistence callback,
socket, pcap parser, signal, clock, random generator or native mode switch.
Feed all spans in emission order before the next packet. Drain returned records.
Call `CephReader.finish` before `TcpReader.finish`. Fixed-read scratch leases stay
charged until the next fixed read or parser finish. An observed interval may end
with active TCP connections, but not with a hole or partial messenger frame.

`PacketMeta(capture, boot, netns, interface, packet, source_offset, wire_len,
observed_ns, link="ethernet-ipv4")` is immutable. The four identity strings have
1–128 characters. Integer fields are unsigned 64-bit values, not booleans.
`packet` and `source_offset` are **original source coordinates**, not a normalized
adapter record counter. Packet IDs must increase within each capture domain.
Packet source ranges must not overlap. Gaps are allowed for container headers or
upstream filtering; they do not prove loss-free acquisition. `observed_ns` remains
in the source clock domain. P3 must qualify that clock and the upstream counters.

`SourceRef` stores capture ID, original packet ID/offset, `[packet_start,packet_end)`,
observation timestamp and `[stream_start,stream_end)`. Packet positions are relative
to the supplied complete frame. Add `source_offset` to obtain absolute source byte
positions. Reordered data retains the original packet references. A frame can have
several references to one packet or references in a different packet arrival order.
No normalized packet or frame counter replaces these coordinates.

`Span(stream, direction, byte_offset, data, source_refs, identity, budget, event)`
contains transient bytes. Direction 0 is the SYN initiator/client; direction 1 is
the server. Stream IDs/generations start at 1. Identity contains the capture domain,
both IPv4/port tuples, both initial sequence numbers, and SYN/SYN-ACK source anchors.
Events are `syn`, `synack`, `data`, `fin`, `rst`, and `retransmit`. A retransmit span
contains **no bytes**; its references describe only verified overlapping ranges.
SYN/FIN/RST references cover the original packet, with zero application-stream
length. Do not persist `Span.data`. Feed each span once. `CephReader` owns its lease
after feed. A TCP-only consumer must call `Span.release()` once instead.

`Budget.take(n)` admits storage before copying. `used`, `peak` and `limit` are public
diagnostics. Both parsers share the same per-direction budget object. Do not mutate
leases, parser internals, span identities or limits during a run.

All emitted dictionaries are sanitized and contain no input views or byte buffers:

- Common messenger fields: `kind`, `record`, `stream`, `direction`, `frame_offset`,
  `frame_end`, `messenger_epoch`, and exact `source_refs`. `record` counts emitted
  records only. It is not upstream continuity evidence.
- `Banner`: advertised address, server-observed peer address when present, and TCP
  identity. `Connect`/`ConnectReply`: features, host/protocol, global/connect sequence,
  opaque authorizer **length only**, flags and reply tag. Successful replies include
  `accepted_features`, `predecessor_stream` and `reset_epoch`.
- `SequenceExchange`/`Control`: actual negotiation SEQ, ACK or keepalive values.
  ACK is transport metadata, never an application SAFE reply.
- `Message`: complete header, decoded `fields`, full identity tuple
  `(stream,direction,frame_offset,messenger_epoch,seq,tid,type)`, observed client
  entity when known, feature-basis record references, replay/gap labels, CRC status,
  and `signed`/`signature_verified` flags. `signature_verified` is always false.
- `TransportDuplicate`/`TransportEnd`: original packet references, TCP identity,
  messenger epoch and event. They have no messenger-frame offsets or message body.

`WireError` exposes static `code`/`reason` strings without payload bytes. Codes are
`WireTCP`, `WireTCPProfile`, `WireCeph1`, `WireCephCRC`, `WireCephFields`, or
`ObserverCapacity`. Feed/finish failures latch failure. A failed reader cannot
continue or issue a successful finish receipt. There is no eviction/retry fallback.
The caller maps errors to prerequisite/runtime outcomes according to trial phase.

`decode_front(type,version,compat,front,middle_len,data_len,data,salt,
session_features=None,server_features=None)` is a pure test/adapter helper. It does
not validate framing, CRCs or source continuity. Its request input is explicitly the
modern FS_BTIME profile. Only the complete reader selects that negotiated profile.
`crc32c(data, crc=0)` supports incremental Ceph CRC32C. `covers_flush_ack(fields,I,T)`
tests CAPS op8, inode I and tid≥T only. The caller must first join the exact native
client/key/rank and the relevant interval. Cap ID, seq and dirty equality do not
narrow coverage. FLUSHSNAP_ACK10 is excluded.

## Supported and unsupported profiles

The link profile is complete Ethernet-II IPv4/TCP without FCS, or `raw-ipv4`.
It supports IPv4 without options, bounded opaque TCP options, ordinary SYN/SYN-ACK/
ACK, sequence wrap, reorder, identical overlaps, FIN and RST (including RST without
ACK). Ethernet minimum-frame padding is outside IPv4 `total_length`. Duplicate
loopback observations with identical bytes/sequence create transport metadata, not
another Ceph message. Conflicting overlap or overlap older than retained history
fails. Peer ACKs and empty-segment sequence positions also expose missing tails.
The first FIN retains its original packet/time/offset/range and byte position.
A later gap-filling packet emits that retained anchor. A duplicate FIN does not
replace it. One256-byte charge per direction admits this record before allocation;
successful TCP finish releases that charge with the overlap history.

Unsupported transport profiles fail: VLAN, IPv6 transport, Linux cooked/pcap link
containers, IP options/fragments/reserved fragmentation flag, ECN/reserved TCP bits,
urgent data, SYN+FIN/RST, FIN+RST, fast-open data, simultaneous open, missing SYN
negotiation, or same-client-ISN tuple reuse across any retained generation. Tuple
reuse with a new observed SYN creates a new stream after terminal transport.
IP/TCP checksums are not verified; offload affects captured checksums. Mandatory
messenger CRCs are verified. Neither a captured packet nor its TCP ACK proves peer
application processing.

Messenger profile: `ceph v027`, legacy address136, connect33, reply26, client entity
type8 to MDS protocol32. Authorizer challenge16 and retries2/4/5 are bounded distinct
states. READY1 starts a new epoch; SEQ13 retains a proved predecessor or starts the
observed connect-seq0 session. Both SEQ directions must complete before message
parsing. SEQ requires negotiated RECONNECT_SEQ bit6. Invalid generations/tags and
failed/WAIT/unknown negotiation replies fail. Lossy connect/reply flags are outside
this lossless MDS profile and fail. msgr2 is unsupported.

RETRY_GLOBAL's next u32 CONNECT generation must exceed **both** the preceding
client generation and the server hint. A lower hint is valid if the client still
advances its own generation. Other connections may advance the shared allocator,
so the next value need not be exactly max+1. This profile does not support u32
rollover: a retry after either preceding value reaches0xffffffff fails explicitly.
The final nonwrapped value0xffffffff itself remains valid when it exceeds both.

The frame is tag7 + packed header53 `<QQHHHIIIHBQHHI>` + front/middle/data + footer.
`data_off` adds **no padding**. MSG_AUTH is bit23; accepted features are the mask
intersection. Footer size is21 with MSG_AUTH, otherwise13. CRC32C is reflected
Castagnoli `0x82f63b78`, seed0, no complement. Header CRC covers49 bytes. Front/middle
CRCs are mandatory. Data CRC is computed while skipping payload and checked unless
NOCRC2 says omitted. COMPLETE1 is required; SIGNED4 without MSG_AUTH fails. Signature
and opaque authorizer bytes never become an authentication claim. ACK8, CLOSE6,
KEEPALIVE9 and KEEPALIVE2/ACK14/15 are parsed only in their proper ready states.

Forward message-sequence gaps are valid and do not imply capture loss. Old sequences
are labeled `transport_replay`; TCP retransmission is a different event. A new
same-tid frame has a new full identity, but is not by itself proof of application
resend. The peer-send/processing and daemon-authentication joins remain external.

| Required message | Version / compat profile | Retained metadata and validation |
|---|---|---|
| REQUEST24 | v4–6 / compat0 or1; head1–3 | Modern98/106/118-byte head; requires negotiated FS_BTIME bit59. Extended retry/fwd, flags separate from open flags, both salted binary paths/base/role/length, open offsets0/4/24/28, N4 filelock offsets0/1/2/10/18/26/34. Validate all44-byte releases/names, stamp, groups and versioned suffix blobs. |
| REPLY26 | v1 / compat0 | Signed I32 result, SAFE head byte, all three blobs, modern InodeStat1–9, DirStat1, LeaseStat1–2. Dentry parent/name and target remain separate. Capless target and absent target are valid. CREATE extra uses observed server OPEN's DELEG_INO bit13; created inode is not target identity. Bounded READDIR/LSSNAP, GETFILELOCK and GETVXATTR extras are consumed. |
| CAPS784 (`0x310`) | v10–14 / compat0 or1 | Head92/body84, scalar caps and header tid, size/max/truncate scalars, range sequence, flags/barrier/oldest tid. Check snap length, xattr middle length, flock and IMPORT peer21, and every version suffix. EXPORT has `range=null`, not invented zeroes. |
| SESSION22 | v0 / compat0; v1–7 / compat0 or1 | Head28/op/seq, salted raw UUID, timeout, supported bits, flags and oldest tid. Parse all metric/cap-auth suffixes; discard authorization context. Timeout is absent/valid/malformed; valid means1–10 ASCII digits with U32 value. Malformed retains only a salted hash. |
| RECONNECT23 | v3 / compat0; v4–5 / compat4 | Empty front, data payload. Walk cap2/compat1 records, binary path hashes, locks and realms. v3 raw realms, v4 counted framed realms without a more byte, v5 explicit bool more. Track series/chunks by observed client and server/session scope; record interruption only within that scope. |
| RECLAIM27 / REPLY28 | v1 / compat1 | Salted complete UUID, flags, signed result/epoch and full address union. FINISH must be alone and expects no reply. One outstanding observed exchange per messenger session; carry it across SEQ reconnect. Header tid is not reclaim correlation. |

Trace encoding uses the client's **accepted REQUEST_OPEN** bitmap, as
`CInode::encode_inodestat` does. The server's OPEN bitmap independently selects
CREATE extra decoding, as `Client::make_request` does. The reader records both
bases. It promotes a pending client bitmap only after observed OPEN. A head-only
OPEN advertises zero server features; it does not erase the client bitmap.
Client features can survive TCP/MDS changes only through observed capture-domain,
advertised address/nonce and client-origin source-entity continuity. They are never
filled from an MDS reply's source number. Unknown or changed identity/features fail
the required join. P3/P4 must still authenticate this passive association.

Reconnect rows use the observed client identity plus the actual TCP server endpoint,
advertised server address/nonce and negotiated host/protocol. Messenger continuity
and server feature bases use that same server scope. Different endpoints cannot
share counters even if they advertise the same address/nonce. Interleaved A/B
chunks keep independent series and64-chunk/4096-cap/4096-realm limits. A SEQ
reconnect within the same messenger epoch continues its chain. A newer epoch
interrupts only that server's unfinished series. An older epoch cannot roll that
chain backward. A changed advertised identity or endpoint starts a separate row;
the decoder does not infer a cross-address logical-rank join.

Addresses preserve type, nonce, family, full address, port, IPv6 flowinfo and scope.
IPv4 uses a zero-prefixed16-byte hex address. Legacy families are BE; ADDR2 families
are LE. Ports and IPv6 flowinfo are BE; scope is LE. Empty addresses stay empty.
ADDR2 marker1 entries and marker2 vectors can mix legacy/ADDR2 IPv4/IPv6 entries.
Duplicate semantic entries fail. Raw address padding is not persisted.

Unknown relevant versions/compatibilities fail `WireCephFields`. Legacy request
heads/reply traces and reconnect1/2 are not guessed. Other message types are fully
framed/CRC/sequence checked before emitting `Skipped`. Snap/inline/fscrypt/xattr/
authorization payloads are bounded and skipped, not exported. Optional inode kind
payloads are bounded opaque versioned frames; their content is not interpreted.
The standalone decoder does not manufacture core `FACTS` or fixture-native flags.

## Fixed admission bounds

Constructors can reduce these limits. They cannot exceed the listed maxima.
`TcpReader` defaults: max_streams8, max_total_streams128, max_packets100000,
direction_bytes4194304, history_bytes65536, max_segments4096.
Packet count can explicitly rise to1000000. History must fit direction_bytes.
`CephReader` defaults: max_streams128, max_records100000, max_front1048576,
max_reconnect1048576, max_payload67108864, max_rounds16, max_reconnect_chains128.
Record count can explicitly rise to1000000. Active streams remain at most8.
`max_reconnect_chains` can be reduced to exercise row pressure. Chain rows are
admitted before allocation and retained for the reader lifetime, including final
series. There is no expiry or silent eviction. A completed series can reuse its
same identity row for a later series without freeing another peer's row.

The shared4 MiB **byte-buffer budget** includes pending TCP data, delivered span
bytes, retained overlap history, temporary history concatenation/copy, parser
queue/read scratch and required metadata storage. Fixed metadata storage reserves
two times its byte length before allocation. Pending/span and queue records also
carry256-byte admission charges; retained FIN anchors also carry256-byte charges.
Parser source-range records carry512-byte charges. This
is not a claim that CPython RSS, returned artifacts or caller-owned packets fit4 MiB.
All structural counts are separately bounded before allocation/iteration.
History's explicit bounded suffix is not proof of older overlaps; older overlap
fails rather than being accepted or evicted silently. Stream identities never evict.

Additional bounds:128 source domains,128 messenger peer rows,128 observed client
identities;128 retained reconnect-chain rows across all clients/servers;
4096 source ranges per messenger frame;1024 items per field container;
4096 aggregate counted items per decoded message, shared by nested cursors;
64 reclaim addresses;64 chunks and4096 caps/realms per reconnect series. Packet
buffers are at most65549 bytes. TCP sequence admission is within the direction
budget ahead of delivered bytes. No bound derives from the daemon16 KiB interface.

Finish receipts validate parser drain, source-coordinate consistency, zero supplied
loss/error counters and outstanding leases. They do **not** attest native counter
provenance, source fsync, installed binary/config hashes, boot identity, capture
start barriers or clock accuracy. P3 must retain those independently. A quiet
reader or consecutive `record` values cannot qualify native continuity.

## Fixture provenance and verification

`vectors.json` contains seven literal fronts and one literal full frame. None came
from a capture, authorizer, key or daemon invocation. The three original literals
spell a SESSION head, a binary UUID reclaim and an empty address-vector reply.
The four added literals use independently assigned packed offsets, not the test
builders or provider code: request head98/path suffix; caps head92/body84/v12 suffix;
reply head15 with inode1 and a nonempty8-byte-entry fragtree; reconnect4 with one
cap and one realm but **no** v5 more byte. The full frame assigns the53 header bytes
by offset and uses independent bit-stream polynomial division for its CRCs.
Whitespace inside hex is not a byte. Expected hashes use salt `bytes(range(32))`.
The known CRC literal `CRC32C("123456789",0)=0x58e3fa20` checks the convention.

The focused tests additionally construct independent synthetic packed layouts.
There is no provider encoder or round-trip-only acceptance. They test all required
front versions, every required suffix cut, framing fragmentation/coalescing,
wrap/reorder/conflicting and identical overlap, tuple reuse, reset/SEQ/gaps, metadata
privacy, full address unions, N4 ranges, covering ACKs and limits before allocation.
One parser-copy probe observes that front allocation does not occur on failed
admission. Tests install signal/process/socket guards **before** provider import.
The AST test forbids provider runtime `assert` and non-whitelisted imports.

Only these explicit modules are in the test scope:

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -m unittest test_g3_wire_tcp test_g3_wire_ceph
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O -m unittest test_g3_wire_tcp test_g3_wire_ceph
```

Actual results, the seven-file frozen manifest/archive and full input/mode manifests
for r2 are recorded in `current/g3-wire-ceph-corrections-r2.md` and its new exclusive
handoff directory. The r2 inventory replaces only the ledger path: six provider/
test/fixture paths plus that new ledger. Original `current/g3-wire-ceph.md`, the r1
manifest49f8f7e6/archive97ca85d4, completed review and all failed probes stay frozen
as history. Their78/earlier70/77 counts are not new r2 results. The union of both
seven-path inventories has eight paths; neither archive overwrites the other.
`vectors.json` and all original literal byte expectations remain unchanged in r2.
No broad discovery, Cargo, native examples, peer collection or live actions belong
to this package's evidence.

## Protocol references and exact pins

Local Ceph revision: `8db74a411f998b5e6a697cf541113b0f98c1bf9f`.
Local root: `/home/ubuntu/ceph`. Tag copies use
`https://raw.githubusercontent.com/ceph/ceph/v20.2.0/` plus the paths below.
These are source-format references, not installed package equivalence.

Symbols: `msgr.h::{ceph_msg_connect,ceph_msg_connect_reply,ceph_msg_header,ceph_msg_footer}`;
`ProtocolV1::{handle_connect_reply_2,handle_ack_seq,client_ready,open,handle_message_footer,session_reset}`;
`AsyncMessenger::get_global_seq` (review-added local source);
`Message::encode` and `decode_message` CRC paths;
`MClient*::{encode_payload,decode_payload}`;
`ceph_fs.cc::{encode,decode}(ceph_mds_request_head)`;
`CInode::encode_inodestat`, `Server::{handle_client_session,set_reply_extra_bl}`;
`Client::{make_request,insert_trace,insert_readdir_results,_encode_filelocks}`;
`mdstypes.cc::{feature_bitset_t,metric_spec_t,cap_reconnect_t,snaprealm_reconnect_t}::{encode,decode}`;
`MDSCapAuth::decode`, `MDSCapMatch::decode`, `optmetadata_multiton::decode`,
`fragtree_t::{encode,encode_nohead}`. `cephfs_features.h` fixes bit9/13; the
messenger feature header fixes bits6/23/59. Rust codecs corroborate the layouts.
No kernel implementation was translated or used as an encoder.

For WCEP-1, pinned v20.2.0 `Client::send_reconnect(MetaSession*)` sends all chunks
through one selected MDS session. `Server` handles that session's final chunk.
For WCEP-3, pinned v20.2.0 `ProtocolV1::handle_connect_reply_2` passes the server hint
to `get_global_seq`. The review-added local allocator raises its current counter
to the hint if needed, then increments it. Its declaration and return value are
u32. No C++ executable or native observation is part of this evidence.

Full SHA256 pins (`L` local, `T` v20.2.0):

| Path under Ceph | L | T |
|---|---|---|
| src/include/msgr.h | `7eb1d06765ef153f6480b0a28c410df7c56946e42ac98637139079b372134e23` | `2a0c37379a08a841f98ca339d720d830ecc50f4d984d2a1528ad10545af7fc0d` |
| src/include/ceph_fs.h | `275ea2f71ff553a19a47a96a7fdbc8fc0d63b2a5b19f05f4472141c053993b3f` | `5518acc49a18050f6c84d148a5fab5d57f0a33ef713ef99f20e37e0127f925cb` |
| src/include/ceph_features.h | `c20efb803c63d95bb83f7b94092aa5ec1f64d2a5fb7c7f1c2e394ffa31b244f0` | `c5a7c25ba0414b4c3b6eed012672b5cbda4972ea91056be048bfac64f349f93e` |
| src/include/filepath.h | `89255bce6c8f50040686bf9a12b63eaaf7d445402caaeea7e8f8fedb6e115de1` | `42875e8e0cf9a859bf1083e766b7453dd9ac10ab78faae7d324b7082d4507ce8` |
| src/include/frag.h | `ac32d19481349ecc73c418894586a0f68c7520c11b1e08fbdc8ccd2b59f6bb7d` | `fb27f4b3e12513eed0f86292d0035b03ebdbde5e5a77aef180097bf5e31e35b9` |
| src/include/cephfs/types.h | `e8a27167fc996ca2090f1bad563ca75c09b912946dff8d24861ef2d6969c8bd5` | `a8aa1ff2a810578472b26a08757ac5dfd44fb99dd3fbdcf27c346becc53b13ed` |
| src/common/ceph_fs.cc | `4298a1f559cf2c12b2cfe81b5635c2e7b4bad0fbe2d85229f3a2a0df69fbeee8` | `70dfc126c148a06e1a3373c29b11bd674d7f2b2a1b5132fc5dcabda104cd695a` |
| src/common/frag.cc | `5c828c3abaf3aa6fff9d4a92d30c164e3e6316f08a8b36464977ee53dd77f8d4` | Moved: tag has inline definitions in frag.h |
| src/msg/Message.h | `19fa97b7261b6f4728e2b99fa3f1e1c6e060a735098af6031a09f72413bffd58` | `81f9b84fc9f6b2bdd93aad232a441eae268ac337a1c8a2075b927b7a954a629a` |
| src/msg/Message.cc | `8368f9c1fe581a5ccb6e7e83a0e8ec535989b7dc64516a2f0966cdc56c4db741` | `9c778ea7185e1910fb6d97bbfdcfe446a52f4eac94ff9a89faf5a3cfcdf0f562` |
| src/msg/async/ProtocolV1.cc | `86e3867f034a73c54cc9c458a38ea3b651e32be9d2feb71ea2645a985d2b5e22` | `75f42529fc26c137fdb22d50edb8ba4c99ee3eff8d6092bd7328027a2f778d53` |
| src/msg/async/AsyncMessenger.cc | `47dff709e86e92676119504ee30774583e2f4d49155b84a89e22c74585918050` | Not used; review-pinned local allocator reference only |
| src/msg/async/AsyncMessenger.h | `7a3828732ec2e7dd4b1d5ce0f31a538280a3c4cb33264212125920e2a45a95f7` | Not used; review-pinned local u32 declaration only |
| src/messages/MClientRequest.h | `379eb20972203f24c25e7423e188f23df7d98010336042a25b11a24bd4c104f7` | `4acfb512dca787c6c13a945c40610732b4c4de8669609dd03cd7a53875c1c037` |
| src/messages/MClientReply.h | `d8336fcc6027b38d35541cf63ab27fcfd4e4428e163d3b745a83389df0e064af` | `949f55db71a47e185095c39f7681953a0e7d136feb37ad6c18addda24b1f764b` |
| src/messages/MClientCaps.h | `bd85fedafb0779a02ef0adc4454c5b35edcf448ff0613a9f5debe805a04b7177` | `f3d23e37262cc80d2676b26ba44f4a6a4c3a51fd45da2cf137cdbe6360a445f9` |
| src/messages/MClientSession.h | `8b8b7f97bfbf800c0256df02cdb1affbc82dd05f3820203ae286ca896a91c664` | `08ef5164778bea71d8907bf93e011dc5ce07b0f00258c79a84f0ae48a5c8883c` |
| src/messages/MClientReconnect.h | `659d44c5b890e18d20690b36af6e5a3b92358229aa2f04151e75bbedd72bf12f` | `0b050d72e98e53784c89cfbea5d5417e724bee943e0e04df677d0cd4cac5706b` |
| src/messages/MClientReclaim.h | `1e6e920b128b9b9e1a2967611072d18ab28ee1b4d4670292705c7c1137115268` | `1d6d429b8614b015372b7a3eae81cf893efa0cfd045b8e5ab63ad1ba8336df66` |
| src/messages/MClientReclaimReply.h | `91c32c3651e3b41123bbf32e6dd2f88097e1cb9df9f51dceb083137f02ee79c2` | `5ec97a2eaad2a86826efa4f7cb289ae20c0f598ac4f81284fab6779b68378ea5` |
| src/mds/mdstypes.cc | `2cd6085609e919979a3d0bd5c1727fe320fa3011f96b96cae17b184d92992baa` | `baa9c9dc61b1f0edead104e4839cbe1a6d1ed7a87e8216931cef1a357e1d5c75` |
| src/mds/MDSAuthCaps.h | `09e482309026c840847a8c7d0d35191ae115be5460e566a4ad27cf66c395c59a` | `862284afc4b922b4c319ca943b741083a9254b37bae70918bdb7d1bf03f7e2fe` |
| src/mds/cephfs_features.h | `2596a2c882f5f462ea96e4ac70946055a75fed81157ece33c57670244884e1fb` | `18f3cc7ecdbbeddd11e8ed1af7aaee6249a3e9da284b3832c8586454a74a248d` |
| src/mds/CInode.cc | `62edd67500c701c76a81281b2c5a3a5c8712f1bc4880a67da692d8f341366748` | `5807c3e47fde44c6b541a75efe061879106301c13c5b07a91242e3651f95e1db` |
| src/mds/Server.cc | `285d32219964953c6fc58ba9456142466f9a0bbd3eb10a63da24603ce33653a1` | `03c57c40a37f1420600ad5e64aad6732f885775eedebd78dcaa65f837c45ff01` |
| src/mds/Locker.cc | `2edd49ec8e570a7115d4cc5988e1361b39b410a8838de3dfeb6af875691502e9` | `3116af9460af22be0774c0c6b7a91365597c2547bfeb387849ae1a213e0d8cac` |
| src/client/Client.cc | `a2f6c89e026ef4cb29c890d4609871034fa1ac2a4cd1e9ed232db44c146ed42b` | `f1f94640de66ed673f63576aeb671b7b5dabbdb63b7758fd7a812645a77fa5bf` |

Rust read-only source pins:

| Repository path | SHA256 |
|---|---|
| crates/proto/src/request.rs | `f664a125f7a525ac2a78e012d430c06aacf67e621608f086824e6739a16cc53a` |
| crates/proto/src/reply.rs | `30f0bd0f51f75ff1cf7fe5e70fc5508d134305887f8e490bdcee3af1ab01c18b` |
| crates/proto/src/caps.rs | `3325f58667f4c6073051f2f09f96aa33c19f57fa6c1aa433a995421c16a9d9b4` |
| crates/proto/src/session.rs | `4b659d5ee6082c58d672ead95418feb7b7c1919debb0aab19768c9e32004101c` |
| crates/proto/src/reconnect.rs | `ef16080a942c2c44c28340a155e8b43849b7c65a5c9c1692472225cd5622e0e6` |
| crates/proto/src/reclaim.rs | `0e58b0b6f9b45fe3f86ecff21b60065e2a234eea5605cb5841b79db556276d45` |
| crates/proto/src/types.rs | `25dae102a32450e44d9df493528d461d8c0d216e032e2970b54fbdd77e644324` |
| crates/proto/src/features.rs | `0f5cbfcf098f5e6629135597e5396fe09e15aa5a0083fa502eb95144f27d4350` |
| crates/proto/src/frag.rs | `466a3ccf01d2581a220c33c1d885d1440c842f209616fe1b8e8ca7a18ef8e533` |
| crates/denc/src/addr.rs | `f55519dc209a524b505d409d1fd4d793ca218c1868842c22b3d31b3fe591d51e` |

The tag's normal InodeStat/CAPS versions are8/12; local main and this repository
also define9/14. Both are explicit supported schemas, not inferred installed-peer
emission. Tag `src/common/frag.cc` returned404; its pinned `src/include/frag.h`
contains the equivalent inline map encoder. The machine-readable audited pin map
records that absence, rather than inventing a tag hash. The original smaller pin
map remains retained. No raw capture, key or auth-handshake artifact was read.
