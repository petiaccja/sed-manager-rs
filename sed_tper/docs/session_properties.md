# Session communication properties plan

This is a plan to track the communication properties (the values exchanged by
the `Properties` method) soundly in the protocol stack, and to expose the
properties in effect for each session through `tper::Session`. The immediate
motivation is the MBR upload: `Set` calls on the MBR table have to be split
into chunks that fit the device's limits. The same is true for `Get` (reading
the MBR back for verification).

The plan is not final. Open decisions are listed at the end.

## Spec background

Section numbers refer to [Core_v2.01.md](../../docs/specification/Core_v2.01.md)
unless noted otherwise.

### When negotiated properties take effect

- Properties are kept per ComID. HostProperties "apply to any sessions started
  using the ComID ... once the TPer has processed the method" (5.2.2.1).
- "Submitted values ... SHALL only apply to sessions started after the
  submission of those values, and not to sessions that are already open on
  that ComID" (5.2.2.3). A session's properties are therefore fixed when it
  starts, so each session needs its own snapshot.
- "It is the host's responsibility to insure that Properties method
  invocations have processed prior to invocation of any session startup
  methods that rely on those invocations" (5.2.2.3).
- A stack reset resets the ComID's communication properties to their defaults
  (3.3.4.7.5, item 7).
- On static ComIDs, a TCG hardware reset or power cycle can reset the TPer's
  knowledge of the HostProperties without the host noticing. The host is
  advised to "invoke Properties with the HostProperties parameter prior to
  each invocation of StartSession on statically allocated ComIDs" (5.2.2.4,
  informative). Suspend/resume does this to a laptop's drive.
- "If a particular property is not returned by the TPer, then the host SHALL
  NOT change its assumption about the TPer's capabilities related to that
  property" (5.2.2.4). The TPer's `Properties` describe its own receive
  capabilities, so they don't depend on whether the TPer echoed the
  HostProperties back.

### `Properties` and `StartSession` in the same IF-SEND

The spec gives no way to tell whether a `StartSession` sent together with a
`Properties` call got the old or the new properties:

- Responses on the control session may come back in any order: "the possible
  different ordering of responses to Session Manager layer methods" (5.2.2.1.2,
  5.2.1). Nothing says the TPer processes control-session methods in the order
  they were sent.
- HostProperties apply "once the TPer has processed the method and prepared a
  response" (5.2.2.1). `SyncSession` has no field that reports which
  properties the session uses (only `TransTimeout` and `InitialCredit`).
- The spec makes it the host's job instead: "It is the host's responsibility
  to insure that Properties method invocations have processed prior to
  invocation of any session startup methods that rely on those invocations"
  (5.2.2.3). The only point at which the host *knows* the method has been
  processed is when it receives the response. So `StartSession` must go in a
  later IF-SEND than the `Properties` response it relies on.

Batching is also limited in practice:

- The initial assumptions are MaxMethods = MaxSubpackets = MaxPackets = 1, so
  the first `Properties` call always has to be sent alone. Batching could only
  happen on a re-sync.
- If the control session stays on the initial assumptions (see below), its
  MaxMethods stays 1 and batching never happens. Too many methods in a control
  session subpacket "MAY" be ignored by the TPer (5.2.2.4.1.7).

When a `Properties` call times out, the host can't tell whether the TPer
processed it, so either set of host properties may be in effect. The receive
direction is safe either way, because the TPer never uses more than our
advertised capabilities, and we can always receive those. Limits *derived*
from the receive direction (`max_get_bytes_len`) must assume the smaller of
the two possibilities, i.e. the previous values.

### When to take a session's snapshot

The latest point at which the snapshot is still correct is the IF-SEND of the
`StartSession`, using only the `Properties` responses received before it.
This holds with async and any reordering, because it relies only on what the
host has seen:

- **Response received before the `StartSession` IF-SEND:** the TPer
  processed the `Properties` call before it even received the
  `StartSession`, so it definitely applies.
- **Response received after that IF-SEND, or never (timeout):** unknown. The
  TPer may have processed it before or after the `StartSession`.
- **`Properties` sent after the `SyncSession` arrived:** definitely doesn't
  apply.

Taking the snapshot earlier (e.g. when packetizing) is correct but can be
stale. Taking it later (e.g. when the `SyncSession` arrives) is wrong.

What a `Properties` call still in flight at the IF-SEND means depends on the
kind of property:

- **Limits:** the uncertainty can be absorbed by taking the min of the
  confirmed and the in-flight values.
