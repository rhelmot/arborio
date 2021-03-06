use super::common::*;
use crate::app_state::{EventPhase, MapEvent, StylegroundSelection};
use crate::celeste_mod::config::AttributeType;
use crate::lenses::{
    CurrentMapImplLens, CurrentMapLens, CurrentStylegroundImplLens, CurrentStylegroundLens,
    IsFailedLens, StylegroundNameLens,
};
use crate::map_struct::{Attribute, CelesteMap, CelesteMapStyleground};
use crate::{AppEvent, AppState};
use std::cell::RefCell;
use std::rc::Rc;
use vizia::*;

macro_rules! edit_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::$attr),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = x;
                emit(cx, style);
                true
            },
        );
    };
}
macro_rules! edit_check {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_check(
            $cx,
            $label,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::$attr),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = x;
                emit(cx, style);
            },
        );
    };
}
macro_rules! edit_optional_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentStylegroundImplLens {}
                .then(CelesteMapStyleground::$attr)
                .then(UnwrapLens::new()),
            |cx, x| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.$attr = if x.is_empty() { None } else { Some(x) };
                emit(cx, style);
                true
            },
        );
    };
}

pub struct StyleListWidget {}

impl StyleListWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build(cx, move |cx| {
            ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                Label::new(cx, "Foregrounds").class("style_category");
                build_active_style_list(
                    cx,
                    true,
                    CurrentMapImplLens {}.then(CelesteMap::foregrounds),
                );
                Label::new(cx, "Backgrounds").class("style_category");
                build_active_style_list(
                    cx,
                    false,
                    CurrentMapImplLens {}.then(CelesteMap::backgrounds),
                );
            });
        })
    }
}

impl View for StyleListWidget {
    fn element(&self) -> Option<String> {
        Some("style_list".to_owned())
    }
}

fn build_active_style_list<L>(cx: &mut Context, fg: bool, lens: L)
where
    L: Lens<Target = Vec<CelesteMapStyleground>> + Copy,
    <L as Lens>::Source: Model,
{
    Binding::new(cx, lens.map(|vec| vec.len()), move |cx, len_lens| {
        for idx in (0..len_lens.get_fallible(cx).unwrap_or(0)).rev() {
            let lens = lens.index(idx);
            HStack::new(cx, move |cx| {
                Label::new(cx, lens.then(StylegroundNameLens {}));
            })
            .class("palette_item")
            .class("list_highlight")
            .bind(CurrentStylegroundLens {}, move |handle, selected| {
                let is_me =
                    selected.get_fallible(handle.cx) == Some(StylegroundSelection { fg, idx });
                handle.checked(is_me);
            })
            .on_press(move |cx| {
                let tab = cx.data::<AppState>().unwrap().current_tab;
                cx.emit(AppEvent::SelectStyleground {
                    tab,
                    styleground: Some(StylegroundSelection { fg, idx }),
                });
            });
        }
    });
}

pub struct StyleTweakerWidget {}

