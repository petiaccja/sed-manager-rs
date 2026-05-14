//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

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
    loop {
        match (sender.is_empty(), sender.is_closed(), item.is_empty(), item.is_closed()) {
            (false, _, false, _) => (),
            (false, _, true, true) => (),
            _ => break,
        }
        let (sender, item) = (sender.pop(), item.pop());
        match (sender, item) {
            (Ready(Some(sender)), Ready(Some(item))) => {
                committed += 1;
                let _ = sender.send(item);
            }
            (_, Ready(Some(_))) => panic!("desynchronized"),
            _ => (),
        }
    }
    committed
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot::error::TryRecvError;

    use crate::rpc::protocol::shared::{buffer::Buffer, pipe::SinkPipe};

    use super::*;

    #[test]
    fn both_ready() {
        let (tx, mut rx) = oneshot::channel();
        let mut sender = Buffer::new();
        let mut item = Buffer::new();
        sender.push(tx);
        item.push(35);
        assert_eq!(1, commit(&mut sender, &mut item));
        assert_eq!(rx.try_recv(), Ok(35));
    }

    #[test]
    fn sender_ready() {
        let (tx, mut rx) = oneshot::channel::<i32>();
        let mut sender = Buffer::new();
        let mut item = Buffer::new();
        sender.push(tx);
        assert_eq!(0, commit(&mut sender, &mut item));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        assert!(!sender.is_empty());
    }

    #[test]
    fn sender_ready_item_closed() {
        let (tx, mut rx) = oneshot::channel::<i32>();
        let mut sender = Buffer::new();
        let mut item = Buffer::new();
        sender.push(tx);
        item.close();
        assert_eq!(0, commit(&mut sender, &mut item));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Closed));
        assert!(sender.is_empty());
    }

    #[test]
    fn item_ready() {
        let mut sender = Buffer::new();
        let mut item = Buffer::new();
        item.push(35);
        assert_eq!(0, commit(&mut sender, &mut item));
        assert!(!item.is_empty());
    }

    #[test]
    fn nothing_ready() {
        let mut sender = Buffer::new();
        let mut item = Buffer::<i32>::new();
        assert_eq!(0, commit(&mut sender, &mut item));
    }
}
