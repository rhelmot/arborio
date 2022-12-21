use std::collections::HashMap;
use std::str::FromStr;

use crate::textedit_dropdown::TextboxDropdown;
use crate::validator_box;
use crate::validator_box::validator_box;
use arborio_maploader::map_struct::Attribute;
use arborio_modloader::config::AttributeType;
use arborio_state::lenses::{
    hash_map_nth_key_lens, HashMapIndexWithLens, HashMapLenLens, IsFailedLens,
};
use arborio_utils::vizia::fonts::icons_names::DOWN;
use arborio_utils::vizia::prelude::*;

#[derive(Lens)]
pub struct NewAttributeData {
    name: String,
    ty: AttributeType,
}

impl Model for NewAttributeData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|msg, _| match msg {
            NewAttributeDataEvent::SetName(name) => self.name = name.clone(),
            NewAttributeDataEvent::SetTy(ref ty) => self.ty = *ty,
        });
    }
}

enum NewAttributeDataEvent {
    SetName(String),
    SetTy(AttributeType),
}

pub fn tweak_attr_text<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + PartialEq + Clone,
    F: 'static + Send + Sync + Fn(&mut EventContext, <L as Lens>::Target) -> bool,
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name).class("label");
        validator_box::validator_box(cx, lens, setter, |cx, valid| {
            cx.toggle_class("validation_error", !valid);
        });
    });
}

pub fn tweak_attr_text_dropdown<L, LL, F>(
    cx: &mut Context,
    name: &'static str,
    lens: L,
    options: LL,
    setter: F,
) where
    L: Lens<Target = String>,
    LL: Send + Sync + Lens<Target = Vec<String>>,
    <LL as Lens>::Source: Model,
    F: 'static + Send + Sync + Clone + Fn(&mut EventContext, String),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name).class("label");
        TextboxDropdown::new(cx, lens, options, setter);
    });
}

pub fn tweak_attr_check<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens<Target = bool>,
    F: 'static + Send + Sync + Copy + Fn(&mut EventContext, bool),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name).class("label");
        Binding::new(cx, lens, move |cx, lens| {
            Checkbox::new(cx, lens.clone()).on_toggle(move |cx| {
                setter(cx, !lens.get(cx));
            });
        });
    });
}

pub fn advanced_attrs_editor(
    cx: &mut Context,
    attributes_lens: impl Lens<Target = HashMap<String, Attribute>> + Copy + Send + Sync,
    setter: impl 'static + Clone + Send + Sync + Fn(&mut EventContext, String, Attribute),
    adder: impl 'static + Send + Sync + Fn(&mut EventContext, String, AttributeType),
    remover: impl 'static + Clone + Send + Sync + Fn(&mut EventContext, String),
) {
    Binding::new(
        cx,
        attributes_lens.then(HashMapLenLens::new()),
        move |cx, len| {
            let len = len.get_fallible(cx).unwrap_or(0);
            for i in 0..len {
                let setter = setter.clone();
                let remover = remover.clone();
                let key_lens = attributes_lens.then(hash_map_nth_key_lens(i));
                HStack::new(cx, move |cx| {
                    Label::new(cx, key_lens);

                    let attr_value_lens = HashMapIndexWithLens::new(attributes_lens, key_lens);
                    let s_value_lens = attr_value_lens.then(Attribute::text);
                    let i_value_lens = attr_value_lens.then(Attribute::int);
                    let f_value_lens = attr_value_lens.then(Attribute::float);
                    let b_value_lens = attr_value_lens.then(Attribute::bool);

                    let setter2 = setter.clone();
                    attr_editor(
                        cx,
                        s_value_lens,
                        key_lens,
                        move |cx, key, val| setter2(cx, key, Attribute::Text(val)),
                        false,
                    );
                    let setter2 = setter.clone();
                    attr_editor(
                        cx,
                        i_value_lens,
                        key_lens,
                        move |cx, key, val| setter2(cx, key, Attribute::Int(val)),
                        false,
                    );
                    let setter2 = setter.clone();
                    attr_editor(
                        cx,
                        f_value_lens,
                        key_lens,
                        move |cx, key, val| setter2(cx, key, Attribute::Float(val)),
                        false,
                    );
                    Binding::new(cx, IsFailedLens::new(b_value_lens), move |cx, failed| {
                        if !failed.get(cx) {
                            let setter2 = setter.clone();
                            Checkbox::new(cx, b_value_lens).on_toggle(move |cx| {
                                let b = b_value_lens.get(cx);
                                setter2(cx, key_lens.get(cx), Attribute::Bool(!b));
                            });
                        }
                    });

                    Label::new(cx, "\u{e15b}")
                        .class("icon")
                        .class("remove_btn")
                        .on_press(move |cx| {
                            let keyed = key_lens.get(cx);
                            remover(cx.as_mut(), keyed);
                        });
                });
            }
        },
    );
    HStack::new(cx, move |cx| {
        NewAttributeData {
            name: "".to_string(),
            ty: AttributeType::String,
        }
        .build(cx);
        Textbox::new(cx, NewAttributeData::name).on_edit(|cx, text| {
            cx.emit(NewAttributeDataEvent::SetName(text));
        });
        Dropdown::new(
            cx,
            |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "").bind(NewAttributeData::ty, |handle, ty| {
                        let text = format!("{:?}", ty.get(handle.cx));
                        handle.text(&text);
                    });
                    Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                })
            },
            |cx| {
                VStack::new(cx, |cx| {
                    for ty in [
                        AttributeType::String,
                        AttributeType::Int,
                        AttributeType::Bool,
                        AttributeType::Float,
                    ] {
                        Label::new(cx, &format!("{:?}", ty))
                            .class("dropdown_element")
                            .class("btn_highlight")
                            .on_press(move |cx| {
                                cx.emit(PopupEvent::Close);
                                cx.emit(NewAttributeDataEvent::SetTy(ty));
                            });
                    }
                });
            },
        );
        Label::new(cx, "\u{e145}")
            .class("icon")
            .class("add_btn")
            .on_press(move |cx| {
                let name = NewAttributeData::name.get(cx);
                if !name.is_empty() {
                    let weh = NewAttributeData::ty.get(cx);
                    adder(cx.as_mut(), name, weh);
                    cx.emit(NewAttributeDataEvent::SetName("".to_owned()));
                }
            });
    });
}

pub fn attr_editor<T: ToString + FromStr + PartialEq + Clone>(
    cx: &mut Context,
    lens: impl Lens<Target = T>,
    key: impl Send + Sync + Lens<Target = String>,
    setter: impl 'static + Clone + Send + Sync + Fn(&mut EventContext, String, T),
    force: bool,
) {
    if force {
        attr_editor_inner(cx, lens, key, setter);
    } else {
        Binding::new(cx, IsFailedLens::new(lens.clone()), move |cx, failed| {
            if !failed.get(cx) {
                let key = key.clone();
                let setter = setter.clone();
                let lens = lens.clone();
                attr_editor_inner(cx, lens, key, setter);
            }
        });
    }
}

pub fn attr_editor_inner<T: ToString + FromStr + PartialEq + Clone>(
    cx: &mut Context,
    lens: impl Lens<Target = T>,
    key: impl Send + Sync + Lens<Target = String>,
    setter: impl 'static + Clone + Send + Sync + Fn(&mut EventContext, String, T),
) {
    validator_box(
        cx,
        lens,
        move |cx, value| {
            setter(cx, key.get(cx), value);
            true
        },
        move |cx, valid| cx.toggle_class("validation_error", !valid),
    );
}
