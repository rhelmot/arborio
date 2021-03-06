use crate::celeste_mod::config::AttributeType;
use crate::lenses::{HashMapIndexWithLens, HashMapLenLens, HashMapNthKeyLens, IsFailedLens};
use crate::map_struct::Attribute;
use std::collections::HashMap;
use std::str::FromStr;
use vizia::*;

#[derive(Lens)]
pub struct NewAttributeData {
    name: String,
    ty: AttributeType,
}

impl Model for NewAttributeData {
    fn event(&mut self, _cx: &mut Context, event: &mut Event) {
        if let Some(msg) = event.message.downcast() {
            match msg {
                NewAttributeDataEvent::SetName(name) => self.name = name.clone(),
                NewAttributeDataEvent::SetTy(ty) => self.ty = *ty,
            }
        }
    }
}

enum NewAttributeDataEvent {
    SetName(String),
    SetTy(AttributeType),
}

pub fn tweak_attr_text<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Data,
    F: 'static + Send + Sync + Fn(&mut Context, <L as Lens>::Target) -> bool,
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        validator_box(cx, lens, setter, |cx, valid| {
            cx.current.toggle_class(cx, "validation_error", !valid);
        });
    });
}

pub fn validator_box<L, F1, F2>(cx: &mut Context, lens: L, setter: F1, set_valid: F2)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Data,
    F1: 'static + Send + Sync + Fn(&mut Context, <L as Lens>::Target) -> bool,
    F2: 'static + Send + Sync + Fn(&mut Context, bool),
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

#[derive(Debug, Clone, Lens)]
struct ModelContainer<T: 'static + Clone + Send + Sync> {
    val: T,
}

#[derive(Debug)]
enum ModelEvent<T> {
    Set(Option<T>), // only an option so the data can be taken out
}

impl<T: 'static + Clone + Send + Sync> Model for ModelContainer<T> {
    fn event(&mut self, _cx: &mut Context, event: &mut Event) {
        if let Some(ModelEvent::Set(msg)) = event.message.downcast() {
            if let Some(v) = msg.take() {
                self.val = v;
            }
        }
    }
}

impl Model for EditingState {
    fn event(&mut self, _cx: &mut Context, event: &mut Event) {
        if let Some(msg) = event.message.downcast() {
            match msg {
                EditingStateEvent::End => self.editing = false,
                EditingStateEvent::Start => self.editing = true,
                EditingStateEvent::Valid(b) => self.valid = *b,
            }
        }
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
    <L as Lens>::Target: ToString + FromStr + Data + Send + Sync,
    F1: 'static + Send + Sync + Clone + Fn(&mut Context, &<L as Lens>::Target) -> bool,
    F2: 'static + Send + Sync + Clone + Fn(&mut Context, <L as Lens>::Target),
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
                            setter(cx, value);
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
                            cx.emit(ModelEvent::Set(Some(val)));
                            true
                        } else {
                            false
                        }
                    },
                    move |cx, valid| {
                        cx.current.toggle_class(cx, "validation_error", !valid);
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

pub fn tweak_attr_check<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens<Target = bool>,
    F: 'static + Send + Sync + Copy + Fn(&mut Context, bool),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
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
    setter: impl 'static + Clone + Send + Sync + Fn(&mut Context, String, Attribute),
    adder: impl 'static + Fn(&mut Context, String, AttributeType),
    remover: impl 'static + Clone + Fn(&mut Context, String),
) {
    Binding::new(
        cx,
        attributes_lens.then(HashMapLenLens::new()),
        move |cx, len| {
            let len = len.get_fallible(cx).unwrap_or(0);
            for i in 0..len {
                let setter = setter.clone();
                let remover = remover.clone();
                let key_lens = attributes_lens.then(HashMapNthKeyLens::new(i));
                HStack::new(cx, move |cx| {
                    Label::new(cx, key_lens);

                    let attr_value_lens = HashMapIndexWithLens::new(attributes_lens, key_lens);
                    let s_value_lens = attr_value_lens.then(Attribute::text);
                    let i_value_lens = attr_value_lens.then(Attribute::int);
                    let f_value_lens = attr_value_lens.then(Attribute::float);
                    let b_value_lens = attr_value_lens.then(Attribute::bool);

                    let setter2 = setter.clone();
                    attr_editor(cx, s_value_lens, key_lens, move |cx, key, val| {
                        setter2(cx, key, Attribute::Text(val))
                    });
                    let setter2 = setter.clone();
                    attr_editor(cx, i_value_lens, key_lens, move |cx, key, val| {
                        setter2(cx, key, Attribute::Int(val))
                    });
                    let setter2 = setter.clone();
                    attr_editor(cx, f_value_lens, key_lens, move |cx, key, val| {
                        setter2(cx, key, Attribute::Float(val))
                    });
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
                            remover(cx, key_lens.get(cx));
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
                    Label::new(cx, ICON_DOWN_OPEN)
                        .class("icon")
                        .class("dropdown_icon");
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
                    adder(cx, name, NewAttributeData::ty.get(cx));
                    cx.emit(NewAttributeDataEvent::SetName("".to_owned()));
                }
            });
    });
}

fn attr_editor<T: ToString + FromStr + Data>(
    cx: &mut Context,
    lens: impl Lens<Target = T>,
    key: impl Send + Sync + Lens<Target = String>,
    setter: impl 'static + Clone + Send + Sync + Fn(&mut Context, String, T),
) {
    Binding::new(cx, IsFailedLens::new(lens.clone()), move |cx, failed| {
        if !failed.get(cx) {
            let key = key.clone();
            let setter = setter.clone();
            Textbox::new(cx, lens.clone()).on_edit(move |cx, text| {
                if let Ok(value) = text.parse() {
                    setter(cx, key.get(cx), value);
                    cx.current.toggle_class(cx, "validation_error", false);
                } else {
                    cx.current.toggle_class(cx, "validation_error", true);
                }
            });
        }
    });
}