impl StyleTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build(cx, |cx| {
                HStack::new(cx, |cx| {
                    Button::new(
                        cx,
                        |cx| {
                            if (CurrentStylegroundLens {}).get_fallible(cx).is_some() {
                                cx.emit(AppEvent::MapEvent {
                                    event: RefCell::new(Some(MapEvent::AddStyleground {
                                        loc: CurrentStylegroundLens {}.get(cx),
                                        style: Box::new(CelesteMapStyleground::default()),
                                    })),
                                    map: CurrentMapLens {}.get(cx),
                                    merge_phase: EventPhase::new(),
                                });
                            }
                        },
                        |cx| Label::new(cx, "\u{e145}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            if (CurrentStylegroundLens {}).get_fallible(cx).is_some() {
                                cx.emit(AppEvent::MapEvent {
                                    event: RefCell::new(Some(MapEvent::RemoveStyleground {
                                        loc: CurrentStylegroundLens {}.get(cx),
                                    })),
                                    map: CurrentMapLens {}.get(cx),
                                    merge_phase: EventPhase::new(),
                                });
                            }
                        },
                        |cx| Label::new(cx, "\u{e15b}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            let sel =
                                if let Some(sel) = (CurrentStylegroundLens {}).get_fallible(cx) {
                                    sel
                                } else {
                                    return;
                                };
                            let max_idx = CurrentMapImplLens {}
                                .map(move |map| map.styles(sel.fg).len())
                                .get(cx);
                            let target = if sel.idx + 1 == max_idx {
                                if sel.fg {
                                    return;
                                }
                                StylegroundSelection { fg: true, idx: 0 }
                            } else {
                                StylegroundSelection {
                                    fg: sel.fg,
                                    idx: sel.idx + 1,
                                }
                            };
                            cx.emit(AppEvent::MapEvent {
                                event: RefCell::new(Some(MapEvent::MoveStyleground {
                                    loc: sel,
                                    target,
                                })),
                                map: CurrentMapLens {}.get(cx),
                                merge_phase: EventPhase::new(),
                            });
                            cx.emit(AppEvent::SelectStyleground {
                                tab: cx.data::<AppState>().unwrap().current_tab,
                                styleground: Some(target),
                            })
                        },
                        |cx| Label::new(cx, "\u{e5ce}").class("icon"),
                    );
                    Button::new(
                        cx,
                        |cx| {
                            let sel =
                                if let Some(sel) = (CurrentStylegroundLens {}).get_fallible(cx) {
                                    sel
                                } else {
                                    return;
                                };
                            let target = if sel.idx == 0 {
                                if !sel.fg {
                                    return;
                                }
                                let max_idx = CurrentMapImplLens {}
                                    .map(move |map| map.styles(false).len())
                                    .get(cx);
                                StylegroundSelection {
                                    fg: false,
                                    idx: max_idx,
                                }
                            } else {
                                StylegroundSelection {
                                    fg: sel.fg,
                                    idx: sel.idx - 1,
                                }
                            };
                            cx.emit(AppEvent::MapEvent {
                                event: RefCell::new(Some(MapEvent::MoveStyleground {
                                    loc: sel,
                                    target,
                                })),
                                map: CurrentMapLens {}.get(cx),
                                merge_phase: EventPhase::new(),
                            });
                            cx.emit(AppEvent::SelectStyleground {
                                tab: cx.data::<AppState>().unwrap().current_tab,
                                styleground: Some(target),
                            })
                        },
                        |cx| Label::new(cx, "\u{e5cf}").class("icon"),
                    );
                });
                ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                    Binding::new(
                        cx,
                        IsFailedLens::new(CurrentStylegroundImplLens {}),
                        move |cx, is_failed| {
                            if !is_failed.get(cx) {
                                Self::members(cx);
                            }
                        },
                    );
                });
            })
            .class("tweaker")
    }

    fn members(cx: &mut Context) {
        edit_text!(cx, "Name", name);
        edit_text!(cx, "Tag", tag);
        edit_text!(cx, "X", x);
        edit_text!(cx, "Y", y);
        edit_text!(cx, "Scroll X", scroll_x);
        edit_text!(cx, "Scroll Y", scroll_y);
        edit_text!(cx, "Speed X", speed_x);
        edit_text!(cx, "Speed Y", speed_y);
        edit_text!(cx, "Color", color); // TODO real validation
        edit_text!(cx, "Alpha", alpha);
        edit_check!(cx, "Flip X", flip_x);
        edit_check!(cx, "Flip Y", flip_y);
        edit_check!(cx, "Loop X", loop_x);
        edit_check!(cx, "Loop Y", loop_y);
        edit_text!(cx, "Wind", wind);
        edit_check!(cx, "Instant In", instant_in);
        edit_check!(cx, "Instant out", instant_out);
        edit_optional_text!(cx, "Show If Flag", flag);
        edit_optional_text!(cx, "Hide If Flag", not_flag);
        edit_optional_text!(cx, "Override If Flag", always);
        tweak_attr_picker(
            cx,
            "Dreaming Status",
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::dreaming),
            [None, Some(true), Some(false)],
            |_, item| {
                match item {
                    // clion has a false-positive error here
                    None => "Both",
                    Some(true) => "Dreaming",
                    Some(false) => "Awake",
                }
                .to_owned()
            },
            |cx, item| {
                let mut style = CurrentStylegroundImplLens {}.get(cx);
                style.dreaming = item;
                emit(cx, style);
            },
        );
        edit_optional_text!(cx, "Exclude Rooms", exclude);
        edit_optional_text!(cx, "Only Rooms", only);
        edit_text!(cx, "Fade X", fade_x);
        edit_text!(cx, "Fade Y", fade_y);

        advanced_attrs_editor(
            cx,
            CurrentStylegroundImplLens {}.then(CelesteMapStyleground::attributes),
            |cx, key, value| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.insert(key, value);
                emit(cx, current);
            },
            |cx, key, ty| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.insert(
                    key,
                    match ty {
                        AttributeType::String => Attribute::Text("".to_owned()),
                        AttributeType::Float => Attribute::Float(0.0),
                        AttributeType::Int => Attribute::Int(0),
                        AttributeType::Bool => Attribute::Bool(false),
                    },
                );
                emit(cx, current);
            },
            |cx, key| {
                let mut current = CurrentStylegroundImplLens {}.get(cx);
                current.attributes.remove(&key);
                emit(cx, current);
            },
        );
    }
}

impl View for StyleTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("style_tweaker".to_owned())
    }
}

fn emit(cx: &mut Context, style: CelesteMapStyleground) {
    cx.emit(AppEvent::MapEvent {
        event: RefCell::new(Some(MapEvent::UpdateStyleground {
            loc: CurrentStylegroundLens {}.get(cx),
            style: Box::new(style),
        })),
        map: CurrentMapLens {}.get(cx),
        merge_phase: EventPhase::new(),
    });
}

fn tweak_attr_picker<T: Data>(
    // TODO move to common when mature
    cx: &mut Context,
    name: &'static str,
    lens: impl Lens<Target = T>,
    items: impl 'static + IntoIterator<Item = T> + Clone,
    labels: impl 'static + Fn(&mut Context, &T) -> String,
    setter: impl 'static + Fn(&mut Context, T),
) {
    let labels = Rc::new(labels);
    let labels2 = labels.clone();
    let setter = Rc::new(setter);
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Dropdown::new(
            cx,
            move |cx| {
                let labels2 = labels2.clone();
                let lens = lens.clone();
                HStack::new(cx, move |cx| {
                    let labels2 = labels2.clone();
                    Label::new(cx, "").bind(lens, move |handle, item| {
                        if let Some(item) = item.get_fallible(handle.cx) {
                            let label = (labels2)(handle.cx, &item);
                            handle.text(&label);
                        }
                    });
                    Label::new(cx, ICON_DOWN_OPEN)
                        .class("icon")
                        .class("dropdown_icon");
                })
            },
            move |cx| {
                let items = items.clone();
                for item in items.into_iter() {
                    let setter = setter.clone();
                    let label = labels(cx, &item);
                    Label::new(cx, &label)
                        .class("dropdown_element")
                        .class("btn_highlight")
                        .on_press(move |cx| {
                            cx.emit(PopupEvent::Close);
                            setter(cx, item.clone());
                        });
                }
            },
        );
    });
}
