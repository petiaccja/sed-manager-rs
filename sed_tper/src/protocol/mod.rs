mod com_id_session;
mod management;
mod protocol_state;
mod rpc_session;
mod sequence_number;
mod session;
mod shared;
mod synchronous_protocol;

use std::sync::Arc;
use std::time::Instant;

use async_channel::RecvError;
use sed_async::timeout_at;
use sed_device::Device;
use sed_packet::{
    com_id::{ComIdRequest, ComIdResponse},
    session_id::SessionId,
};
use sed_spec::methods::Properties;

use crate::Error;
use crate::protocol::{
    protocol_state::ProtocolState,
    shared::{Action, PropertiesChanged},
};

pub use protocol_state::CAPABILITIES;
pub use shared::PropertiesChanged as ConnectionChanged;

/// The full protocol to communicate with the TPer via packets and ComID requests.
#[derive(Debug)]
pub struct Protocol {
    com_id: u16,
    device: Arc<dyn Device>,
    command_rx: async_channel::Receiver<Command>,
    state: ProtocolState,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> (Self, Controller) {
        let (command_tx, command_rx) = async_channel::unbounded();
        let state = ProtocolState::new(com_id, com_id_ext);
        let controller = Controller::new(command_tx, state.properties_changed());
        let protocol = Self { com_id, device, command_rx, state };
        (protocol, controller)
    }

