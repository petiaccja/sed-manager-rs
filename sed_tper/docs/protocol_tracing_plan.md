# Plan: correlating IF-SEND/IF-RECV traces with RPC requests

## Goal

The `Controller` captures the `tracing` span of each method call or ComID request. Correlate
the interface commands (IF-SEND, IF-RECV) that carry out those requests with the request spans,
for both the synchronous and a future asynchronous protocol. Only OpenTelemetry tools are used,
so plain log output is not a concern.

## Design

### What is known about each command

- **IF-SEND:** the payload (ComPacket or ComID request) is known in advance, so the exact set of
  requests it carries is known before the command is issued.
- **IF-RECV:** which requests it will answer is only known after the command completes. Before it
  runs, only the *awaiting* set is known: requests that have been sent and are waiting for a
  response.

### Constraints from `tracing-opentelemetry` (0.32.1)

- Links (`follows_from`) can be added any time before the span closes, even after it started.
- The parent cannot be changed after the span starts. Entering the span (context activation) or
  creating a child starts it, and the IF command span is entered around the IOCTL and has
  `sed_device` child spans. The parent must therefore be chosen when the span is created.
- Events emitted outside any span are not exported, so without a `run` span every event must be
  emitted inside some span.
- A `Span` handle keeps its span open until all clones are dropped. The records hold clones, so
  request spans cannot close while the protocol still links to them.

### Policy

- **IF-SEND:** child of the last carried span, skipping `Span::none()`. The carried spans are the
  spans of the packets in the ComPacket, flattened in packet order. This is the request that
  triggered the IF-SEND. The IF-SEND is linked both ways with every other carried request.
- **IF-RECV:** child of the awaiting request with the earliest `sent_at` (the time its IF-SEND
  completed), across all sessions of the IF-RECV's protocol. The IF-RECV span is passed down with
  the received data, and the lower layers link it both ways with every request it delivered to,
  including its parent.
- **No request span available:** internal requests get their own spans where it's practical (see
  step 6). Otherwise, the IF command span becomes a root span.
- **No `run` span:** `Protocol::run` and `perform_action_or_recv` are no longer instrumented.

In the synchronous one-to-one case, the request span contains its IF-SEND followed by all its
IF-RECVs, including empty polls. Nothing depends on synchronous cycles, so the scheme carries
over to the asynchronous protocol.

With several requests awaiting, the IF-RECV's parent means "polled while this request was the
oldest outstanding one". It may deliver to another request instead; the links make that
visible.

Notes on the ordering:

- Requests carried by the same ComPacket share the same `sent_at`. Any of them is a reasonable
  parent, since they went out in the same IF-SEND, and the others still get links on delivery.
- Within a session, packets and the methods inside them are first-in, first-out, so "last" is
  the most recently issued request. Across sessions, the order comes from iterating
  `RpcSession::sessions`, a `HashMap`, so it's arbitrary. That's acceptable today because every
  ComPacket carries a single session's packet. If sessions are ever batched into one ComPacket,
  the batching should preserve issue order; add a comment where the vector is built.

### Spans are attached to packets

Under TCG batching, a single packet may carry several methods (in one or more sub-packets), and
the packets produced by one poll may be split across several ComPackets, each sent with its own
IF-SEND. Spans are therefore attached to each packet individually rather than to a batch of
packets: the send path carries `Vec<(Packet, Vec<Span>)>`, where the inner vector holds the spans
of the methods inside that packet, in method order. Whichever layer groups packets into
ComPackets moves each packet's spans along with it. From there on, each ComPacket (or ComID
request) is queued in `SynchronousProtocol` together with its spans, so every IF-SEND sees
exactly the spans of the packets it carries.

## Out of scope

- **The `Properties` method.** Its response is processed whenever it arrives and isn't paired
  with a request, so it is not traced by this plan. The IF-SEND carrying the host's Properties
  call has no request span and becomes a root span, and the IF-RECV that delivers the response
  makes no link for it.

## Current state of the code

- Each session's packets are sent in a separate ComPacket, with at most one packet per session
  per poll, so every IF-SEND currently carries one method. The multi-request path is still
  implemented for the asynchronous protocol and for future batching.