- **Functional capabilities** (Asynchronous, AckNak, SequenceNumbers,
  ContinuedTokens): the session can't run with a mode that is "possibly on".
  If an in-flight call could change one of these, no `StartSession` may be
  sent until it completes.

### Initial assumptions depend on the SSC

- Table 168 (Core) gives the initial assumptions: MaxComPacketSize 1024,
  MaxPacketSize 1004, MaxIndTokenSize 968, and 1 method/subpacket/packet.
- "SSCs MAY redefine the initial assumptions ... the initial assumptions the
  host makes about the TPer are based on the supported SSC the host discovers
  during Level 0 Discovery" (5.2.2.4).
- The first `Properties` call "SHALL be encoded using the minimum assumptions
  outlined above in Table 168, or the minimum assumptions defined by the
  TPer's SSC" (5.2.2.4), so the SSC's values may be used from the start.

The SSCs fall into three groups:

| SSC (Level 0 feature code)                                                                                             | MaxComPacketSize | MaxPacketSize | MaxIndTokenSize | MaxPackets / MaxSubpackets / MaxMethods | MaxResponseComPacketSize (TPer reports) | Other Table 168 properties |
| ---------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------- | --------------- | --------------------------------------- | --------------------------------------- | -------------------------- |
| No recognized SSC: Core Table 168                                                                                      | 1024             | 1004          | 968             | 1 / 1 / 1                               | not listed                              | see below                  |
| Opal 1.00 (0x0200), Opal 2 (0x0203), Opalite (0x0301), Pyrite 1 (0x0302), Pyrite 2 (0x0303), Ruby (0x0304), KPIO (0x0305) | **2048**         | **2028**      | **1992**        | 1 / 1 / 1                               | ≥ 2048                                  | Core Table 168             |
| Enterprise (0x0100)                                                                                                    | 1024             | 1004          | **256**         | 1 / 1 / 1 (Core Table 168)              | ≥ 1024                                  | Core Table 168             |

The other Table 168 properties are MaxAggTokenSize 968, and ContinuedTokens,
SequenceNumbers, AckNak, and Asynchronous all `false`. None of the SSCs
redefine them.

Sources: Opal 1.00 4.1.1.1, Opal 2.30 4.1.1.1 (Table 17), Opalite, Pyrite
1.0, Pyrite 2.01, Ruby, and KPIO all have the same properties table (KPIO adds
KMIP-specific rows). Enterprise 9.2.2.1 lists the values in its `Properties`
method description.

Notes:

- **Directions in the Opal family.** "Initial Assumption" appears only in the
  "Host Property ... Values Accepted" column, i.e. the TPer's assumption
  about the host (our inbound direction). The "TPer Property ... Values
  Reported" column gives the same numbers as guaranteed minimums. Using
  those as the host's assumption about the TPer (our outbound direction) is
  safe, because the TPer SHALL support at least that much.
- **Directions in Enterprise.** 9.2.2.1 states the MaxPacketSize,
  MaxComPacketSize, and MaxIndTokenSize values "also apply to communications
  from the TPer to the Host", so they cover both directions.
- **Enterprise's Table 14 contradicts 9.2.2.1.** It says MaxPacketSize and
  MaxComPacketSize "SHALL be at least 512". We go with the 1024-based
  values from 9.2.2.1 (which match Core): 512 bytes is too small to be
  practical, likely even for the `Properties` call itself, and devices
  limited to less than 1024 bytes aren't expected to be around anymore.
  Such devices will fail with SEDManager.
- **Enterprise's token size is lower than Core's.** MaxIndTokenSize ≥ 256
  is *lower* than Core's 968, so `Properties::INITIAL` is not a safe
  assumption for token size on Enterprise. In practice the management
  traffic has no tokens that large, but a regular session must not fall back
  to `INITIAL` on Enterprise if the properties sync fails.
- **MaxAggTokenSize** stays at Core's 968 even where MaxIndTokenSize is
  larger. This is harmless: ContinuedTokens is initially `false`, and then
  MaxAggTokenSize is ignored (5.2.2.4.1.8).

### The control (management) session

- The control session (TSN = HSN = 0) exists as soon as the ComID is issued
  and lives as long as the ComID. It is never "started" (3.3.7.1.2).
- Its flow control "is performed in the same manner as the flow control for
  Regular Sessions" (3.3.7.1.2). Under buffer management, "credit subpackets
  SHALL be exchanged after ComID acquisition ... This credit only applies to
  the control session for that ComID" (3.3.8.2).
