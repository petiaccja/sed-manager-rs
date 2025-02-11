use tokio::sync::oneshot;

pub struct Tracked<Item, Status> {
    pub item: Item,
    pub promises: Vec<oneshot::Sender<Status>>,
}

impl<Item, Status> Tracked<Item, Status> {
    pub fn new(item: Item, promises: Vec<oneshot::Sender<Status>>) -> Self {
        Self { item, promises }
    }

    pub fn get(&self) -> &Item {
        &self.item
    }

    pub fn get_mut(&mut self) -> &mut Item {
        &mut self.item
    }

    pub fn get_promises(&self) -> &Vec<oneshot::Sender<Status>> {
        &self.promises
    }

    pub fn get_mut_promises(&mut self) -> &mut Vec<oneshot::Sender<Status>> {
        &mut self.promises
    }

    pub fn map<Mapped>(self, f: impl FnOnce(Item) -> Mapped) -> Tracked<Mapped, Status> {
        Tracked { item: f(self.item), promises: self.promises }
    }

    pub fn try_map<Mapped>(self, f: impl FnOnce(Item) -> Result<Mapped, Status>) -> Option<Tracked<Mapped, Status>>
    where
        Status: Clone,
    {
        match f(self.item) {
            Ok(item) => Some(Tracked { item, promises: self.promises }),
            Err(status) => {
                self.promises.into_iter().for_each(|promise| drop(promise.send(status.clone())));
                None
            }
        }
    }

    pub fn merge<Other, Merged>(
        self,
        mut other: Tracked<Other, Status>,
        f: impl FnOnce(Item, Other) -> Merged,
    ) -> Tracked<Merged, Status> {
        let mut promises = self.promises;
        promises.append(&mut other.promises);
        Tracked { item: f(self.item, other.item), promises }
    }

    pub fn close(self, status: Status)
    where
        Status: Clone,
    {
        self.promises.into_iter().for_each(|promise| drop(promise.send(status.clone())));
    }
}
