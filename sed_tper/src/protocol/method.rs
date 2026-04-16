use std::collections::VecDeque;
use std::time::Instant;

use sed_packet::Ignore;
use sed_packet::token::{Detokenize, Detokenizer, MessageError};
use sed_spec::methods::{MethodResult, MgmtMethodCall, MgmtMethodCallParams, Properties};
use tracing::Span;

use crate::error::Error;
use crate::protocol::message::MethodResponse;

pub type AnyMethodResult = MethodResult<Vec<Ignore>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MgmtMethodCallSend {
    StartSession { hsn: u32 },
    SyncSession { hsn: u32, tsn: u32 },
    CloseSession { hsn: u32, tsn: u32 },
}

impl Detokenize for MgmtMethodCallSend {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let call = MgmtMethodCall::detokenize(detokenizer)?;
        match call.params {
            MgmtMethodCallParams::StartSession(params) => {
                Ok(MgmtMethodCallSend::StartSession { hsn: params.host_session_id })
            }
            MgmtMethodCallParams::SyncSession(params) => {
                Ok(MgmtMethodCallSend::SyncSession { hsn: params.host_session_id, tsn: params.sp_session_id })
            }
            MgmtMethodCallParams::CloseSession(params) => Ok(MgmtMethodCallSend::CloseSession {
                hsn: params.remote_session_number,
                tsn: params.local_session_number,
            }),
            MgmtMethodCallParams::Properties(_) => {
                Err(D::Error::message("the properties may only be set by the protocol"))
            }
        }
    }
}

/// Information about methods that are being written via IF-SEND.
#[derive(Debug)]
pub struct WriteQueuedMethod {
    pub channel: oneshot::Sender<MethodResponse>,
    pub span: Span,
    /// This is information is required by the management session.
    pub mgmt_session_meta: Option<Box<(MgmtMethodCallSend, Properties)>>,
}

/// Information about methods that are waiting for IF-RECV.
#[derive(Debug)]
pub struct RecvQueuedMethod {
    pub channel: oneshot::Sender<MethodResponse>,
    pub span: Span,
    pub deadline: Instant,
    /// This is information is required by the management session.
    pub mgmt_session_meta: Option<Box<Properties>>,
}

/// Retains the pending methods that haven't timed out yet.
///
/// A timeout error is sent as a reply to those that timed out.
///
/// # Returns
///
/// The number of methods that have timed out.
pub fn retain_alive(time: Instant, queue: &mut VecDeque<RecvQueuedMethod>) -> usize {
    let mut count = 0;
    while let Some(RecvQueuedMethod { deadline, .. }) = queue.front()
        && deadline <= &time
    {
        count += 1;
        let RecvQueuedMethod { channel, span, .. } = queue.pop_front().unwrap();
        let _ = channel.send((Err(Error::TimedOut), span));
    }
    count
}
