use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::com_id::{ComIdRequest, ComIdResponse};

use crate::Error;

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

    pub fn handle_com_request(&mut self, request: ComIdRequest, sender: Sender<Result<ComIdResponse, Error>>) {
        self.requests.push_back(RequestRecord { request, sender });
    }

    pub fn handle_iface_send_done(&mut self, time: Instant, result: Result<(), Error>) {
        if let Some(RequestSendingRecord { sender }) = self.request_sending.take() {
            let deadline = time + self.timeout;
            match result {
                Ok(_) => self.request_receiving = Some(RequestReceivingRecord { deadline, sender }),
                Err(err) => drop(sender.send(Err(err))),
            }
        }
    }

    pub fn handle_iface_recv_done(&mut self, response: ComIdResponse) {
        if let Some(RequestReceivingRecord { sender, .. }) = self.request_receiving.take() {
            let _ = sender.send(Ok(response));
        }
    }

    pub fn poll_action(&mut self, time: Instant) -> ComIdAction {
        // Remove timed out entires.
        while let Some(record) = self.request_receiving.take_if(|record| record.deadline < time) {
            let _ = record.sender.send(Err(Error::TimedOut));
        }

        // Get next action.
        if self.request_sending.is_none()
            && self.request_receiving.is_none()
            && let Some(RequestRecord { request, sender }) = self.requests.pop_front()
        {
            self.request_sending = Some(RequestSendingRecord { sender });
            ComIdAction::Send(request)
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
}

#[derive(Debug)]
struct RequestSendingRecord {
    sender: Sender<Result<ComIdResponse, Error>>,
}

#[derive(Debug)]
struct RequestReceivingRecord {
    deadline: Instant,
    sender: Sender<Result<ComIdResponse, Error>>,
}

pub enum ComIdAction {
    None,
    Sleep { until: Instant },
    Send(ComIdRequest),
}