- "The transmission timeout in effect for control sessions is the
  transmission timeout reported in the Properties method" (3.3.9.4).
- Sequence numbers start at 1 "since the start of the session" (3.3.9.2),
  and ACK/NAK is enabled "via invocation of the Properties method". The spec
  doesn't say how the control session switches into numbered or ACKed mode
  partway through, when the `Properties` call itself was sent without it.
- Section 5.2.2.4 says "All communication from that point can use the
  mutually discovered capabilities", while 5.2.2.3 limits submitted values to
  "sessions started after". The two are hard to reconcile for the control
  session.
- Buffer management has **no** entry in the Properties table, only a Level 0
  Discovery bit (Opal: vendor-unique) and `InitialCredit` in
  `StartSession`/`SyncSession`. Nothing defines how the host announces
  support on the control session.
- Asynchronous mode is ComID-wide by nature, because it changes the rules for
  interleaving IF-SEND and IF-RECV (3.3.10). It cannot apply to some sessions
  on a ComID and not others. It also forces MaxMethods = 0 (Table 167).

**Conclusion:** the only interpretation that is clearly safe is to keep the
control session on the initial *features* (SSC-adjusted): no sequence
numbers, ACK/NAK, credits, or async, regardless of the negotiated
properties. Both sides are required to support that baseline, and the spec
doesn't define how the control session would switch modes. The negotiated
*limits* are fine to use on the control session: they only bound sizes, and
the TPer's `Properties` state what it can receive on the ComID, so staying
within them is safe whichever session the data is for. Control session
traffic is small (`Properties`, `StartSession` with at most a 32-byte
password, `SyncSession`, `CloseSession`). `StartTrustedSession` with
certificate chains could get large, but it isn't implemented. If async or
ACK/NAK are implemented one day, they should be treated as per-ComID modes,
negotiated before any regular session starts, and probably only on
dynamically allocated ComIDs, where the reset ambiguity above doesn't exist.

### Method size limits across IF-SEND/IF-RECV

**Host to TPer.** Under the synchronous protocol, "Methods SHALL NOT span
ComPackets" (3.3.10.3). An interface command carries exactly one ComPacket
(3.2.3.1). One method call must therefore fit in one IF-SEND:

- payload ≤ MaxComPacketSize − ComPacket header (20);
- packet ≤ MaxPacketSize, so payload ≤ MaxPacketSize − packet header (24) −
  sub-packet header (12), after rounding the sub-packet up to a multiple of 4
  (3.2.3.4.1.2.2);
- every single token ≤ MaxIndTokenSize. We don't support continued tokens,
  so MaxAggTokenSize doesn't come into play.

The TPer aborts an IF-SEND longer than its MaxComPacketSize at the interface
level (3.3.10.7, item 3; Opal 3.3.1). Tokens may span packets in general
(3.2.3.1), but that only matters for async mode or continued tokens.

**TPer to host.** Opal 3.3.1, Pyrite, and Enterprise all require that the
response to each method, including protocol overhead, fits entirely within
the response buffer. Otherwise the TPer returns an empty result with
RESPONSE_OVERFLOW and no partial data (see also 3.3.10.7, item 4). Multiple
IF-RECVs for one IF-SEND only happen when:

- the TPer is still processing (Length = 0, OutstandingData = 1);
- the host asked for too few bytes (retry with MinTransfer);
- one IF-SEND produced several method responses (S3 → S2 → S3, 3.3.10.6).

A single method response is never assembled from several IF-RECVs. The
effective limit for a response is the smaller of the TPer's
MaxResponseComPacketSize and what the TPer believes the host can receive (the
echoed HostProperties, or the initial assumptions if none were echoed), minus
the overhead.

This means `Get` on a byte table needs its own chunk limit, derived from the
**receive** direction. Reading the MBR back with an oversized `Get` fails
with RESPONSE_OVERFLOW instead of returning less data. The receive limit is
also not the same as the limit for `Set`.

## Problems in the current implementation

1. **Power cycles on static ComIDs** aren't handled (see 5.2.2.4 above). This
   is deferred for now.
