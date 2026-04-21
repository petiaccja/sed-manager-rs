use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use sed_async_runtime::{CancelSender, cancel_channel};
use tracing::{Span, instrument};

use crate::{
    error::Error,
    protocol::{
        message::{ComResponse, ComResponseReceived, Message, SendComRequest, SendComRequestDone},
        protocol::{Address, Context},
    },
};

#[derive(Debug)]
pub struct ComSession {
    timeout: Duration,
    send_queue: VecDeque<SendComRequest>,
    state: State,
}

#[derive(Debug)]
enum State {
    Idle,
    AwaitingSend,
    AwaitingReceipt {
        channel: oneshot::Sender<ComResponse>,
        deadline: Instant,
        span: Span,
        cancel_sender: CancelSender,
    },
}

impl ComSession {
    const ADDRESS: Address = Address::ComSession;

    pub fn new(timeout: Duration) -> Self {
        Self { timeout, send_queue: VecDeque::new(), state: State::Idle }
    }

    #[instrument(level = "debug")]
    pub fn send_com_request(&mut self, context: Context, SendComRequest { request, channel, span }: SendComRequest) {
        Span::current().follows_from(span);
        match &self.state {
            State::Idle => {
                context.send(
                    Address::DeviceSession,
                    Message::SendComRequest(SendComRequest { request, channel, span: Span::current() }),
                );
                self.state = State::AwaitingSend;
            }
            _ => {
                self.send_queue.push_back(SendComRequest { request, channel, span: Span::current() });
            }
        };
    }

    #[instrument(level = "debug")]
    pub fn send_com_request_done(
        &mut self,
        context: Context,
        SendComRequestDone { status, channel, span }: SendComRequestDone,
    ) {
        Span::current().follows_from(span);
        if let Err(error) = status {
            let _ = channel.send(Err(error));
            self.state = State::Idle;
        } else {
            let deadline = Instant::now() + self.timeout;
            let (cancel_token, cancel_sender) = cancel_channel();
            context.send_timeout(Self::ADDRESS, deadline, Some(cancel_token));
            self.state = State::AwaitingReceipt { channel, deadline, span: Span::current(), cancel_sender };
        }
    }

    #[instrument(level = "debug")]
    pub fn timeout(&mut self, time: Instant) {
        self.state = match core::mem::replace(&mut self.state, State::Idle) {
            State::Idle => State::Idle,
            State::AwaitingSend => State::AwaitingSend,
            State::AwaitingReceipt { channel, deadline, span, .. } => {
                Span::current().follows_from(span);
                if deadline <= time {
                    let _ = channel.send(Err(Error::TimedOut));
                }
                State::Idle
            }
        }
    }

    #[instrument(level = "debug")]
    pub fn com_response_received(&mut self, ComResponseReceived { response }: ComResponseReceived) {
        match core::mem::replace(&mut self.state, State::Idle) {
            State::Idle => (),
            State::AwaitingSend => (),
            State::AwaitingReceipt { channel, cancel_sender, span, .. } => {
                Span::current().follows_from(span);
                cancel_sender.cancel();
                let _ = channel.send(Ok(response));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sed_packet::com_id::HandleComIdRequest;
    use sed_packet::com_id::HandleComIdResponse;
    use sed_packet::com_id::HandleComIdResponseParams;
    use sed_packet::com_id::StackResetStatus;
    use tracing::Span;

    use googletest::matchers::*;
    use googletest::prelude::*;

    #[test]
    fn send_com_request() {
        let mut com_session = ComSession::new(Duration::from_millis(1000));
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        let request = HandleComIdRequest::stack_reset(0x12, 0x00);
        com_session
            .send_com_request(context, SendComRequest { request: request.clone(), channel: tx, span: Span::current() });

        let (address, content) = queue.try_recv().unwrap();
        assert_that!(address, eq(&Address::DeviceSession));
        assert_that!(content, field!(&Message::SendComRequest.0, ref field!(SendComRequest.request, eq(&request))));
        assert!(queue.is_empty());
        assert_eq!(rx.has_message(), false);
    }

    #[test]
    fn send_com_request_done_error() {
        let mut com_session = ComSession::new(Duration::from_millis(1000));
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        com_session.send_com_request_done(
            context,
            SendComRequestDone { status: Err(Error::NotSupported), channel: tx, span: Span::current() },
        );

        assert!(queue.is_empty());
        assert_eq!(rx.try_recv(), Ok(Err(Error::NotSupported)));
    }

    #[tokio::test]
    async fn send_com_request_done_timeout() {
        let mut com_session = ComSession::new(Duration::from_millis(0));
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        com_session
            .send_com_request_done(context, SendComRequestDone { status: Ok(()), channel: tx, span: Span::current() });

        tokio::task::yield_now().await; // Let the timeout task run.

        let item = queue.try_recv();
        assert!(matches!(item, Ok((Address::ComSession, Message::Timeout(_)))), "{item:?}");
        assert_eq!(rx.has_message(), false);
    }

    #[tokio::test]
    async fn send_com_request_done_receive() {
        let mut com_session = ComSession::new(Duration::from_millis(u64::MAX));
        let (context, queue) = Context::mock();
        let (tx, rx) = oneshot::channel();
        com_session
            .send_com_request_done(context, SendComRequestDone { status: Ok(()), channel: tx, span: Span::current() });

        let response = HandleComIdResponse {
            com_id: 0,
            com_id_ext: 0,
            params: HandleComIdResponseParams::StackReset {
                available_data_length: 0,
                status: StackResetStatus::Success,
            },
        };

        com_session.com_response_received(ComResponseReceived { response: response.clone() });

        assert!(queue.is_empty());
        assert_eq!(rx.try_recv(), Ok(Ok(response)));
    }
}