- `Management`'s `MethodSendingRecord` and `MethodReceivingRecord` have no `span` field, so a
  StartSession loses its span once it is sent.
- The recovery stack reset is queued with `Span::none()`.
- The protocol emits no events.

## Implementation steps

### Step 1: Spans and send times in the records

- **Management:** add the missing `span: Span` to `MethodSendingRecord` and
  `MethodReceivingRecord`.
- **Receiving records:** add `sent_at: Instant` to the receiving records in `session.rs`,
  `management.rs`, and `com_id_session.rs`. It is set from the `time` that
  `handle_iface_send_done` already receives.
- **Optional:** derive `deadline` as `sent_at + timeout` instead of storing it. Keeping both
  fields is also fine.

### Step 2: Carried spans on the send path

- **Action types:** `session::Action::Send`, `management::Action::Send`, and `RpcAction::Send`
  change from `Send(Vec<Packet>)` to `Send(Vec<(Packet, Vec<Span>)>)`. Each packet is paired with
  the spans of the methods it carries, in method order.
  - `Session::poll_action` and `Management::poll_method_calls` pair each packet with the span of
    the method call they packetize. A packet without a traced request (the host's Properties
    call) gets an empty vector.
  - `Span` isn't `PartialEq`, so these enums drop their `PartialEq`/`Eq` derives.
- **`RpcSession`:** `packets_to_send` becomes `VecDeque<Vec<(Packet, Vec<Span>)>>`, and
  `reduce_actions` passes the pairs through unchanged.
- **Grouping into ComPackets:** `ProtocolState::poll_action` builds the ComPacket from the
  packets of the pairs and flattens the packets' spans, in packet order, into the ComPacket's
  spans.
  - Today, each `RpcAction::Send` becomes exactly one ComPacket. If this grouping ever splits the
    packets across several ComPackets, each ComPacket gets only the spans of its own packets.
- **Protocol 0x02:** `ComIdAction::Send(ComIdRequest)` becomes `Send(ComIdRequest, Span)`.
  `ComIdSession::poll_action` returns a clone of the span it keeps in `request_sending`.
- **Messages and spans travel together through `SynchronousProtocol`:**
  - `handle_send(message)` becomes `handle_send(message, spans: Vec<Span>)`, and the queue
    becomes `VecDeque<(SendMessage, Vec<Span>)>`.
  - `poll_action` returns `(Action, Vec<Span>)`. The spans are those of the message it pops when
    it returns `Action::Send`, and empty for every other action.
  - `ProtocolState::poll_action` passes them through, also returning `(Action, Vec<Span>)`, and
    the runner uses them as the carried spans for the IF-SEND.
  - The shared `Action` keeps its `PartialEq`/`Eq` derives.

### Step 3: Parent span for the next IF-RECV

Each layer gets a `next_recv_span` method that returns the span the next IF-RECV is parented
under: the awaiting request with the earliest `sent_at`. The query is passed down the layers:

```rust
// Session, Management, ComIdSession, RpcSession
fn next_recv_span(&self) -> Option<(Instant, &Span)>;

// ProtocolState
pub fn next_recv_span(&self, protocol: u8) -> Option<Span>;
```

| Layer           | `next_recv_span`                                                                                     |
| --------------- | ---------------------------------------------------------------------------------------------------- |
| `Session`       | Front of `method_calls_receiving` when `Active`, otherwise `None`                                    |
| `Management`    | Minimum by `sent_at` over the fronts of the `start_session_calls_receiving` queues                  |
| `RpcSession`    | Minimum by `sent_at` over `Management` and every `Session`                                           |
| `ComIdSession`  | `request_receiving`                                                                                  |
| `ProtocolState` | Dispatches on `protocol` to `com_id_session` or `rpc_session`, drops the `Instant`, clones the span |

- Only queue fronts are considered: records enter a receiving queue at send completion, so each
  queue is ordered by `sent_at`.
- The inner layers return a borrowed `&Span`; only `ProtocolState` clones it.
- Ties are resolved by `min_by_key`, which returns the first of equal keys.

