//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use sed_async::{PolyRuntime, Runtime};
use sed_device::StorageDevice;
use sed_packet::discovery::Discovery;
use sed_packet::{
    MaxBytes,
    com_id::{ComIdRequest, ComIdResponsePayload, ComIdState, StackResetStatus},
};
use sed_spec::{
    methods::Properties,
    objects::{AuthorityRef, SecurityProviderRef},
};
use sorbit::ser_de::FromBytes;
use tracing::instrument;

use crate::{
    Error, Session,
    protocol::{CAPABILITIES, Controller, PropertiesChanged, Protocol},
};

/// A connection to the storage device using a particular ComID/ComIDExt pair.
///
/// Through the `Tper`, you can perform all communication layer, management
/// layer, and session layer operations on the device. This means managing
/// ComIDs, creating session, and invoking RPC methods.
///
/// It is a very bad idea to connect multiple `Tper`s on the same ComID/ComIDExt
/// pair, and will most certainly lead to catastrophe. You should use dynamic
/// ComID management if your device supports it, or stick with one `Tper` per
/// base ComID.
#[derive(Debug)]
pub struct Tper {
    com_id: u16,
    com_id_ext: u16,
    device: Arc<dyn StorageDevice>,
    controller: Controller,
    protocol_task: <PolyRuntime as Runtime>::JoinHandle<()>,
    host_session_id: Arc<AtomicU32>,
}

impl Tper {
    /// The ComID on which the `Tper` is connected.
    pub fn com_id(&self) -> u16 {
        self.com_id
    }

    /// The ComID extension on which the `Tper` is connected.
    pub fn com_id_ext(&self) -> u16 {
        self.com_id_ext
    }

    /// Get the communication capabilities of the host. These properties
    /// reflect how this library implements the TCG protocol stack.
    pub fn capabilities(&self) -> Properties {
        CAPABILITIES
    }

    /// Create a handle to the Tper's connection and protocol stack.
    ///
    /// The [`TperHandle`] can perform a subset of the tasks of the [`Tper`].
    pub fn handle(&self) -> TperHandle {
        TperHandle { controller: self.controller.clone(), host_session_id: self.host_session_id.clone() }
    }

    /// Listen to changes in the connection properties.
    ///
    /// Unlike the capabilities of the host or the device, the connection's
    /// properties are dynamic. The connection properties are negotiated between
    /// the host and the device to limits that both parties can meet.
    ///
    /// When the communication properties change, an event is emitted. The
    /// device's capabilities are also returned.
    pub fn properties_changed(&self) -> async_broadcast::Receiver<PropertiesChanged> {
        self.controller.properties_changed()
    }

    /// Connect to a device on the specified ComID and ComID extension.
    ///
    /// The ComID pair has to either be a static ComID or dynamically allocated
    /// prior to calling this function.
    ///
    /// To run the communication protocol, an async runtime is required. If you
    /// don't specify one explicitly, the one associated with the current thread
    /// will be picked.
    ///
    /// If you terminate the async runtime, and thereby the protocol, all
    /// subsequent requests will time out, and you'll likely need to do a stack
    /// reset to get the device's communication stack synchronized again.
    #[instrument(level = "info", skip(runtime))]
    pub fn connect(com_id: u16, com_id_ext: u16, device: Arc<dyn StorageDevice>, runtime: Arc<PolyRuntime>) -> Self {
        let (protocol, controller) = Protocol::new(com_id, com_id_ext, device.clone(), runtime.clone());
        controller.sync_properties();
        let protocol_task = runtime.spawn(protocol.run());
        let host_session_id = Arc::new(AtomicU32::new(1));
        Self { com_id, com_id_ext, device, controller, protocol_task, host_session_id }
    }

