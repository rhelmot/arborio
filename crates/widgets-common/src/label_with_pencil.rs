use crate::container_model::{ModelContainer, ModelContainerSetter};
use crate::validator_box::validator_box;
use arborio_utils::vizia::prelude::*;
use std::str::FromStr;

#[derive(Debug, Lens)]
struct EditingState {
    editing: bool,
    valid: bool,
}

#[derive(Debug)]
enum EditingStateEvent {
    Start,
    End,
    Valid(bool),
}

impl Model for EditingState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|msg, _| match msg {
            EditingStateEvent::End => self.editing = false,
            EditingStateEvent::Start => self.editing = true,
            EditingStateEvent::Valid(b) => self.valid = *b,
        });
    }
}

// should editable be a lens?
pub fn label_with_pencil<L, F1, F2>(
    cx: &mut Context,
    lens: L,
    validator: F1,
    setter: F2,
    editable: bool,
) -> Handle<impl View>
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Eq + Clone + Send + Sync,
    F1: 'static + Send + Sync + Clone + Fn(&mut EventContext, &<L as Lens>::Target) -> bool,
    F2: 'static + Send + Sync + Clone + Fn(&mut EventContext, <L as Lens>::Target),
{
    HStack::new(cx, move |cx| {
        EditingState {
            editing: false,
            valid: true,
        }
        .build(cx);

        Binding::new(cx, EditingState::editing, move |cx, editing_lens| {
            let setter = setter.clone();
            let validator = validator.clone();
            let editing = editing_lens.get(cx);
            let lens = lens.clone();
            if editing {
                ModelContainer { val: lens.get(cx) }.build(cx);
                Label::new(cx, "\u{e5ca}")
                    .font("material")
                    .class("btn_highlight")
                    .class("pencil_icon")
                    .on_press(move |cx| {
                        if EditingState::valid.get(cx) {
                            let value = ModelContainer::val.get(cx);
                            setter(cx.as_mut(), value);
                            cx.emit(EditingStateEvent::End);
                        }
                    })
                    .bind(EditingState::valid, move |handle, lens| {
                        let val = lens.get(handle.cx);
                        handle.toggle_class("disabled", val);
                    });
                Label::new(cx, "\u{e5cd}")
                    .font("material")
                    .class("btn_highlight")
                    .class("pencil_icon")
                    .on_press(|cx| {
                        cx.emit(EditingStateEvent::End);
                    });
                validator_box(
                    cx,
                    ModelContainer::val,
                    move |cx, val| {
                        if validator(cx, &val) {
                            cx.emit(ModelContainerSetter::Val(val));
                            true
                        } else {
                            false
                        }
                    },
                    move |cx, valid| {
                        cx.toggle_class("validation_error", !valid);
                        cx.emit(EditingStateEvent::Valid(valid));
                    },
                );
            } else {
                if editable {
                    Label::new(cx, "\u{e150}")
                        .font("material")
                        .class("btn_highlight")
                        .class("pencil_icon")
                        .on_press(move |cx| cx.emit(EditingStateEvent::Start));
                }
                Label::new(cx, lens).class("pencilable_label");
            }
        });
    })
}
