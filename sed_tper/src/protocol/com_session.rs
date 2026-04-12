use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use sed_spec::methods::Properties;
use tracing::{Span, trace};

use crate::{
    error::Error,
    protocol::{
        message::{ComResponseReceived, ComResult, Message, SendComRequest, SendComRequestDone},
        protocol::{Address, Context},
    },
};

const TIMEOUT: Duration = Properties::ASSUMED.def_trans_timeout;

#[derive(Debug)]
pub struct ComSession {
    send_queue: VecDeque<SendComRequest>,
    state: State,
}

#[derive(Debug)]
enum State {
    Idle,
    AwaitingSend,
    AwaitingReceipt { channel: oneshot::Sender<ComResult>, span: Span, deadline: Instant },
}

impl ComSession {
    const ADDRESS: Address = Address::ComSession;

    pub fn new() -> Self {
        Self { send_queue: VecDeque::new(), state: State::Idle }
    }

    pub fn send_com_request(&mut self, context: Context, message: SendComRequest) {
        trace!(parent: &message.span, "request to send received");
        match &self.state {
            State::Idle => {
                context.send(Address::DeviceSession, Message::SendComRequest(message));
                self.state = State::AwaitingSend;
            }
            _ => {
                self.send_queue.push_back(message);
            }
        };
    }

    pub fn send_com_request_done(
        &mut self,
        context: Context,
        SendComRequestDone { status, channel, span }: SendComRequestDone,
    ) {
        if let Err(error) = status {
            let _ = channel.send((Err(error), span));
            self.state = State::Idle;
        } else {
            trace!(parent: &span, "request sent to interface");
            let deadline = Instant::now() + TIMEOUT;
            context.send_timeout(Self::ADDRESS, deadline);
            self.state = State::AwaitingReceipt { channel, span, deadline };
        }
    }

    pub fn timeout(&mut self, time: Instant) {
        self.state = match core::mem::replace(&mut self.state, State::Idle) {
            State::Idle => State::Idle,
            State::AwaitingSend => State::AwaitingSend,
            State::AwaitingReceipt { channel, span, deadline } => {
                if deadline <= time {
                    let _ = channel.send((Err(Error::TimedOut), span));
                }
                State::Idle
            }
        }
    }

    pub fn com_response_received(&mut self, ComResponseReceived { response }: ComResponseReceived) {
        match core::mem::replace(&mut self.state, State::Idle) {
            State::Idle => (),
            State::AwaitingSend => (),
            State::AwaitingReceipt { channel, span, .. } => {
                let _ = channel.send((Ok(response), span));
            }
        }
    }
}
