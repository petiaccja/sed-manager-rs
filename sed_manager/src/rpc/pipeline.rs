use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;

pub trait Connect {
    type Pair;
    fn connect(&mut self, pair: &mut Self::Pair);
    fn disconnect(&mut self);
}

pub trait Receive {
    type Item;
    async fn recv(&mut self) -> Option<Self::Item>;
    fn try_recv(&mut self) -> Option<Self::Item>;
}

pub trait UnbufferedSend {
    type Item;
    async fn send(&mut self, item: Self::Item) -> Result<(), mpsc::error::SendError<Self::Item>>;
}

pub trait BufferedSend {
    type Item;
    fn send(&mut self, item: Self::Item) -> Result<(), mpsc::error::SendError<Self::Item>>;
}

pub trait Process: Send + Sync {
    type Output: Send + Sync;
    type Error: Send + Sync;

    fn update(&mut self) -> impl Future<Output = Result<Option<Self::Output>, Self::Error>> + Send;

    fn run(mut self) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send
    where
        Self: Sized,
    {
        async move {
            loop {
                match self.update().await {
                    Ok(Some(output)) => break Ok(output),
                    Err(err) => break Err(err),
                    _ => (),
                }
            }
        }
    }
}

pub trait LinearProcess {
    type Input: Connect<Pair = Self::Output>;
    type Output: Connect<Pair = Self::Input>;
    fn input_mut(&mut self) -> &mut Self::Input;
    fn output_mut(&mut self) -> &mut Self::Output;
}

pub trait BoxedProcess: Send + Sync {
    type Output: Send + Sync;
    type Error: Send + Sync;

    fn run(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;
}

pub struct PullInput<Item> {
    receiver: Option<mpsc::Receiver<Item>>,
}

#[derive(Clone)]
pub struct PullOutput<Item> {
    sender: Option<mpsc::Sender<Item>>,
}

pub struct PushInput<Item> {
    receiver: Option<mpsc::UnboundedReceiver<Item>>,
}

#[derive(Clone)]
pub struct PushOutput<Item> {
    sender: Option<mpsc::UnboundedSender<Item>>,
}

impl<Item> PullInput<Item> {
    pub fn new() -> Self {
        Self { receiver: None }
    }
}

impl<Item> PullOutput<Item> {
    pub fn new() -> Self {
        Self { sender: None }
    }
}

impl<Item> PushInput<Item> {
    pub fn new() -> Self {
        Self { receiver: None }
    }
}

impl<T> PushOutput<T> {
    pub fn new() -> Self {
        Self { sender: None }
    }
}

impl<Item> Receive for PullInput<Item> {
    type Item = Item;

    async fn recv(&mut self) -> Option<Item> {
        if let Some(receiver) = self.receiver.as_mut() {
            receiver.recv().await
        } else {
            None
        }
    }

    fn try_recv(&mut self) -> Option<Item> {
        if let Some(receiver) = self.receiver.as_mut() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
}

impl<Item> UnbufferedSend for PullOutput<Item> {
    type Item = Item;
    async fn send(&mut self, item: Self::Item) -> Result<(), mpsc::error::SendError<Item>> {
        if let Some(sender) = self.sender.as_mut() {
            sender.send(item).await
        } else {
            Err(mpsc::error::SendError::<Item>(item))
        }
    }
}

impl<Item> Receive for PushInput<Item> {
    type Item = Item;

    async fn recv(&mut self) -> Option<Item> {
        if let Some(receiver) = self.receiver.as_mut() {
            receiver.recv().await
        } else {
            None
        }
    }

    fn try_recv(&mut self) -> Option<Item> {
        if let Some(receiver) = self.receiver.as_mut() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
}

impl<Item> BufferedSend for PushOutput<Item> {
    type Item = Item;

