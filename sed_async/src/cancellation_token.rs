use std::task::Poll;

use pin_project::pin_project;

pub fn cancel_channel() -> (CancelToken, CancelSender) {
    let (tx, rx) = oneshot::async_channel();
    (CancelToken { rx }, CancelSender { tx })
}

#[derive(Debug)]
#[pin_project]
pub struct CancelToken {
    #[pin]
    rx: oneshot::AsyncReceiver<()>,
}

impl Future for CancelToken {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut projected = self.project();
        let rx = &mut projected.rx;
        match rx.as_mut().poll(cx) {
            // The channel has been signaled normally => the token is cancelled.
            Poll::Ready(Ok(_)) => Poll::Ready(()),
            // The channel was dropped => the token wasn't and will never be cancelled.
            // The channel should not register the waker in this case, and this future
            // should thus never be polled again.
            Poll::Ready(Err(_)) => Poll::Pending,
            // The channel has not been signaled => pending.
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct CancelSender {
    #[pin]
    tx: oneshot::Sender<()>,
}

impl CancelSender {
    pub fn cancel(self) {
        let _ = self.tx.send(());
    }
}
