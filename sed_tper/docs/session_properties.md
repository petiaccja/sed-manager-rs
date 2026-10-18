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

### Initial assumptions depend on the SSC

- Table 168 (Core) gives the initial assumptions: MaxComPacketSize 1024,
  MaxPacketSize 1004, MaxIndTokenSize 968, and 1 method/subpacket/packet.
- "SSCs MAY redefine the initial assumptions ... the initial assumptions the
  host makes about the TPer are based on the supported SSC the host discovers
  during Level 0 Discovery" (5.2.2.4).
- Opal 2 (4.1.1.1, Table 17) and Pyrite 2 raise them to MaxComPacketSize
  2048, MaxPacketSize 2028, and MaxIndTokenSize 1992.
- Enterprise (9.2.2.1) requires only **MaxIndTokenSize ≥ 256**, which is
  *lower* than Core's 968. `Properties::INITIAL` is therefore not a safe
  assumption for token size on Enterprise. In practice the management traffic
  has no tokens that large, but a regular session must not fall back to
  `INITIAL` on Enterprise if the properties sync fails.

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
control session on the initial assumptions (SSC-adjusted) with no sequence
numbers, ACK/NAK, credits, or async, regardless of the negotiated
properties. Both sides are required to support that baseline. Control session
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

1. **The session snapshot is taken at the wrong time.**
   `Management::handle_sync_session` takes `self.properties` when the
   `SyncSession` *arrives* (see the TODO in `management.rs`). It should use
   the properties in effect when the `StartSession` was *sent*.
2. **`Properties` is not explicitly ordered before `StartSession`.** It
   currently works only because the synchronous protocol finishes one
   IF-SEND/IF-RECV cycle before starting the next. If the `Properties`
   response times out (`PropertiesSync::Receiving` → `Idle`), a
   `StartSession` can go out while the TPer's view is unknown.
3. **Internal recovery doesn't re-sync.** `Management::handle_reset` drops
   back to `INITIAL`, but only `Tper::stack_reset` calls `sync_properties()`.
   The automatic recovery path (`Action::Recover` in `protocol_state.rs`)
   stays on the initial properties until the user resets manually.
4. **Power cycles on static ComIDs** aren't handled (see 5.2.2.4 above).
5. **The size check is incomplete.** `protocol::Session::handle_method_call`
   only checks `max_gross_packet_size − 24 − 12`. It ignores the sub-packet
   padding, `max_gross_compacket_size − 20`, and `max_ind_token_size`. The
   last one is the binding limit for large `Set` calls.
6. **`PropertiesChanged` events are dropped.** The broadcast channel has
   capacity 1 and overflow is disabled. Every `Controller` clone holds an
   *active* `Receiver` that is never read. After the first broadcast the
   channel stays full, and later `try_broadcast` calls fail silently.
7. **The two directions are mixed.** `Properties::common(capabilities,
   tper_properties)` merges both directions into one struct. Sending is
   limited by the TPer's `Properties` (what it can receive). Receiving is
   limited by the TPer's MaxResponseComPacketSize and by the HostProperties
   the TPer echoed. Problem 5 and the `Get` limit both need the directions
   kept separate.
8. **The initial assumptions ignore the SSC** (see above).
   `Properties::INITIAL` follows Core Table 168. That is too pessimistic for
   Opal and Pyrite, and too optimistic about token size for Enterprise.

## Plan

### Step 1: Model the negotiated state explicitly in `Management`

- Replace `properties: Properties` with a struct that holds the two
  directions separately:
  - `tper_properties`: the TPer's receive capabilities from the `Properties`
    response. This bounds what we send.
  - `host_properties_echoed`: the HostProperties the TPer echoed. This
    bounds what the TPer sends us, together with its
    MaxResponseComPacketSize.
- Both start from the initial assumptions. Following 5.2.2.4, a property
  the TPer didn't return keeps its assumed value.
- Fix problem 6. Either enable `set_overflow(true)`, or remove the active
  receiver from `Controller`: keep only the `InactiveReceiver` and call
  `activate_cloned()` on demand in `properties_changed()`. I prefer the
  second, because then only receivers that actually listen are attached to
  the channel.

### Step 2: Order `Properties` before every `StartSession`, and snapshot at send time

- In `poll_method_calls`, when the next call is a `StartSession` and no sync
  has completed since the last reset or the last `StartSession`:
  - push a `Properties` request;
  - **hold** the `StartSession` until the sync finishes, whether it succeeds
    or times out.

  This fixes problems 2 and 4 at the cost of one extra round trip per
  session start. `StartSession`s queued back to back may share one sync.
  Never put `Properties` and `StartSession` in the same IF-SEND (see
  "`Properties` and `StartSession` in the same IF-SEND" above).
- If the sync timed out, the snapshot is the minimum of the previous and the
  requested values, because the TPer may or may not have processed the call.
