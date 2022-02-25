use std::str::FromStr;

use crate::lenses::{CurrentRoomLens, RoomTweakerScopeLens};
use crate::map_struct::{CelesteMapLevel, CelesteMapLevelUpdate};
use crate::{AppEvent, AppState};
use vizia::*;

pub struct RoomTweakerWidget {}

macro_rules! edit_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentRoomLens {}.then(CelesteMapLevel::$attr),
            |cx, x| {
                emit(
                    cx,
                    CelesteMapLevelUpdate {
                        $attr: Some(x),
                        ..CelesteMapLevelUpdate::default()
                    },
                );
            },
        );
    };
}
macro_rules! edit_check {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_check(
            $cx,
            $label,
            CurrentRoomLens {}.then(CelesteMapLevel::$attr),
            |cx, x| {
                emit(
                    cx,
                    CelesteMapLevelUpdate {
                        $attr: Some(x),
                        ..CelesteMapLevelUpdate::default()
                    },
                );
            },
        );
    };
}

impl RoomTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build2(cx, |cx| {
                Binding::new(cx, RoomTweakerScopeLens {}, |cx, _| {
                    Self::members(cx);
                })
            })
            .class("tweaker")
    }

    fn members(cx: &mut Context) {
        tweak_attr_text(
            cx,
            "Name",
            CurrentRoomLens {}
                .then(CelesteMapLevel::name.map(|n| n.strip_prefix("lvl_").unwrap().to_owned())),
            |cx, name| {
                emit(
                    cx,
                    CelesteMapLevelUpdate {
                        name: Some(format!("lvl_{}", name)),
                        ..CelesteMapLevelUpdate::default()
                    },
                )
            },
        );

        edit_text!(cx, "Color", color);
        edit_text!(cx, "Camera Offset X", camera_offset_x);
        edit_text!(cx, "Camera Offset Y", camera_offset_y);
        edit_text!(cx, "Wind Pattern", wind_pattern);
        edit_check!(cx, "Space", space);
        edit_check!(cx, "Underwater", underwater);
        edit_check!(cx, "Whisper", whisper);
        edit_check!(cx, "Dark", dark);
        edit_check!(cx, "Disable Down Transition", disable_down_transition);
        edit_text!(cx, "Music", music);
        edit_text!(cx, "Alt Music", alt_music);
        edit_text!(cx, "Ambience", ambience);
        edit_text!(cx, "Music Progress", music_progress);
        edit_text!(cx, "Ambience Progress", ambience_progress);

        HStack::new(cx, move |cx| {
            Label::new(cx, "Music Layers");
            for i in 0..6 {
                let lens =
                    CurrentRoomLens {}.then(CelesteMapLevel::music_layers.map(move |x| x[i]));
                Binding::new(cx, lens, move |cx, lens| {
                    Checkbox::new(cx, lens.clone()).on_toggle(move |cx| {
                        let mut layers = [None; 6];
                        layers[i] = Some(!*lens.get(cx));
                        emit(
                            cx,
                            CelesteMapLevelUpdate {
                                music_layers: layers,
                                ..CelesteMapLevelUpdate::default()
                            },
                        );
                    });
                });
            }
        });
    }
}

impl View for RoomTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("room-tweaker".to_owned())
    }
}

fn emit(cx: &mut Context, update: CelesteMapLevelUpdate) {
    let app = cx.data::<AppState>().unwrap();
    let tab = app.map_tab_unwrap();
    let event = AppEvent::UpdateRoomMisc {
        map: tab.id.clone(),
        idx: tab.current_room,
        update,
    };
    cx.emit(event);
}

fn tweak_attr_text<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
where
    L: Lens,
    <L as Lens>::Target: ToString + FromStr + Data,
    F: 'static + Send + Sync + Fn(&mut Context, <L as Lens>::Target),
{
    HStack::new(cx, move |cx| {
        Label::new(cx, name);
        Textbox::new(cx, lens).on_edit(move |cx, value| {
            if let Ok(parsed) = value.parse() {
                setter(cx, parsed);
                cx.current.toggle_class(cx, "validation_error", false);
            } else {
                cx.current.toggle_class(cx, "validation_error", true);
            }
        });
    });
}

fn tweak_attr_check<L, F>(cx: &mut Context, name: &'static str, lens: L, setter: F)
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
