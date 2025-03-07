use tokio::sync::oneshot;

use crate::rpc::protocol::shared::pipe::SourcePipe;

pub type Sender<Item> = oneshot::Sender<Item>;

pub fn is_done<Item>(sender: &dyn SourcePipe<Sender<Item>>, _item: &dyn SourcePipe<Item>) -> bool {
    sender.is_done()
}

pub fn is_aborted<Item>(sender: &dyn SourcePipe<Sender<Item>>, item: &dyn SourcePipe<Item>) -> bool {
    !sender.is_done() && item.is_done()
}

pub fn commit<Item>(sender: &mut dyn SourcePipe<Sender<Item>>, item: &mut dyn SourcePipe<Item>) -> usize {
    use core::task::Poll::*;

    let mut committed = 0;
    while !sender.is_empty() {
        let (sender, item) = (sender.pop(), item.pop());
        match (sender, item) {
            (Ready(Some(sender)), Ready(Some(item))) => {
                committed += 1;
                drop(sender.send(item));
            }
            (_, Ready(Some(_))) => panic!("desynchronized"),
            _ => (),
        }
    }
    committed
}