- When a `StartSession` is released, store a snapshot of the current state
  in its `MethodSendingRecord`. Carry it through `MethodReceivingRecord` and
  use it in `StackAction::Spawn`. This fixes problem 1, and the TODO can be
  removed.
- Leave the control session's own size check on the initial assumptions, as
  it is now (see "The control (management) session" above).

### Step 3: Re-sync after any successful stack reset

- Request the `Properties` sync from `Management::handle_reset` itself, or
  from `ProtocolState` when it sees a successful `StackReset` response. Then
  remove the explicit call in `Tper::stack_reset`. Recovery and manual reset
  then take the same path, which fixes problem 3. With step 2 in place this
  is mostly belt-and-braces, but it keeps `PropertiesChanged` accurate for
  the GUI.

### Step 4: Compute the limits in one place

- Add a type in `sed_tper` (e.g. `SessionLimits`) computed from the
  snapshot:
  - `max_method_len()` = `min(MaxPacketSize − 24, MaxComPacketSize − 20 −
    24) − 12`, rounded down to a multiple of 4, using the TPer's
    `Properties`;
  - `max_token_len()` = the TPer's `MaxIndTokenSize`;
  - `max_response_len()` = `min(TPer MaxResponseComPacketSize, echoed host
    MaxComPacketSize) − 20 − 24 − 12`, plus the corresponding packet and
    token limits from the echoed HostProperties.
- Use it for the check in `protocol::Session::handle_method_call` (problem
  5).

### Step 5: Hand the snapshot to `tper::Session`

The snapshot never changes during a session, so it is handed over once, when
the session starts. There's nothing to keep in sync.

- Add `Controller::start_session(call)` returning `Result<(Vec<u8>,
  SessionLimits or Properties), Error>`. `StartSession` then stops going
  through the generic `call` channel. `Management` already special-cases
  `StartSession`, so only the type of the reply channel changes.
- `tper::Session` gets a field for the snapshot and `pub fn properties()`
  and/or `pub fn limits()`. `Session::start` uses the new controller call.
  `from_started` gets an extra parameter; callers without a better value
  pass the initial assumptions.
- Add `max_set_bytes_len()` and `max_get_bytes_len()`.
  - Work out the encoding overhead by tokenizing the call or response with
    worst-case integers (`where_ = u64::MAX`) and an empty byte atom, plus
    the difference between a short and a long atom header.
  - For `Set`: `min(max_method_len − overhead, max_token_len −
    long_atom_header)`.
  - For `Get`: the same, using `max_response_len` and the response encoding.

  Deriving the overhead from the real serialization avoids hard-coding token
  sizes.
- Keep `set_bytes`/`get_bytes` as a single RPC each. The chunking loop lives
  in `sed_manager` (`write_mbr`, and a read-back loop for verification).

### Step 6: SSC-aware initial assumptions

- Make the initial assumptions a function of the SSC found in discovery:
  Core Table 168 by default, raised for Opal/Pyrite, and with the Enterprise
  token minimum lowered. `Tper::connect` doesn't know the SSC today, so this
  either takes the discovery as a parameter or keeps Core's values and lowers
  the token size to 256 everywhere. It is low priority as long as the sync
  normally succeeds, but it matters for the Enterprise fallback.

### Step 7: Tests and virtual device

- `Management` unit tests:
  - a `StartSession` queued before a sync is held until the sync completes;
  - the snapshot reflects the properties at send time, not at `SyncSession`
    time (inject a second `Properties` response in between);
  - a reset triggers a re-sync;
  - two consecutive `PropertiesChanged` events both arrive (regression test
    for problem 6).
- `protocol::Session`: a test at the exact boundary covering padding, the
  ComPacket limit, and the token limit.
- Virtual device:
  - reject IF-SEND above its MaxComPacketSize (interface error) and
    oversized packets/tokens, as the spec requires;
  - return RESPONSE_OVERFLOW for responses that don't fit;
  - make its capabilities configurable (64 KiB today) so a small limit
    (e.g. 2 KiB) exercises many chunks quickly.
- Integration test against the virtual device: `set_bytes` of exactly
  `max_set_bytes_len()` succeeds, one byte more returns `MethodTooLarge`
  without touching the device, and the same for `get_bytes` /
  `max_get_bytes_len()`.

## Open decisions

- **Sync before every `StartSession`, or only after a reset?** Recommended:
  every time. The spec advises it for static ComIDs, and it covers
  suspend/resume.
- **The TPer omits the HostProperties echo.** Per 5.2.2.4, using the TPer's
  own `Properties` for the send direction is spec-correct, and the receive
  direction stays on the initial assumptions. The current code is more
  conservative and keeps both directions on the initial assumptions.
  Recommended: follow the spec, because the directions are separate after
  step 1.
- **Fix for problem 6:** overflow mode, or an on-demand activated receiver
  (preferred).
- **Step 6 scope:** a full SSC-aware table, or just lower the token size
  assumption to cover Enterprise.
