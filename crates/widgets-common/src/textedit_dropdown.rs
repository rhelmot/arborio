use arborio_utils::vizia::fonts::icons_names::DOWN;
use arborio_utils::vizia::prelude::*;
use std::marker::PhantomData;

pub struct TextboxDropdown<T> {
    t: PhantomData<T>,
}

impl<T> TextboxDropdown<T> {
    pub fn new<L, LL, F>(cx: &mut Context, lens: L, options: LL, callback: F) -> Handle<'_, Self>
    where
        T: ToString + Data,
        L: Lens<Target = T>,
        LL: Lens<Target = Vec<T>>,
        <LL as Lens>::Source: Model,
        F: 'static + Clone + Send + Sync + Fn(&mut EventContext, String),
    {
        Self {
            t: Default::default(),
        }
        .build(cx, move |cx| {
            let callback2 = callback.clone();
            Dropdown::new(
                cx,
                move |cx| {
                    let callback = callback2.clone();
                    let lens = lens.clone();
                    HStack::new(cx, move |cx| {
                        Textbox::new(cx, lens.clone()).on_edit(move |cx, value| {
                            callback(cx, value);
                        });
                        Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                    })
                },
                move |cx| {
                    let callback = callback.clone();
                    List::new(cx, options.clone(), move |cx, _idx, lens| {
                        let callback = callback.clone();
                        Label::new(cx, lens.clone())
                            .class("dropdown_element")
                            .on_press(move |cx| {
                                let value = lens.get(cx).to_string();
                                callback.clone()(cx, value);
                                cx.emit(PopupEvent::Close);
                            });
                    });
                },
            );
        })
    }
}

impl<T: 'static> View for TextboxDropdown<T> {
    fn element(&self) -> Option<&'static str> {
        Some("textbox_dropdown")
    }
}