    fn send(&mut self, value: Self::Item) -> Result<(), mpsc::error::SendError<Self::Item>> {
        if let Some(sender) = self.sender.as_mut() {
            sender.send(value)
        } else {
            Err(mpsc::error::SendError::<Self::Item>(value))
        }
    }
}

impl<Item> Connect for PullInput<Item> {
    type Pair = PullOutput<Item>;
    fn connect(&mut self, pair: &mut Self::Pair) {
        let (tx, rx) = mpsc::channel::<Item>(1);
        self.receiver = Some(rx);
        pair.sender = Some(tx);
    }
    fn disconnect(&mut self) {
        self.receiver = None;
    }
}

impl<Item> Connect for PullOutput<Item> {
    type Pair = PullInput<Item>;
    fn connect(&mut self, pair: &mut Self::Pair) {
        pair.connect(self);
    }
    fn disconnect(&mut self) {
        self.sender = None;
    }
}

impl<Item> Connect for PushInput<Item> {
    type Pair = PushOutput<Item>;
    fn connect(&mut self, pair: &mut Self::Pair) {
        let (tx, rx) = mpsc::unbounded_channel::<Item>();
        self.receiver = Some(rx);
        pair.sender = Some(tx);
    }
    fn disconnect(&mut self) {
        self.receiver = None;
    }
}

impl<Item> Connect for PushOutput<Item> {
    type Pair = PushInput<Item>;
    fn connect(&mut self, pair: &mut Self::Pair) {
        pair.connect(self);
    }
    fn disconnect(&mut self) {
        self.sender = None;
    }
}

impl<P: Process + 'static> BoxedProcess for P {
    type Output = P::Output;
    type Error = P::Error;
    fn run(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>> {
        Box::pin(<Self as Process>::run(*self))
    }
}

pub fn connect<Port: Connect>(port: &mut Port, pair: &mut Port::Pair) {
    port.connect(pair);
}

pub fn spawn<T: Process + Send + 'static>(
    process: T,
) -> impl Future<Output = Result<T::Output, T::Error>> + Sync + Send {
    let handle = tokio::spawn(process.run());
    async move {
        match handle.await {
            Ok(result) => result,
            Err(err) => {
                if err.is_cancelled() {
                    panic!("task was not supposed to be cancelled");
                } else {
                    std::panic::resume_unwind(err.into_panic());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rpc::test_utils::run_async_test;

    use super::*;

    struct Counter {
        numbers: PullOutput<u32>,
        current: u32,
        end: u32,
    }

    struct Sum {
        numbers: PullInput<u32>,
        sums: PullOutput<u32>,
        current: u32,
    }

    struct Collect {
        sums: PullInput<u32>,
        output: Vec<u32>,
    }

    impl Counter {
        fn new(range: std::ops::Range<u32>) -> Self {
            Self { numbers: PullOutput::new(), current: range.start, end: range.end }
        }
    }

    impl Sum {
        fn new() -> Self {
            Self { numbers: PullInput::new(), sums: PullOutput::new(), current: 0 }
        }
    }

    impl Collect {
        fn new() -> Self {
            Self { sums: PullInput::new(), output: vec![] }
        }
    }

    impl Process for Counter {
        type Output = ();
        type Error = ();
        async fn update(&mut self) -> Result<Option<()>, Self::Error> {
            let _ = self.numbers.send(self.current).await;
            self.current += 1;
            if self.current < self.end {
                Ok(None)
            } else {
                Ok(Some(()))
            }
        }
    }

    impl Process for Sum {
        type Output = ();
        type Error = ();
        async fn update(&mut self) -> Result<Option<()>, Self::Error> {
            if let Some(number) = self.numbers.recv().await {
                self.current += number;
                let _ = self.sums.send(self.current).await;
                Ok(None)
            } else {
                Ok(Some(()))
            }
        }
    }

    impl Process for Collect {
        type Output = Vec<u32>;
        type Error = ();
        async fn update(&mut self) -> Result<Option<Self::Output>, Self::Error> {
            if let Some(sum) = self.sums.recv().await {
                self.output.push(sum);
                Ok(None)
            } else {
                Ok(Some(std::mem::replace(&mut self.output, vec![])))
            }
        }
    }

    #[test]
    fn sum() {
        run_async_test(async {
            let mut counter = Counter::new(0..6);
            let mut sum = Sum::new();
            let mut collect = Collect::new();
            connect(&mut counter.numbers, &mut sum.numbers);
            connect(&mut sum.sums, &mut collect.sums);

            let counter_task = spawn(counter);
            let sum_task = spawn(sum);
            let collect_task = spawn(collect);

            counter_task.await.unwrap();
            sum_task.await.unwrap();
            let result = collect_task.await.unwrap();
            assert_eq!(result, [0, 1, 3, 6, 10, 15]);
        });
    }
}
