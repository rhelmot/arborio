use arborio_utils::vizia::prelude::*;
use std::str::FromStr;

pub fn validator_box<L, F1, F2>(cx: &mut Context, lens: L, setter: F1, set_valid: F2)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Eq + Clone,
    F1: 'static + Send + Sync + Fn(&mut EventContext, <L as Lens>::Target) -> bool,
    F2: 'static + Send + Sync + Fn(&mut EventContext, bool),
{
    Textbox::new(cx, lens).on_edit(move |cx, value| {
        if let Ok(parsed) = value.parse() {
            if setter(cx, parsed) {
                set_valid(cx, true);
            } else {
                set_valid(cx, false);
            }
        } else {
            set_valid(cx, false);
        }
    });
}
