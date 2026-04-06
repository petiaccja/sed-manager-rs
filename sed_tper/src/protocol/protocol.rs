use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    time::Duration,
};

pub struct Protocol {
    context: Context,
}

pub struct Context {
    queue: VecDeque<(Topic, Box<dyn Any>)>,
}

impl Context {
    pub fn send(&mut self, topic: Topic, message: impl Any) {
        self.queue.push_back((topic, Box::new(message) as Box<dyn Any>));
    }
}

pub struct Dispatcher {
    message_handlers: HashMap<Topic, HashMap<Id, Box<dyn MessageHandler>>>,
}

impl Dispatcher {
    pub fn dispatch(&mut self, context: &mut Context, topic: Topic, message: Box<dyn Any>) {
        let Some(topic_handlers) = self.message_handlers.get_mut(&topic) else {
            return;
        };
        let mut message = Some(message);
        for handler in topic_handlers.values_mut() {
            match message {
                Some(some_message) => {
                    message = handler.handle(context, &topic, some_message);
                }
                None => break,
            }
        }
    }
}

pub trait MessageHandler {
    fn handle(&mut self, context: &mut Context, topic: &Topic, message: Box<dyn Any>) -> Option<Box<dyn Any>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Topic {
    ComLayer,
    SessionLayer { tsn: u32, hsn: u32 },
    ManagementLayer,
    DeviceLayer,
    Stack,
}
