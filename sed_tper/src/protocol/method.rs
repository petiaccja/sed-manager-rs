use std::collections::VecDeque;
use std::time::Instant;

use sed_packet::token::{Command, Detokenize, Detokenizer, MessageError};
use sed_packet::{Ignore, Uid};
use sed_spec::preconfig::core::shared::invoking_id::SESSION_MANAGER;
use sed_spec::preconfig::core::shared::sm_method_id::{CLOSE_SESSION, PROPERTIES, START_SESSION, SYNC_SESSION};
use tracing::Span;

use crate::error::Error;
use crate::method_status::MethodStatus;
use crate::protocol::message::MethodResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodResultPlaceholder;

impl Detokenize for MethodResultPlaceholder {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let _args = Vec::<Ignore>::detokenize(detokenizer)?;
        let end_of_data = Command::detokenize(detokenizer)?;
        if end_of_data != Command::EndOfData {
            return Err(D::Error::message("expected a CALL token"));
        }
        let _status_code = Vec::<MethodStatus>::detokenize(detokenizer)?;

        Ok(MethodResultPlaceholder)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodCallPlaceholder {
    StartSession { hsn: u32 },
    SyncSession { hsn: u32 },
    CloseSession { hsn: u32, tsn: u32 },
    Properties,
    Session,
    EndOfSession,
}

impl Detokenize for MethodCallPlaceholder {
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let command = Command::detokenize(detokenizer)?;
        match command {
            Command::Call => (),
            Command::EndOfSession => return Ok(MethodCallPlaceholder::EndOfSession),
            _ => return Err(D::Error::message("expected a CALL token")),
        }

        let invoking_id = Uid::detokenize(detokenizer)?;

        let method_id = Uid::detokenize(detokenizer)?;

        let method_call = if invoking_id == SESSION_MANAGER {
            let mut arg_index = 0;
            match method_id {
                START_SESSION => {
                    let mut hsn = 0;
                    detokenizer.detokenize_list(|de| match fetch_add(&mut arg_index, 1) {
                        0 => {
                            hsn = u32::detokenize(de)?;
                            Ok(())
                        }
                        _ => Ignore::detokenize(de).map(|_| ()),
                    })?;
                    MethodCallPlaceholder::StartSession { hsn }
                }
                SYNC_SESSION => {
                    let mut hsn = 0;
                    detokenizer.detokenize_list(|de| match fetch_add(&mut arg_index, 1) {
                        0 => {
                            hsn = u32::detokenize(de)?;
                            Ok(())
                        }
                        _ => Ignore::detokenize(de).map(|_| ()),
                    })?;
                    MethodCallPlaceholder::SyncSession { hsn }
                }
                CLOSE_SESSION => {
                    let mut hsn = 0;
                    let mut tsn = 0;
                    detokenizer.detokenize_list(|de| match fetch_add(&mut arg_index, 1) {
                        0 => {
                            hsn = u32::detokenize(de)?;
                            Ok(())
                        }
                        1 => {
                            tsn = u32::detokenize(de)?;
                            Ok(())
                        }
                        _ => Ignore::detokenize(de).map(|_| ()),
                    })?;
                    MethodCallPlaceholder::CloseSession { hsn, tsn }
                }
                PROPERTIES => {
                    Vec::<Ignore>::detokenize(detokenizer)?;
                    MethodCallPlaceholder::Properties
                }
                _ => return Err(D::Error::message(format!("unrecognized SM method ID: {invoking_id}"))),
            }
        } else {
            Vec::<Ignore>::detokenize(detokenizer)?;
            MethodCallPlaceholder::Session
        };

        let end_of_data = Command::detokenize(detokenizer)?;
        if end_of_data != Command::EndOfData {
            return Err(D::Error::message("expected a CALL token"));
        }
        let _status_code = Vec::<MethodStatus>::detokenize(detokenizer)?;

        Ok(method_call)
    }
}

#[derive(Debug)]
pub struct PendingMethod {
    pub channel: oneshot::Sender<MethodResult>,
    pub span: Span,
    pub deadline: Instant,
}

/// Retains the pending methods that haven't timed out yet.
///
/// A timeout error is sent as a reply to those that timed out.
///
/// # Returns
///
/// The number of methods that have timed out.
pub fn retain_alive(time: Instant, queue: &mut VecDeque<PendingMethod>) -> usize {
    let mut count = 0;
    while let Some(PendingMethod { deadline, .. }) = queue.front()
        && deadline <= &time
    {
        count += 1;
        let PendingMethod { channel, span, .. } = queue.pop_front().unwrap();
        let _ = channel.send((Err(Error::TimedOut), span));
    }
    count
}

fn fetch_add(value: &mut usize, rhs: usize) -> usize {
    let current = *value;
    *value += rhs;
    current
}
