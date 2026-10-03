# Issue: the RPC protocol can get stuck in `Recovering`

## Problem

When the RPC protocol (`SynchronousProtocol<ComPacket, ComPacket>`) can't continue, for example
after too many failed IF-RECVs or a protocol violation by the device, it goes into
`Phase::Recovering` and `ProtocolState` queues a STACK_RESET on the ComID protocol. In
`Recovering`, `poll_action` always returns `Action::None`.

The only way out of `Recovering` is in `ProtocolState::handle_iface_com_request_recv_done`. There,
`rpc_protocol.handle_reset()` is called only if the STACK_RESET response:

- matches both `com_id` and `com_id_ext`, and
- reports `StackResetStatus::Success` with `available_data_length >= 4`.

In any other case the RPC protocol stays in `Recovering` forever:

- the device reports `StackResetStatus::Failure`,
- the device responds with a different `com_id`/`com_id_ext`,
- the STACK_RESET request times out or its IF-SEND/IF-RECV fails, or
- the ComID protocol starts its own recovery while a stop has been requested. This aborts the
  pending STACK_RESET and doesn't queue a new one.

The method calls already queued, and any made later, are never sent. They aren't completed with an
error, so callers hang until their own timeouts (if any). The cause isn't reported to the user.

This was found through a bug in the virtual device: its STACK_RESET response had `com_id` in place
of `com_id_ext`, so the recovery never finished and all later tests hung.

## Suggested fix

- Watch the outcome of the recovery STACK_RESET instead of dropping its result
  (`oneshot::channel().0`).
- If the reset fails, times out, or is aborted, leave `Recovering` anyway:
    - fail all pending RPC requests with a descriptive error (and trace it), and
    - reset `rpc_protocol` and `rpc_session` so new calls can at least be attempted, or move to a
      terminal "broken" state where new calls fail immediately instead of hanging.
- Add tests for each failure case, against a mock device or a virtual device that can be told to
  fail the STACK_RESET.
