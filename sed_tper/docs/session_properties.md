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

1. **In-flight `Properties` calls are not accounted for.** The snapshot is
   taken in `Management::handle_iface_send_done`, i.e. at the
   `StartSession`'s IF-SEND (see "When to take a session's snapshot"). It
   contains only confirmed values. Under the synchronous protocol, a
   `Properties` response always arrives before the next IF-SEND. So the only
   uncertain case is a `Properties` call that timed out and whose response
   arrives late, or never. That is harmless today, because:
   - only limits are negotiated;
   - the host always sends the same HostProperties;
   - the previous values are the smaller ones, which is the safe side.

   It becomes a real problem once async (several IF-SENDs outstanding) or a
   functional capability is negotiated.
2. ~~**Internal recovery doesn't re-sync.**~~ `Management::handle_reset` drops
   back to `INITIAL`, but only `Tper::stack_reset` calls `sync_properties()`.
   The automatic recovery path (`Action::Recover` in `protocol_state.rs`)
   stays on the initial properties until the user resets manually.
3. **Power cycles on static ComIDs** aren't handled (see 5.2.2.4 above).
4. ~~**The size check is incomplete.**~~ `protocol::Session::handle_method_call`
   only checks `max_gross_packet_size − 24 − 12`. It ignores the sub-packet
   padding, `max_gross_compacket_size − 20`, and `max_ind_token_size`. The
   last one is the binding limit for large `Set` calls.
5. ~~**`PropertiesChanged` events are dropped.**~~ The broadcast channel has
   capacity 1 and overflow is disabled. Every `Controller` clone holds an
   *active* `Receiver` that is never read. After the first broadcast the
   channel stays full, and later `try_broadcast` calls fail silently.
6. ~~**The two directions are mixed.**~~ `Properties::common(capabilities,
   tper_properties)` merges both directions into one struct, and that is
   also what `tper::Session::properties()` returns now. Sending is
   limited by the TPer's `Properties` (what it can receive). Receiving is
   limited by the TPer's MaxResponseComPacketSize and by the HostProperties
   the TPer echoed. Problem 4 and the `Get` limit both need the directions
   kept separate. `Properties::common` should be renamed and cleaned up to
   reflect that the two kinds of properties combine differently:
   - **Limits** (sizes, counts) are per direction. Each direction can use
     whatever its receiver supports, so no min across both sides is needed.
   - **Functional capabilities** (Asynchronous, AckNak, SequenceNumbers,
     ContinuedTokens) must be the lowest common denominator of host and
     TPer, because both sides have to run the same protocol.
7. **The initial assumptions ignore the SSC** (see above).
   `Properties::INITIAL` follows Core Table 168. That is too pessimistic for
   Opal and Pyrite, and too optimistic about token size for Enterprise.

## Plan

### ✓ ~~Step 1~~: Model the negotiated state explicitly in `Management`

- Replace `properties: Properties` with a struct that holds the two
  directions separately:
  - `tper_properties`: the TPer's receive capabilities from the `Properties`
    response. This bounds what we send.
  - `host_properties_echoed`: the HostProperties the TPer echoed. This
    bounds what the TPer sends us, together with its
    MaxResponseComPacketSize.
- Both start from the initial assumptions. Following 5.2.2.4, a property
  the TPer didn't return keeps its assumed value.
- Fix problem 5. Either enable `set_overflow(true)`, or remove the active
  receiver from `Controller`: keep only the `InactiveReceiver` and call
  `activate_cloned()` on demand in `properties_changed()`. I prefer the
  second, because then only receivers that actually listen are attached to
  the channel.

### Step 2: Re-sync before `StartSession`, and account for in-flight calls

The snapshot is taken at the `StartSession`'s IF-SEND
(`Management::handle_iface_send_done`) and carried through
`StartSessionReceivingRecord` to `StackAction::Spawn` and to `tper::Session`.
Under the synchronous protocol, with only limits negotiated, this is already
correct without a gate. What's left:

- When no sync has completed since the last reset or the last
  `StartSession`, push a `Properties` request before releasing the
  `StartSession`. This fixes problem 3 at the cost of one extra round trip
  per session start. `StartSession`s queued back to back may share one sync.
  Never put `Properties` and `StartSession` in the same IF-SEND (see
  "`Properties` and `StartSession` in the same IF-SEND" above).
- When a `Properties` call is in flight at the `StartSession`'s IF-SEND
  (e.g. one that timed out but may still be processed), the snapshot's
  limits are the min of the confirmed and the requested values. This fixes
  problem 1 for limits.
- Once async or any functional capability can be negotiated, **hold** the
  `StartSession` in `poll_start_session_calls` while `properties_call` is not
  `Idle`. A min can't resolve a functional capability.
- The control session's own size check uses the negotiated outbound limits,
  but the control session keeps the initial features (see "The control
  (management) session" above).

### ✓ ~~Step 3~~: Re-sync after any successful stack reset

- Request the `Properties` sync from `Management::handle_reset` itself, or
  from `ProtocolState` when it sees a successful `StackReset` response. Then
  remove the explicit call in `Tper::stack_reset`. Recovery and manual reset
  then take the same path, which fixes problem 2. With step 2 in place this
  is mostly belt-and-braces, but it keeps `PropertiesChanged` accurate for
  the GUI.

### ✓ ~~Step 4~~: Compute the limits in one place

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
  4).

### Step 5: Chunk limits on `tper::Session`

`tper::Session` already receives its snapshot from
`Controller::call_start_session` and exposes it through `properties()`.
What's left:

- Once step 1 separates the directions, change the snapshot (and
  `properties()`) to the two-direction type, or add `limits()` returning the
  `SessionLimits` from step 4.
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
  - a `StartSession` is preceded by a `Properties` sync (step 2);
  - a reset triggers a re-sync;
  - two consecutive `PropertiesChanged` events both arrive (regression test
    for problem 5).
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
- **Fix for problem 5:** overflow mode, or an on-demand activated receiver
  (preferred).
- **Step 6 scope:** a full SSC-aware table, or just lower the token size
  assumption to cover Enterprise.