    /// Discover the capabilities of the provided device.
    #[instrument(level = "info", ret, err)]
    pub async fn discover(device: &dyn StorageDevice) -> Result<Discovery, Error> {
        let bytes = device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096).await?;
        Discovery::from_bytes(&bytes).map_err(Error::InvalidDiscovery)
    }

    /// Discover the currently connected device.
    #[instrument(level = "info", skip(self))]
    pub async fn discover_current(&self) -> Result<Discovery, Error> {
        Self::discover(&*self.device).await
    }

    /// Verify the status of a ComID.
    ///
    /// The argument may be any ComID, it doesn't have to be the one on which
    /// this [`Tper`] is connected. The ComID may also be invalid.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn verify_com_id_valid(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, Error> {
        use ComIdResponsePayload::*;

        let request = ComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.payload {
            Verify { com_id_state, .. } => Ok(com_id_state),
            _ => Err(Error::TimedOut),
        }
    }

    /// Reset the stack on the given ComID.
    ///
    /// All sessions will be terminated on the ComID. Use this when some session
    /// got stuck or desynchronized, and the device is not responding to RPCs as
    /// expected.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<(), Error> {
        use ComIdResponsePayload::*;
        use StackResetStatus::*;

        let request = ComIdRequest::stack_reset(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.payload {
            StackReset { status: Success, available_data_length: 1.. } => {
                self.controller.sync_properties();
                Ok(())
            }
            StackReset { available_data_length: 0, .. } => {
                tracing::error!(context = "this event should not escape the protocol", "stack_reset_pending");
                Err(Error::StackResetFailed)
            }
            StackReset { status: Failure, available_data_length: 1.. } => Err(Error::StackResetFailed),
            NoResponseAvailable { .. } => Err(Error::TimedOut),
            Verify { .. } => Err(Error::TimedOut),
        }
    }

    /// Start an RPC session on the given `sp`, optionally authentaced as
    /// `authority`.
    ///
    /// If the authority is omitted, the session will start on the `Anybody`
    /// authority. You can later use the [`authenticate`] method inside the
    /// session to authenticate, provided that the device supports it.
    ///
    /// The spawned session will inherit the RPC protocol of the `Tper`. The
    /// protocol is owned jointly, and will only shut down once all sessions
    /// are terminated.
    ///
    /// [`authenticate`]: Session::authenticate
    #[instrument(level = "debug", skip(self, password), ret, err)]
    pub async fn start_session(
        &self,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
        password: Option<MaxBytes<32>>,
    ) -> Result<Session, Error> {
        let host_session_number = self.host_session_id.fetch_add(1, Ordering::Relaxed);
        Session::start(self.controller.clone(), host_session_number, sp, authority, password).await
    }

    /// Initiate the shutdown of the protocol stack held internally.
    ///
    /// Once the returned future is complete, the protocol stack is shut down.
    /// Note that all live [`Session`]s that hold a reference to the protocol stack
    /// needs to be shut down and dropped, otherwise the protocol stack will not be
    /// able to shut down.
    #[instrument(level = "debug", skip(self))]
    pub fn close(self) -> <PolyRuntime as Runtime>::JoinHandle<()> {
        self.protocol_task
    }
}

/// A handle to the `Tper`'s protocol stack through which you can do a subset of
/// the [`Tper`]'s tasks.
///
/// The [`Tper`] itself owns the underlying protocol stack that communicates with
/// the [`Device`]. Closing the protocol stack must be done through [`Tper::close`],
/// the [`TperHandle`] cannot be used for purpose. Keep it in mind though that
/// live handles will keep the protocol stack alive and `close()` will not
/// return until you drop all handles and all sessions.
///
/// For the moment, this object deals only with session management. This could
/// be extended in theory, but in the current codebase there are limited uses
/// of this object so keeping the API minimal makes it less likely to spread
/// and makes it easier to remove if future design changes.
#[derive(Debug)]
pub struct TperHandle {
    controller: Controller,
    host_session_id: Arc<AtomicU32>,
}

impl TperHandle {
    /// See [`Tper::start_session`].
    #[instrument(level = "debug", skip(self, password), ret, err)]
    pub async fn start_session(
        &self,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
        password: Option<MaxBytes<32>>,
    ) -> Result<Session, Error> {
        let host_session_number = self.host_session_id.fetch_add(1, Ordering::Relaxed);
        Session::start(self.controller.clone(), host_session_number, sp, authority, password).await
    }
}