### Step 4: IF command spans in the runner (`mod.rs`)

- Remove `#[instrument]` from `run` and from `perform_action_or_recv`.
- Add a linking helper to `shared.rs`:

```rust
/// Links two spans both ways with `follows_from`.
pub fn link_both_ways(a: &Span, b: &Span);
```

  `Span::follows_from` is a no-op when either span is disabled, so the helper needs no special
  cases.
- **Span creation:**
  - Use `info_span!(parent: p, "if_send", protocol, len)` and
    `info_span!(parent: p, "if_recv", protocol, transfer_len)`, with `parent: None` when there is
    no request span.
  - Record only the protocol and lengths. Never record payloads, because tokens can contain
    passwords or `DataStore` data.
- **IF-SEND:** after creating the span, call `link_both_ways` for every carried span except the
  one chosen as parent.
- **Running the command:** run the device call as `.instrument(iface.clone())`, so the spans
  `sed_device` already creates nest underneath it.
- **Handling the result:** call `handle_iface_*_done` inside `iface.in_scope(..)`.
  `handle_iface_recv_done` also takes `&iface` (step 5).
- **Where the spans come from:** for IF-SEND, the carried spans returned by
  `ProtocolState::poll_action` (step 2). For IF-RECV, `state.next_recv_span(protocol)` (step 3),
  called right after `poll_action` returns `Action::Recv` and before the `await`. It must come
  after `poll_action`, because `poll_action` removes timed-out records. A span handed to
  `perform_action_or_recv` can't borrow `state`, so the spans are owned clones. Cloning a `Span`
  is cheap (refcounted).

### Step 5: Passing the IF-RECV span down to delivery points

The IF-RECV span (`iface: &Span`) is threaded from `handle_iface_recv_done` through
`handle_iface_com_packet_recv_done` and `RpcSession::handle_packet` into the handlers below.
`link_both_ways(iface, &record.span)` is called wherever tokens are attributed to a request:

- **`Session::handle_tokens`:** when a packet's tokens arrive, link the front of
  `method_calls_receiving`, even if the response is only partial. Also link every record
  completed in the extraction loop, including the EOS case.
- **`Management::handle_tokens`:** for SyncSession, link the record matched by HSN.
- **`ComIdSession::handle_iface_recv_done`:** link `request_receiving`.

The IF-RECV's parent request is linked as well, so in the one-to-one case it is both the parent
and a link of the IF-RECV.

### Step 6: Internal requests

- **Recovery stack reset:** `ProtocolState::poll_action` creates
  `info_span!(parent: <parent>, "stack_reset", reason = "recovery")`, where the parent is
  `next_recv_span` (step 3) of the protocol that requested recovery, or a root span if it returns
  `None`. For protocol 0x02, the parent is queried before `com_id_session.handle_reset()` clears
  the records. The request is queued with this span.
- **EOS on abort:**
  - `State::Aborting` becomes `Aborting { cause: Span }`, where the cause is the IF-RECV span that
    delivered the bad tokens (from step 5).
  - The EOS packet is sent as `Send(vec![(eos_packet, vec![cause])])`.

### Step 7: Updating the existing tests

No new tests are added for the traces. The existing tests are updated so that they compile and
pass after each step, in the same commit as the change that affects them:

- **Step 2:**
  - Tests matching `Send` on `session::Action`, `management::Action`, or `RpcAction` match the
    packets and ignore the spans, e.g.
    `pat!(Action::Send(elements_are![(eq(&packet), anything())]))`.
  - Calls to `SynchronousProtocol::handle_send` pass a span vector, e.g. `vec![]`.
  - Tests calling `poll_action` on `SynchronousProtocol` or `ProtocolState` take `.0` of the
    result.
- **Step 5:** calls to `handle_iface_recv_done`, `handle_packet`, and `handle_tokens` pass
  `&Span::none()` as the IF-RECV span.
- **Steps 1, 3, 4, and 6** change internal state and the runner only; they don't change any
  signature used by the tests.

## Suggested commit order

Steps 1–2 together, then 3–4 (useful traces from this point), then 5, then 6. Each commit
includes its test updates from step 7, and builds and passes the tests.
