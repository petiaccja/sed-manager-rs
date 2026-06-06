use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use oneshot::Sender;
use sed_packet::com_id::{ComIdRequest, ComIdResponse};

use crate::Error;

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

    pub fn poll_requests(&mut self) -> Option<ComIdRequest> {
        if self.request_sending.is_none() && self.request_receiving.is_none() {
            self.requests.pop_front().map(|RequestRecord { request, sender }| {
                self.request_sending = Some(RequestSendingRecord { sender });
                request
            })
        } else {
            None
        }
    }

    pub fn poll_timeout(&self) -> Option<Instant> {
        self.request_receiving.as_ref().map(|record| record.deadline)
    }

    pub fn notify_time(&mut self, time: Instant) {
        while let Some(record) = self.request_receiving.take_if(|record| record.deadline < time) {
            let _ = record.sender.send(Err(Error::TimedOut));
        }
    }
}

struct RequestRecord {
    request: ComIdRequest,
    sender: Sender<Result<ComIdResponse, Error>>,
}

struct RequestSendingRecord {
    sender: Sender<Result<ComIdResponse, Error>>,
}

struct RequestReceivingRecord {
    deadline: Instant,
    sender: Sender<Result<ComIdResponse, Error>>,
}
