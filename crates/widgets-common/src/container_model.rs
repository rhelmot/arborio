use arborio_utils::vizia::prelude::*;
use std::cell::RefCell;

#[derive(Debug, Clone, Lens)]
pub struct ModelContainer<T: 'static + Clone + Send + Sync> {
    pub val: T,
}

#[derive(Debug)]
pub enum ModelEvent<T> {
    Set(RefCell<Option<T>>), // only an option so the data can be taken out
}

impl<T: 'static + Clone + Send + Sync> Model for ModelContainer<T> {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|msg, _| {
            let ModelEvent::Set(msg) = msg;
            if let Some(v) = msg.take() {
                self.val = v;
            }
        });
    }
}
