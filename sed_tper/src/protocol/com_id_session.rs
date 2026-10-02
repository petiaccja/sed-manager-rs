//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::com_id::{ComIdRequest, ComIdResponse};
use tracing::Span;

use crate::{Error, protocol::shared::link_both_ways};

#[derive(Debug)]
pub struct ComIdSession {
    timeout: Duration,
    requests: VecDeque<RequestRecord>,
    request_sending: Option<RequestSendingRecord>,
    request_receiving: Option<RequestReceivingRecord>,
}

impl ComIdSession {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout, requests: VecDeque::new(), request_sending: None, request_receiving: None }
    }

    pub fn handle_com_request(
        &mut self,
        request: ComIdRequest,
        sender: Sender<Result<ComIdResponse, Error>>,
        span: Span,
    ) {
        self.requests.push_back(RequestRecord { request, sender, span });
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, result: Result<(), Error>) {
        if let Some(RequestSendingRecord { sender, span }) = self.request_sending.take() {
            let deadline = time + self.timeout;
            match result {
                Ok(_) => {
                    self.request_receiving = Some(RequestReceivingRecord { sent_at: time, deadline, sender, span })
                }
                Err(err) => drop(sender.send(Err(err))),
            }
        }
    }

    /// Process the response received in the IF-RECV with the span `iface`.
    pub fn handle_iface_recv_done(&mut self, response: ComIdResponse, iface: &Span) {
        if let Some(RequestReceivingRecord { sender, span, .. }) = self.request_receiving.take() {
            link_both_ways(iface, &span);
            let _ = sender.send(Ok(response));
        }
    }

    /// The span of the request awaiting a response.
    pub fn next_recv_span(&self) -> Option<(Instant, &Span)> {
        self.request_receiving.as_ref().map(|record| (record.sent_at, &record.span))
    }

    pub fn handle_reset(&mut self) {
        for RequestRecord { sender, .. } in self.requests.drain(..) {
            let _ = sender.send(Err(Error::Aborted));
        }
        if let Some(RequestSendingRecord { sender, .. }) = self.request_sending.take() {
            let _ = sender.send(Err(Error::Aborted));
        }
        if let Some(RequestReceivingRecord { sender, .. }) = self.request_receiving.take() {
            let _ = sender.send(Err(Error::Aborted));
        }
        *self = Self::new(self.timeout);
    }

    pub fn poll_action(&mut self, time: Instant) -> ComIdAction {
        // Remove timed out entires.
        while let Some(record) = self.request_receiving.take_if(|record| record.deadline < time) {
            let _ = record.sender.send(Err(Error::TimedOut));
        }

        // Get next action.
        if self.request_sending.is_none()
            && self.request_receiving.is_none()
            && let Some(RequestRecord { request, sender, span }) = self.requests.pop_front()
        {
            self.request_sending = Some(RequestSendingRecord { sender, span: span.clone() });
            ComIdAction::Send(request, span)
        } else if let Some(deadline) = self.request_receiving.as_ref().map(|record| record.deadline) {
            ComIdAction::Sleep { until: deadline }
        } else {
            ComIdAction::None
        }
    }
}

#[derive(Debug)]
struct RequestRecord {
    request: ComIdRequest,
    sender: Sender<Result<ComIdResponse, Error>>,
    span: Span,
}

#[derive(Debug)]
struct RequestSendingRecord {
    sender: Sender<Result<ComIdResponse, Error>>,
    span: Span,
}

#[derive(Debug)]
struct RequestReceivingRecord {
    /// The time when the request was sent.
    sent_at: Instant,
    /// The time when the request times out.
    deadline: Instant,
    sender: Sender<Result<ComIdResponse, Error>>,
    span: Span,
}

pub enum ComIdAction {
    None,
    Sleep {
        until: Instant,
    },
    /// The request is paired with its span.
    Send(ComIdRequest, Span),
}
