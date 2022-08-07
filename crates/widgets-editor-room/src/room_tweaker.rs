use arborio_maploader::action::RoomAction;
use arborio_maploader::map_struct::{CelesteMapLevel, CelesteMapLevelUpdate};
use arborio_state::data::app::AppState;
use arborio_state::data::tabs::AppTab;
use arborio_state::data::EventPhase;
use arborio_state::lenses::{
    CurrentRoomLens, RectHLens, RectWLens, RectXLens, RectYLens, RoomTweakerScopeLens,
};
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::*;

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
            .build(cx, |cx| {
                Binding::new(cx, RoomTweakerScopeLens {}, |cx, _| {
                    ScrollView::new(cx, 0.0, 0.0, false, true, Self::members);
                });
            })
            .class("tweaker")
    }

    fn members(cx: &mut Context) {
        tweak_attr_text(
            cx,
            "Name",
            CurrentRoomLens {}.then(
                CelesteMapLevel::name.map(|n| n.strip_prefix("lvl_").unwrap_or(n).to_owned()),
            ),
            |cx, name| {
                let app = cx.data::<AppState>().unwrap();
                let maptab = if let Some(AppTab::Map(maptab)) = app.tabs.get(app.current_tab) {
                    maptab
                } else {
                    panic!()
                };
                let map = &app.loaded_maps.get(&maptab.id).unwrap().map;
                if map
                    .levels
                    .iter()
                    .enumerate()
                    .any(|(i, lvl)| i != maptab.current_room && lvl.name == name)
                {
                    return false;
                }

                emit(
                    cx,
                    CelesteMapLevelUpdate {
                        name: Some(name),
                        ..CelesteMapLevelUpdate::default()
                    },
                );
                true
            },
        );
        HStack::new(cx, move |cx| {
            Label::new(cx, "X");
            Textbox::new(
                cx,
                CurrentRoomLens {}
                    .then(CelesteMapLevel::bounds)
                    .then(RectXLens::new()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    emit_bounds(cx, Some(parsed), None, None, None);
                    cx.toggle_class("validation_error", false);
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Y");
            Textbox::new(
                cx,
                CurrentRoomLens {}
                    .then(CelesteMapLevel::bounds)
                    .then(RectYLens::new()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    emit_bounds(cx, None, Some(parsed), None, None);
                    cx.toggle_class("validation_error", false);
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Width");
            Textbox::new(
                cx,
                CurrentRoomLens {}
                    .then(CelesteMapLevel::bounds)
                    .then(RectWLens::new()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    emit_bounds(cx, None, None, Some(parsed), None);
                    cx.toggle_class("validation_error", false);
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Height");
            Textbox::new(
                cx,
                CurrentRoomLens {}
                    .then(CelesteMapLevel::bounds)
                    .then(RectHLens::new()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    emit_bounds(cx, None, None, None, Some(parsed));
                    cx.toggle_class("validation_error", false);
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });

        edit_text!(cx, "Color", color);
        edit_text!(cx, "Camera Offset X", camera_offset_x);
        edit_text!(cx, "Camera Offset Y", camera_offset_y);
        edit_text!(cx, "Wind Pattern", wind_pattern);
        edit_check!(cx, "Space", space);
        edit_check!(cx, "Underwater", underwater);
        edit_check!(cx, "Whisper", whisper);
        edit_check!(cx, "Dark", dark);
        edit_check!(cx, "Disable Down Transition", disable_down_transition);
        edit_text!(cx, "Enforce Dash Number", enforce_dash_number);
        edit_text!(cx, "Music", music);
        edit_text!(cx, "Alt Music", alt_music);
        edit_text!(cx, "Ambience", ambience);
        edit_text!(cx, "Music Progress", music_progress);
        edit_text!(cx, "Ambience Progress", ambience_progress);
        edit_text!(cx, "Delay Alt Music Fade", delay_alt_music_fade);

        HStack::new(cx, move |cx| {
            Label::new(cx, "Music Layers");
            for i in 0..4 {
                let lens =
                    CurrentRoomLens {}.then(CelesteMapLevel::music_layers.map(move |x| x[i]));
                Binding::new(cx, lens, move |cx, lens| {
                    Checkbox::new(cx, lens.clone()).on_toggle(move |cx| {
                        let mut layers = [None; 4];
                        layers[i] = Some(!lens.get(cx));
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
    fn element(&self) -> Option<&'static str> {
        Some("room-tweaker")
    }
}

fn emit(cx: &mut EventContext, update: CelesteMapLevelUpdate) {
    let app = cx.data::<AppState>().unwrap();
    let tab = app.map_tab_unwrap();
    cx.emit(tab.id.room_action(
        tab.current_room,
        EventPhase::new(),
        RoomAction::UpdateRoomMisc {
            update: Box::new(update),
        },
    )); // TODO batch correctly
}

fn emit_bounds(
    cx: &mut EventContext,
    update_x: Option<i32>,
    update_y: Option<i32>,
    update_w: Option<i32>,
    update_h: Option<i32>,
) {
    let app = cx.data::<AppState>().unwrap();
    let tab = app.map_tab_unwrap();
    let mut bounds = app.loaded_maps.get(&tab.id).unwrap().map.levels[tab.current_room].bounds;
    if let Some(x) = update_x {
        bounds.origin.x = x;
    }
    if let Some(y) = update_y {
        bounds.origin.y = y;
    }
    if let Some(w) = update_w {
        bounds.size.width = w;
    }
    if let Some(h) = update_h {
        bounds.size.height = h;
    }
    cx.emit(tab.id.room_action(
        tab.current_room,
        EventPhase::new(),
        RoomAction::MoveRoom { bounds },
    )); // TODO batch correctly
}
