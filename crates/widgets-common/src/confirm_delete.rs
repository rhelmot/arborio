use arborio_state::lenses::StaticerLens;
use arborio_utils::vizia::prelude::*;

#[derive(Debug, Clone, Lens)]
struct DeleteState {
    started: bool,
    validated: bool,
}

#[derive(Debug)]
enum DeleteEvent {
    Start,
    Cancel,
    Validate(bool),
}

impl Model for DeleteState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|msg, _| match msg {
            DeleteEvent::Start => self.started = true,
            DeleteEvent::Cancel => self.started = false,
            DeleteEvent::Validate(b) => self.validated = *b,
        });
    }
}

pub fn deleter<F1, F2>(
    cx: &mut Context,
    btn_text: &'static str,
    confirm_message: &'static str,
    validate_text: F1,
    do_delete: F2,
) where
    F1: 'static + Send + Sync + Clone + Fn(&mut EventContext, &str) -> bool,
    F2: 'static + Send + Sync + Clone + Fn(&mut EventContext),
{
    DeleteState {
        started: false,
        validated: false,
    }
    .build(cx);

    Binding::new(cx, DeleteState::started, move |cx, started| {
        let validate_text = validate_text.clone();
        let do_delete = do_delete.clone();
        if started.get(cx) {
            VStack::new(cx, move |cx| {
                Label::new(cx, confirm_message);
                HStack::new(cx, move |cx| {
                    Textbox::new(cx, StaticerLens::new("")).on_edit(move |cx, value| {
                        let validated = validate_text(cx, &value);
                        cx.emit(DeleteEvent::Validate(validated));
                    });
                    Label::new(cx, btn_text)
                        .class("btn_highlight")
                        .class("danger")
                        .on_press(move |cx| {
                            if DeleteState::validated.get(cx) {
                                do_delete(cx.as_mut());
                            }
                        });
                    Label::new(cx, "Cancel")
                        .class("btn_highlight")
                        .on_press(move |cx| {
                            cx.emit(DeleteEvent::Cancel);
                        });
                });
            })
            .class("delete_confirm_controls");
        } else {
            Label::new(cx, btn_text)
                .class("btn_highlight")
                .class("danger")
                .on_press(move |cx| {
                    cx.emit(DeleteEvent::Start);
                });
        }
    });
}
