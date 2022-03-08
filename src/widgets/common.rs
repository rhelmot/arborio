use std::str::FromStr;
use vizia::*;

pub fn tweak_attr_text<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Data,
    F: 'static + Send + Sync + Fn(&mut Context, <L as Lens>::Target) -> bool,
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Textbox::new(cx, lens).on_edit(move |cx, value| {
            if let Ok(parsed) = value.parse() {
                if setter(cx, parsed) {
                    cx.current.toggle_class(cx, "validation_error", false);
                } else {
                    cx.current.toggle_class(cx, "validation_error", true);
                }
            } else {
                cx.current.toggle_class(cx, "validation_error", true);
            }
        });
    });
}

pub fn tweak_attr_check<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens<Target = bool>,
    F: 'static + Send + Sync + Copy + Fn(&mut Context, bool),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Binding::new(cx, lens, move |cx, lens| {
            Checkbox::new(cx, lens.clone()).on_toggle(move |cx| {
                setter(cx, !*lens.get(cx));
            });
        });
    });
}
