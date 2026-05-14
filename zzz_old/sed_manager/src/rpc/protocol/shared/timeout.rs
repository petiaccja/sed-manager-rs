//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::task::Poll::*;
use core::time::Duration;
use std::time::Instant;

use super::pipe::{SinkPipe, SourcePipe};

pub struct Timeout {
    timeout: Duration,
    last: Instant,
}

impl Timeout {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout, last: Instant::now() }
    }

    pub fn reset(&mut self) {
        self.last = Instant::now();
    }

    pub fn update<Item>(
        &mut self,
        input: &mut dyn SourcePipe<Item>,
        output: &mut dyn SinkPipe<Item>,
        error: Option<impl FnOnce() -> Item>,
    ) {
        while let Ready(Some(item)) = input.pop() {
            output.push(item);
        }
        if self.last.elapsed() > self.timeout {
            if let Some(error) = error {
                output.push(error());
            }
            output.close();
        }
        if input.is_done() {
            output.close();
        }
    }
}