    /// Send and receive messages until the protocol stack is shut down.
    ///
    /// You typically want to spawn this as a task on an async runtime. While
    /// executing, the protocol stack will accept commands through
    /// [`Controller`]s and exchange the message with the device while
    /// respecting the communication protocols.
    ///
    /// To shut down the protocol stack, drop all [`Controller`]s. Once they are
    /// dropped, the protocol stack will still handle pending messages and
    /// timeouts to ensure a graceful shutdown. This will leave the protocol
    /// stack on the device's side ready for a subsequent session, but might
    /// take a little time.
    pub async fn run(self) {
        let Self { com_id, device, command_rx, mut state } = self;

        loop {
            let action = state.poll_action(Instant::now());
            let is_idle = matches!(action, Action::None);
            let command = perform_action_or_recv(&*device, com_id, &mut state, &command_rx, action).await;
            if let Some(command) = command {
                inject_command(&mut state, command);
            } else if is_idle {
                break;
            }
            if command_rx.is_closed() {
                state.request_stop();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Controller {
    command_tx: async_channel::Sender<Command>,
    conn_rx: async_broadcast::Receiver<PropertiesChanged>,
}

impl Controller {
    pub(crate) fn new(
        command_tx: async_channel::Sender<Command>,
        conn_rx: async_broadcast::Receiver<PropertiesChanged>,
    ) -> Self {
        Self { command_tx, conn_rx }
    }

    /// Perform an remote procedure call using tokenized methods.
    pub fn call(&self, session_id: SessionId, call: Vec<u8>) -> oneshot::Receiver<Result<Vec<u8>, Error>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.command_tx.try_send(Command::MethodCall { session_id, call, sender: tx });
        rx
    }

    pub fn sync_properties(&self) {
        let _ = self.command_tx.try_send(Command::SyncProperties);
    }

    /// Notify the protocol stack that a session has been aborted by the device.
    ///
    /// This cleans up the session without sending an EndOfSession, since the
    /// device has already terminated it.
    pub fn report_aborted(&self, session_id: SessionId) {
        let _ = self.command_tx.try_send(Command::ReportAborted { session_id });
    }

    /// Send a ComID request to the device.
    pub fn com_id_request(&self, request: ComIdRequest) -> oneshot::Receiver<Result<ComIdResponse, Error>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.command_tx.try_send(Command::ComRequest { request, sender: tx });
        rx
    }

    /// Listen to changes in connection properties.
    ///
    /// The protocol manages the properties of the communication with the
    /// remote. When the properties change, an event is emitted on the channel.
    /// This typically only happens once at the beginning of the session, as
    /// it's enough to negotiate properties once. If no event comes on the
    /// channel, it means that the device did not respond to the request to
    /// negotiate properties.
    pub fn connection_properties(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.conn_rx.clone()
    }

    /// Spawn a new session with the given ID and properties.
    #[cfg(feature = "test-utils")]
    pub fn spawn(&self, session_id: SessionId, properties: Properties) {
        let _ = self.command_tx.try_send(Command::Spawn { session_id, properties });
    }
}

#[derive(Debug)]
#[rustfmt::skip] // Puts everything on a new line with the #[cfg].
pub enum Command {
    MethodCall { session_id: SessionId, call: Vec<u8>, sender: oneshot::Sender<Result<Vec<u8>, Error>> },
    SyncProperties,
    ComRequest { request: ComIdRequest, sender: oneshot::Sender<Result<ComIdResponse, Error>> },
    ReportAborted { session_id: SessionId },
    #[cfg(feature = "test-utils")]
    Spawn { session_id: SessionId, properties: Properties },
}

fn inject_command(state: &mut ProtocolState, command: Command) {
    match command {
        Command::MethodCall { session_id, call, sender } => state.handle_method_call(session_id, call, sender),
        Command::SyncProperties => state.handle_sync_properties(),
        Command::ComRequest { request, sender } => state.handle_com_request(request, sender),
        Command::ReportAborted { session_id } => state.handle_session_aborted(session_id),
        #[cfg(feature = "test-utils")]
        Command::Spawn { session_id, properties } => state.handle_spawn_session(session_id, properties),
    }
}

async fn perform_action_or_recv(
    device: &dyn Device,
    com_id: u16,
    protocol: &mut ProtocolState,
    rx: &async_channel::Receiver<Command>,
    action: Action,
) -> Option<Command> {
    let com_id = com_id.to_be_bytes();
    match action {
        Action::None => rx.recv().await.ok(),
        Action::Send { protocol: sec_proto, data } => {
            let result = device.security_send(sec_proto, com_id, &data).await;
            protocol.handle_iface_send_done(Instant::now(), sec_proto, result.map_err(|err| err.into()));
            None
        }
        Action::Recv { protocol: sec_proto, transfer_len } => {
            let result = device.security_recv(sec_proto, com_id, transfer_len).await;
            protocol.handle_iface_recv_done(Instant::now(), sec_proto, result.map_err(|err| err.into()));
            None
        }
        Action::Sleep { until } => {
            let result = timeout_at(until, rx.recv()).await;
            match result {
                Ok(Ok(command)) => Some(command),
                Ok(Err(RecvError)) => None,
                Err(()) => None,
            }
        }
        Action::Recover => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    use googletest::assert_that;
    use googletest::matchers::*;
    use sed_device::mock_device::{MockDevice, MockEvent};
    use sed_packet::packet::ComPacket;
    use sed_spec::methods::MethodStatus;
    use sorbit::ser_de::ToBytes as _;

    use crate::protocol::{
        sequence_number::SequenceNumber,
        shared::{
            packetize_one,
            tests::{start_session_call, sync_session_call},
        },
    };

    const SESSION_ID: SessionId = SessionId { hsn: 1, tsn: 2 };

    #[tokio::test]
    async fn method_call_completed() {
        let call = start_session_call(SESSION_ID);
        let response = sync_session_call(SESSION_ID, MethodStatus::Success);
        let call_packet = ComPacket {
            com_id: 1,
            com_id_ext: 0,
            outstanding_data: 0,
            min_transfer: 0,
            length: std::marker::PhantomData,
            payload: vec![packetize_one(
                SessionId::MANAGEMENT,
                SequenceNumber(1),
                call.clone(),
            )],
        };
        let response_packet = ComPacket {
            com_id: 1,
            com_id_ext: 0,
            outstanding_data: 0,
            min_transfer: 0,
            length: std::marker::PhantomData,
            payload: vec![packetize_one(
                SessionId::MANAGEMENT,
                SequenceNumber(1),
                response.clone(),
            )],
        };

        let scenario = [
            MockEvent::Send {
                name: Some("call".into()),
                security_protocol: 0x01,
                protocol_specific: [0x00, 0x01],
                expected: call_packet.to_bytes().unwrap(),
                result: Ok(()),
            },
            MockEvent::Recv {
                name: Some("response".into()),
                security_protocol: 0x01,
                protocol_specific: [0x00, 0x01],
                result: Ok(response_packet.to_bytes().unwrap()),
            },
        ];

        let device = MockDevice::new(scenario.into_iter());
        let (protocol, controller) = Protocol::new(1, 0, Arc::new(device));
        let task = tokio::spawn(protocol.run());

        let response_rx = controller.call(SessionId::MANAGEMENT, call);
        let response_result = tokio::time::timeout(Duration::from_secs(5), response_rx).await;
        assert_that!(response_result, ok(ok(ok(eq(&response)))));

        drop(controller);
        let task_result = tokio::time::timeout(Duration::from_secs(5), task).await;
        assert_that!(task_result, ok(ok(eq(&()))));
    }
}
