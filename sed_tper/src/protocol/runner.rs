use std::{sync::Arc, time::Instant};

use async_channel::RecvError;
use sed_async::timeout_at;
use sed_device::Device;
use sed_packet::{
    com_id::{ComIdRequest, ComIdResponse},
    session_id::SessionId,
};
use sed_spec::methods::Properties;

use crate::{
    Error,
    protocol::{protocol::Protocol, shared::Action},
};

pub async fn run(
    com_id: u16,
    com_id_ext: u16,
    device: Arc<dyn Device>,
    command_rx: async_channel::Receiver<Command>,
    _conn_tx: async_broadcast::Sender<ConnectionProperties>,
) {
    let mut protocol = Protocol::new(com_id, com_id_ext);

    loop {
        let action = protocol.poll_action(Instant::now());
        let is_idle = matches!(action, Action::None);
        let command = perform_action_or_recv(&*device, com_id, &mut protocol, &command_rx, action).await;
        if let Some(command) = command {
            inject_command(&mut protocol, command);
        } else if is_idle {
            break;
        }
        if command_rx.is_closed() {
            protocol.request_stop();
        }
    }
}

fn inject_command(protocol: &mut Protocol, command: Command) {
    match command {
        Command::MethodCall { session_id, call, sender } => protocol.handle_method_call(session_id, call, sender),
        Command::ComRequest { request, sender } => protocol.handle_com_request(request, sender),
        Command::ReportAborted { session_id } => protocol.handle_session_aborted(session_id),
        #[cfg(feature = "test-utils")]
        Command::Spawn { session_id, properties } => protocol.handle_spawn_session(session_id, properties),
    }
}

async fn perform_action_or_recv(
    device: &dyn Device,
    com_id: u16,
    protocol: &mut Protocol,
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

#[derive(Debug, Clone)]
pub struct Controller {
    command_tx: async_channel::Sender<Command>,
    conn_rx: async_broadcast::Receiver<ConnectionProperties>,
}

impl Controller {
    pub(crate) fn new(
        command_tx: async_channel::Sender<Command>,
        conn_rx: async_broadcast::Receiver<ConnectionProperties>,
    ) -> Self {
        Self { command_tx, conn_rx }
    }

    /// Perform an remote procedure call using tokenized methods.
    pub fn call(&self, session_id: SessionId, call: Vec<u8>) -> oneshot::Receiver<Result<Vec<u8>, Error>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.command_tx.try_send(Command::MethodCall { session_id, call, sender: tx });
        rx
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
    pub fn connection_properties(&self) -> async_broadcast::Receiver<ConnectionProperties> {
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
    ComRequest { request: ComIdRequest, sender: oneshot::Sender<Result<ComIdResponse, Error>> },
    ReportAborted { session_id: SessionId },
    #[cfg(feature = "test-utils")]
    Spawn { session_id: SessionId, properties: Properties },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionProperties {
    pub remote_properties: Properties,
    pub connection_properties: Properties,
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
        let (command_tx, command_rx) = async_channel::unbounded();
        let (conn_tx, conn_rx) = async_broadcast::broadcast(1);
        let controller = Controller::new(command_tx, conn_rx);
        let task = tokio::spawn(run(1, 0, Arc::new(device), command_rx, conn_tx));

        let response_rx = controller.call(SessionId::MANAGEMENT, call);
        let response_result = tokio::time::timeout(Duration::from_secs(5), response_rx).await;
        assert_that!(response_result, ok(ok(ok(eq(&response)))));

        drop(controller);
        let task_result = tokio::time::timeout(Duration::from_secs(5), task).await;
        assert_that!(task_result, ok(ok(eq(&()))));
    }
}
