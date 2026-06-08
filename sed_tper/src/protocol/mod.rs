mod com_id_session;
mod management;
mod protocol;
mod rpc_session;
mod runner;
mod sequence_number;
mod session;
mod shared;
mod synchronous_protocol;

use std::sync::Arc;

use sed_device::Device;

use crate::protocol::runner::{Command, run};
use shared::PropertiesChanged;

pub use protocol::CAPABILITIES;
pub use runner::Controller;
pub use shared::PropertiesChanged as ConnectionChanged;

/// The full protocol to communicate with the TPer via packets and ComID requests.
#[derive(Debug)]
pub struct Protocol {
    com_id: u16,
    com_id_ext: u16,
    device: Arc<dyn Device>,
    command_rx: async_channel::Receiver<Command>,
    conn_tx: async_broadcast::Sender<PropertiesChanged>,
}

impl Protocol {
    /// Create a new protocol stack for the `device` on the given ComID and
    /// ComID extension.
    ///
    /// This initializes the protocol stack, but no messages will be delivered
    /// until you call [`run`](Self::run).
    pub fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> (Self, Controller) {
        let (command_tx, command_rx) = async_channel::unbounded();
        let (conn_tx, conn_rx) = async_broadcast::broadcast(1);
        (Self { com_id, com_id_ext, device, command_rx, conn_tx }, Controller::new(command_tx, conn_rx))
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
    pub fn run(self) -> impl Future<Output = ()> + Send + 'static {
        run(self.com_id, self.com_id_ext, self.device, self.command_rx)
    }
}
