use core::task::Poll;

pub trait SourcePipe<Item> {
    fn pop(&mut self) -> Poll<Option<Item>>;
    fn clear(&mut self);
    fn is_closed(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn is_done(&self) -> bool;
}

pub trait SinkPipe<Item> {
    fn push(&mut self, item: Item);
    fn close(&mut self);
}
