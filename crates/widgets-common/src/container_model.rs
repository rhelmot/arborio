use arborio_utils::vizia::prelude::*;

#[derive(Debug, Clone, Lens, Setter)]
pub struct ModelContainer<T: 'static + Clone + Send + Sync> {
    pub val: T,
}

impl<T: 'static + Clone + Send + Sync> Model for ModelContainer<T> {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        if let Some(msg) = event.take::<ModelContainerSetter<T>>() {
            msg.apply(self);
        }
    }
}
