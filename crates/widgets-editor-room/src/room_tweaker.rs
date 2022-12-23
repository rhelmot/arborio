use arborio_maploader::map_struct::{CelesteMapLevel, CelesteMapLevelUpdate};
use arborio_modloader::config::Number;
use arborio_state::data::action::RoomAction;
use arborio_state::data::app::AppState;
use arborio_state::data::tabs::AppTab;
use arborio_state::data::EventPhase;
use arborio_state::lenses::{
    current_room_lens, rect_h_lens, rect_w_lens, rect_x_lens, rect_y_lens, RoomTweakerScopeLens,
};
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::advanced_tweaker::*;

pub struct RoomTweakerWidget {}

macro_rules! edit_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            current_room_lens().then(CelesteMapLevel::$attr),
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
macro_rules! edit_float {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            current_room_lens()
                .then(CelesteMapLevel::$attr)
                .into_lens::<Number>(),
            |cx, x| {
                emit(
                    cx,
                    CelesteMapLevelUpdate {
                        $attr: Some(x.into()),
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
            current_room_lens().then(CelesteMapLevel::$attr),
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
            current_room_lens().then(
                CelesteMapLevel::name.map(|n| n.strip_prefix("lvl_").unwrap_or(n).to_owned()),
            ),
            |cx, name| {
                let app = cx.data::<AppState>().unwrap();
                let Some(AppTab::Map(maptab)) = app.tabs.get(app.current_tab) else { panic!() };
                let map = &app.loaded_maps.get(&maptab.id).unwrap().data;
                if map
                    .levels
                    .iter()
                    .enumerate()
                    .any(|(i, lvl)| i != maptab.current_room && lvl.data.name == name)
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
                current_room_lens()
                    .then(CelesteMapLevel::bounds)
                    .then(rect_x_lens()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    if parsed % 8 == 0 {
                        emit_bounds(cx, Some(parsed), None, None, None);
                        cx.toggle_class("validation_error", false);
                    } else {
                        cx.toggle_class("validation_error", true);
                    }
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Y");
            Textbox::new(
                cx,
                current_room_lens()
                    .then(CelesteMapLevel::bounds)
                    .then(rect_y_lens()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    if parsed % 8 == 0 {
                        emit_bounds(cx, None, Some(parsed), None, None);
                        cx.toggle_class("validation_error", false);
                    } else {
                        cx.toggle_class("validation_error", true);
                    }
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Width");
            Textbox::new(
                cx,
                current_room_lens()
                    .then(CelesteMapLevel::bounds)
                    .then(rect_w_lens()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    if parsed % 8 == 0 {
                        emit_bounds(cx, None, None, Some(parsed), None);
                        cx.toggle_class("validation_error", false);
                    } else {
                        cx.toggle_class("validation_error", true);
                    }
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Height");
            Textbox::new(
                cx,
                current_room_lens()
                    .then(CelesteMapLevel::bounds)
                    .then(rect_h_lens()),
            )
            .on_edit(move |cx, value| {
                if let Ok(parsed) = value.parse() {
                    if parsed % 8 == 0 {
                        emit_bounds(cx, None, None, None, Some(parsed));
                        cx.toggle_class("validation_error", false);
                    } else {
                        cx.toggle_class("validation_error", true);
                    }
                } else {
                    cx.toggle_class("validation_error", true);
                }
            });
        });

        edit_text!(cx, "Color", color);
        edit_float!(cx, "Camera Offset X", camera_offset_x);
        edit_float!(cx, "Camera Offset Y", camera_offset_y);
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
                    current_room_lens().then(CelesteMapLevel::music_layers.map(move |x| x[i]));
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
    let mut bounds = app.loaded_maps.get(&tab.id).unwrap().data.levels[tab.current_room]
        .data
        .bounds;
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
