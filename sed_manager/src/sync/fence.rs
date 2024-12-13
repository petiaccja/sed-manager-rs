use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use tokio::sync::watch;

const CLOSED: u64 = u64::MAX;
const LOCKED: u64 = u64::MAX - 1;

pub struct Fence {
    value: AtomicU64,
    tx: watch::Sender<u64>,
    _rx: watch::Receiver<u64>, // This receiver is just to keep the channel alive.
}

impl Fence {
    pub fn new() -> Self {
        let value = AtomicU64::new(0);
        let (tx, _rx) = watch::channel(value.load(Ordering::Relaxed));
        Self { value, tx, _rx }
    }

    pub fn signal(&self, value: u64) {
        assert!(value != LOCKED && value != CLOSED, "value is reserved");
        // This is a bit of a spin-lock here, but tokio senders likely also have a spinlock
        // so this should not really be worse for blocking on async contexts.
        let current = loop {
            let current = self.value.swap(LOCKED, Ordering::Acquire);
            if current != LOCKED {
                break current;
            };
        };
        if value > current {
            self.value.swap(value, Ordering::Release);
            let _ = self.tx.send(value);
        } else {
            self.value.swap(current, Ordering::Release);
        }
    }

    pub fn close(&self) {
        self.tx.send(CLOSED).unwrap();
    }

    pub async fn wait(&self, value: u64) -> Result<u64, ()> {
        let mut rx = self.tx.subscribe();
        rx.mark_changed();
        loop {
            rx.changed().await.unwrap();
            let current = *rx.borrow_and_update();
            if current == CLOSED {
                return Err(());
            } else if current >= value {
                return Ok(current);
            };
        }
    }
}
