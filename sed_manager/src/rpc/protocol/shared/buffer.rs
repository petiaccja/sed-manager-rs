use core::task::Poll;
use std::collections::VecDeque;

use super::pipe::{SinkPipe, SourcePipe};

pub struct Buffer<Item> {
    items: VecDeque<Item>,
    closed: bool,
}

impl<Item> Buffer<Item> {
    pub fn new() -> Self {
        Self { items: VecDeque::new(), closed: false }
    }
}

impl<Item> SourcePipe<Item> for Buffer<Item> {
    fn pop(&mut self) -> Poll<Option<Item>> {
        if let Some(item) = self.items.pop_front() {
            Poll::Ready(Some(item))
        } else if !self.is_closed() {
            Poll::Pending
        } else {
            Poll::Ready(None)
        }
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    fn is_closed(&self) -> bool {
        self.closed
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn is_done(&self) -> bool {
        self.is_closed() && self.is_empty()
    }
}

impl<Item> SinkPipe<Item> for Buffer<Item> {
    fn close(&mut self) {
        self.closed = true;
    }

    fn push(&mut self, item: Item) {
        if !self.closed {
            self.items.push_back(item);
        }
    }
}
