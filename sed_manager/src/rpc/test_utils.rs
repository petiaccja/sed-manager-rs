#!cfg[test]

use std::future::Future;

use tokio::task::yield_now;

use super::pipeline::{connect, PullInput, PullOutput, PushInput, PushOutput, Receive};

pub fn run_async_test<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
    rt.block_on(future)
}

pub async fn collect_pull<T>(mut rx: PullInput<T>) -> Vec<T> {
    let mut values = Vec::new();
    while let Some(value) = rx.recv().await {
        values.push(value);
    }
    values
}

pub async fn collect_push<T>(mut rx: PushInput<T>) -> Vec<T> {
    let mut values = Vec::new();
    while let Some(value) = rx.recv().await {
        values.push(value);
    }
    values
}

pub async fn try_collect_pull<T>(out: &mut Vec<T>, rx: &mut PullInput<T>) {
    while let Some(value) = rx.try_recv() {
        out.push(value);
        yield_now().await;
    }
}

pub async fn try_collect_push<T>(out: &mut Vec<T>, rx: &mut PushInput<T>) {
    while let Some(value) = rx.try_recv() {
        out.push(value);
        yield_now().await;
    }
}

pub fn create_source_pull<T>(rx: &mut PullInput<T>) -> PullOutput<T> {
    let mut tx = PullOutput::new();
    connect(&mut tx, rx);
    tx
}

pub fn create_sink_pull<T>(tx: &mut PullOutput<T>) -> PullInput<T> {
    let mut rx = PullInput::new();
    connect(tx, &mut rx);
    rx
}

pub fn create_source_push<T>(rx: &mut PushInput<T>) -> PushOutput<T> {
    let mut tx = PushOutput::new();
    connect(&mut tx, rx);
    tx
}

pub fn create_sink_push<T>(tx: &mut PushOutput<T>) -> PushInput<T> {
    let mut rx = PushInput::new();
    connect(tx, &mut rx);
    rx
}
