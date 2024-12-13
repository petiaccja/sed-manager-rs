use std::ops::DerefMut;

pub trait AsyncExchange<Item> {
    async fn exchange(&self, new: Item) -> Item;
}

impl<T> AsyncExchange<T> for tokio::sync::Mutex<T> {
    async fn exchange(&self, new: T) -> T {
        std::mem::replace(self.lock().await.deref_mut(), new)
    }
}