2. **A timed-out `Properties` call releases the `StartSession`s.**
   `Management::poll_action` never has a `Properties` and a `StartSession` in
   flight at the same time, so the snapshot taken at the `StartSession`'s
   IF-SEND is always exact, except when the `Properties` call times out. The
   TPer may still process it, but the `StartSession`s are released and get
   the confirmed values. That is harmless today, because:
   - only limits are negotiated;
   - the host always sends the same HostProperties;
   - the previous values are the smaller ones, which is the safe side.

   It becomes a real problem once a functional capability is negotiated.
3. **The size check is incomplete.** `max_method_len()` covers the
   sub-packet padding, `MaxPacketSize`, and `MaxComPacketSize`, and is used
   by `protocol::Session::handle_method_call` and the control session. Still
   missing:
   - `MaxIndTokenSize` is not checked anywhere. It is the binding limit for
     large `Set` calls when the TPer's token limit is below its packet
     limit, which is always the case on Enterprise (256).
   - The inbound limit (`properties().inbound.max_method_len()`) ignores the
     TPer's `MaxResponseComPacketSize`. It's only stored in
     `outbound.max_gross_compacket_response_size` and never folded into the
     inbound direction.

## Plan

### Step 2: Re-sync before `StartSession` (deferred)

The snapshot is taken at the `StartSession`'s IF-SEND
(`Management::handle_iface_send_done`) and carried through
`StartSessionReceivingRecord` to `ManagementAction::Spawn` and to
`tper::Session`. `Properties` and `StartSession` are never in flight at the
same time, and a requested `Properties` takes priority over enqueued
`StartSession`s. The control session uses the negotiated outbound limits for
its size check. What's left:

- When no sync has completed since the last reset or the last
  `StartSession`, push a `Properties` request before releasing the
  `StartSession`. This fixes problem 1 at the cost of one extra round trip
  per session start. `StartSession`s queued back to back may share one sync.
  Never put `Properties` and `StartSession` in the same IF-SEND (see
  "`Properties` and `StartSession` in the same IF-SEND" above).
- Once any functional capability can be negotiated, a `Properties` call that
  timed out must keep blocking `StartSession`s (e.g. until a stack reset
  makes the state known again), because a min can't resolve a functional
  capability (problem 2).

### Step 4: Complete the limits

The limits are computed per direction by `Properties::max_method_len()` on
`SessionProperties::outbound` and `inbound`, instead of a separate
`SessionLimits` type. What's left (problem 3):

- `max_token_len()` = the TPer's `MaxIndTokenSize`, enforced for outbound
  method calls.
- Include the TPer's `MaxResponseComPacketSize` in the inbound limit:
  `min(TPer MaxResponseComPacketSize, echoed host MaxComPacketSize) − 20 −
  24 − 12`, e.g. by folding it into `inbound` in
  `Management::handle_properties`.

### Step 5: Chunk limits on `tper::Session`

`tper::Session::properties()` already returns the two-direction
`SessionProperties` snapshot. What's left:

- Add `max_set_bytes_len()` and `max_get_bytes_len()`.
  - Work out the encoding overhead by tokenizing the call or response with
    worst-case integers (`where_ = u64::MAX`) and an empty byte atom, plus
    the difference between a short and a long atom header. The
    `max_set_payload` and `max_get_payload` helpers in
    `sed_tper/tests/integration/tper/session.rs` do a simpler version of
    this (`where_ = 0`, no token limit).
  - For `Set`: `min(max_method_len − overhead, max_token_len −
    long_atom_header)`.
  - For `Get`: the same, using the inbound limits and the response encoding.

  Deriving the overhead from the real serialization avoids hard-coding token
  sizes.
- Keep `set_bytes`/`get_bytes` as a single RPC each. The chunking loop lives
  in `sed_manager` (`write_mbr`, and a read-back loop for verification).

### Step 7: Tests and virtual device

- `Management` unit test: a `StartSession` is preceded by a `Properties`
  sync (step 2).
- `protocol::Session`: a test at the token limit (step 4).
- Virtual device:
  - reject oversized tokens, as the spec requires;
  - exercise a small limit (e.g. 2 KiB) to get many chunks quickly. The
    capabilities can be set through `VirtualDevice::new`, but no test does
    it yet, and `VirtualDeviceBuilder` has no public constructor.
- Integration test against the virtual device: `get_bytes` of one byte more
  than `max_get_bytes_len()` returns `MethodTooLarge` without touching the
  device. Today the oversized request reaches the device, which answers with
  RESPONSE_OVERFLOW.

## Open decisions

- **Sync before every `StartSession`, or only after a reset?** Recommended:
  every time. The spec advises it for static ComIDs, and it covers
  suspend/resume.
