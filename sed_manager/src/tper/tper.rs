use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::device::Device;
use crate::messaging::com_id::{ComIdState, StackResetStatus};
use crate::messaging::discovery::Discovery;
use crate::rpc::{CommandSender, Error as RPCError, Properties, Protocol, Runtime, SessionIdentifier};
use crate::spec::column_types::{AuthorityRef, SPRef};

use super::com_session::ComSession;
use super::control_session::ControlSession;
use super::sp_session::SPSession;

pub struct TPer {
    com_id: u16,
    com_id_ext: u16,
    next_hsn: AtomicU32,
    capabilities: Properties,
    properties: Mutex<Option<Properties>>,
    com_session: ComSession,
    control_session: ControlSession,
    message_sender: CommandSender,
    #[allow(unused)]
    runtime: Arc<dyn core::any::Any>, // Runs the protocol and has to be kept alive
}

pub use crate::rpc::discover;

pub fn get_primary_ssc_com_id(discovery: &Discovery) -> Option<(u16, u16)> {
    discovery.get_primary_ssc().map(|ssc| (ssc.base_com_id(), 0))
}

impl TPer {
    pub fn new<R: Runtime>(device: Arc<dyn Device>, runtime: Arc<R>, com_id: u16, com_id_ext: u16) -> Self {
        let capabilities = Protocol::capabilities();
        let (message_sender, _) = Protocol::spawn(device, &*runtime, com_id, com_id_ext, capabilities.clone());
        Self {
            com_id,
            com_id_ext,
            next_hsn: 1.into(),
            capabilities,
            properties: None.into(),
            message_sender: message_sender.clone(),
            com_session: ComSession::new(message_sender.clone()),
            control_session: ControlSession::new(message_sender.clone()),
            runtime: runtime,
        }
    }

    pub fn new_on_default_com_id<R: Runtime>(device: Arc<dyn Device>, runtime: Arc<R>) -> Result<Self, RPCError> {
        let discovery = discover(&*device)?;
        if let Some((com_id, com_id_ext)) = get_primary_ssc_com_id(&discovery) {
            Ok(Self::new(device, runtime, com_id, com_id_ext))
        } else {
            Err(RPCError::NotSupported)
        }
    }

    pub fn com_id(&self) -> u16 {
        self.com_id
    }

    pub fn com_id_ext(&self) -> u16 {
        self.com_id_ext
    }

    pub fn capabilities(&self) -> &Properties {
        &self.capabilities
    }

    pub async fn discover(&self) -> Result<Discovery, RPCError> {
        self.message_sender.discover().await
    }

    pub async fn current_properties(&self) -> Properties {
        let mut maybe_properties = self.properties.lock().await;
        if let Some(properties) = maybe_properties.deref() {
            properties.clone()
        } else {
            self.change_properties_with_lock(maybe_properties.deref_mut(), &self.capabilities).await
        }
    }

    pub async fn change_properties(&self, properties: &Properties) -> Properties {
        // The caller might give something that exceeds our own capabilities.
        let properties = Properties::common(properties, &self.capabilities);
        let mut output = self.properties.lock().await;
        self.change_properties_with_lock(output.deref_mut(), &properties).await
    }

    async fn change_properties_with_lock(&self, output: &mut Option<Properties>, requested: &Properties) -> Properties {
        let properties = match self.control_session.properties(Some(requested.to_list())).await {
            Ok((tper_capabilities, tper_properties)) => {
                let tper_properties = Properties::from_list(&tper_properties.unwrap_or(tper_capabilities));
                Properties::common(&self.capabilities, &tper_properties)
            }
            Err(_) => Properties::ASSUMED,
        };
        output.replace(properties.clone());
        properties
    }

    pub async fn start_session(
        &self,
        sp: SPRef,
        authority: Option<AuthorityRef>,
        password: Option<&[u8]>,
    ) -> Result<SPSession, RPCError> {
        let hsn = self.next_hsn.fetch_add(1, Ordering::Relaxed);
        let properties = self.current_properties().await;
        let sync_session = self
            .control_session
            .start_session(hsn, sp, true, password, None, None, authority, None, None, None, None, None)
            .await?;
        if sync_session.hsn != hsn {
            return Err(RPCError::Unspecified);
        };
        let trans_timeout = sync_session
            .trans_timeout
            .map(|ms| Duration::from_millis(ms as u64))
            .unwrap_or(properties.def_trans_timeout);
        let properties = Properties { trans_timeout, ..properties };
        Ok(SPSession::new(
            SessionIdentifier { hsn, tsn: sync_session.tsn },
            self.message_sender.clone(),
            properties,
        ))
    }

    pub async fn verify_com_id(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, RPCError> {
        self.com_session.verify_com_id(com_id, com_id_ext).await
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<StackResetStatus, RPCError> {
        let status = self.com_session.stack_reset(com_id, com_id_ext).await;
        let success = status.as_ref().is_ok_and(|status| status == &StackResetStatus::Success);
        let same = (com_id, com_id_ext) == (self.com_id, self.com_id_ext);
        if success && same {
            let _ = self.properties.lock().await.take();
        }
        status
    }
}
