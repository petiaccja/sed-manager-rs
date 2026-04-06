use std::{collections::VecDeque, time::Instant};

use tracing::{Span, trace};

use crate::protocol::{
    messages::{ComIdRequestSent, ComIdResponseReceived, ComResult, SendComIdRequest},
    protocol::{Context, Topic},
};

struct ComSession {
    send_queue: VecDeque<SendComIdRequest>,
    state: State,
}

enum State {
    Ready,
    Pending,
    AwaitingResponse(oneshot::Sender<ComResult>, Span, Instant),
}

impl ComSession {
    fn topic() -> Topic {
        Topic::ComLayer
    }

    fn on_send_request(&mut self, context: &mut Context, message: SendComIdRequest) {
        trace!(parent: &message.span, "request to send received");
        match &self.state {
            State::Ready => {
                context.send(Topic::DeviceLayer, message);
                self.state = State::Pending;
            }
            _ => {
                self.send_queue.push_back(message);
            }
        };
    }

    fn on_request_sent(
        &mut self,
        _context: &mut Context,
        ComIdRequestSent { status, channel, span }: ComIdRequestSent,
    ) {
        if let Some(error) = status {
            let _ = channel.send((Err(error), span));
            self.state = State::Ready;
        } else {
            trace!(parent: &span, "request sent to interface");
            self.state = State::AwaitingResponse(channel, span, Instant::now());
        }
    }

    fn on_receive_response(
        &mut self,
        _context: &mut Context,
        ComIdResponseReceived { response }: ComIdResponseReceived,
    ) {
        match core::mem::replace(&mut self.state, State::Ready) {
            State::Ready => (),
            State::Pending => (),
            State::AwaitingResponse(sender, span, _) => {
                let _ = sender.send((Ok(response), span));
            }
        }
    }
}
