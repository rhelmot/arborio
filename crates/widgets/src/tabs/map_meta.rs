use arborio_state::data::action::MapAction;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::{MapEvent, MapStateData, MapStateUpdate};
use arborio_state::data::sid::SIDFields;
use arborio_state::data::{EventPhase, MapID};
use arborio_state::lenses::CurrentMapImplLens;
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::{tweak_attr_check, tweak_attr_text};
use arborio_widgets_common::common::{ModelContainer, ModelEvent};
use arborio_widgets_common::confirm_delete::deleter;
use std::cell::RefCell;

pub fn build_map_meta_tab(cx: &mut Context, map: MapID) {
    sid_editor(cx, map);
    meta_tweaker(cx, map);
    map_deleter(cx, map);
}

pub fn sid_editor(cx: &mut Context, map: MapID) {
    VStack::new(cx, move |cx| {
        let val = cx
            .data::<AppState>()
            .unwrap()
            .loaded_maps
            .get(&map)
            .unwrap()
            .cache
            .path
            .sid
            .clone();
        let modid = cx
            .data::<AppState>()
            .unwrap()
            .loaded_maps
            .get(&map)
            .unwrap()
            .cache
            .path
            .module;
        ModelContainer { val: val.clone() }.build(cx);
        HStack::new(cx, move |cx| {
            Textbox::new(cx, ModelContainer::<String>::val).on_edit(|cx, val| {
                cx.emit(ModelEvent::Set(RefCell::new(Some(val))));
            });
        });
        Binding::new(cx, ModelContainer::<String>::val, move |cx, lens| {
            let val2 = lens.get(cx);
            let maps = &cx
                .data::<AppState>()
                .unwrap()
                .modules
                .get(&modid)
                .unwrap()
                .maps;
            let match_existing = maps.iter().any(|s| s == &val2);
            match SIDFields::parse(&val2) {
                Ok(fields) => {
                    let similar_count = maps
                        .iter()
                        .filter_map(|sid| {
                            SIDFields::parse(sid).ok().and_then(|parsed| {
                                if sid != &val && parsed.campaign == fields.campaign {
                                    Some(())
                                } else {
                                    None
                                }
                            })
                        })
                        .count();
                    Label::new(
                        cx,
                        &format!("Campaign: {} ({} other)", &fields.campaign, similar_count),
                    );
                    Label::new(
                        cx,
                        &format!(
                            "Chapter: {} (number {}) {} Side",
                            &fields.name,
                            &fields.order,
                            &fields.mode.to_string()
                        ),
                    );
                    if val != val2 && match_existing {
                        Label::new(cx, "Conflicts with an existing name!");
                    }
                    if !match_existing {
                        Button::new(
                            cx,
                            move |cx| {
                                cx.emit(AppEvent::MapEvent {
                                    map: Some(map),
                                    event: MapEvent::SetName { sid: val2.clone() },
                                })
                            },
                            move |cx| Label::new(cx, "Save"),
                        );
                    }
                }
                Err(e) => {
                    Label::new(cx, &e);
                }
            }
        });
    });
}

macro_rules! edit_text {
    ($cx: expr, $label:expr, $attr:ident) => {
        tweak_attr_text(
            $cx,
            $label,
            CurrentMapImplLens {}.then(MapStateData::$attr),
            |cx, x| {
                emit(
                    cx,
                    MapStateUpdate {
                        $attr: Some(x),
                        ..MapStateUpdate::default()
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
            CurrentMapImplLens {}.then(MapStateData::$attr),
            |cx, x| {
                emit(
                    cx,
                    MapStateUpdate {
                        $attr: Some(x),
                        ..MapStateUpdate::default()
                    },
                );
            },
        );
    };
}

fn emit(cx: &mut EventContext, update: MapStateUpdate) {
    let app = cx.data::<AppState>().unwrap();
    let map = app.current_map_id().unwrap();
    cx.emit(map.action(
        EventPhase::new(),
        MapAction::MetaUpdate {
            update: Box::new(update),
        },
    )); // TODO batch correctly
}

fn meta_tweaker(cx: &mut Context, _map: MapID) {
    edit_text!(cx, "Foreground Tiles", fg_tiles);
    edit_text!(cx, "Background Tiles", bg_tiles);
    edit_text!(cx, "Animated Tiles", animated_tiles);
    edit_text!(cx, "Sprites", sprites);
    edit_text!(cx, "Portraits", portraits);
    edit_text!(cx, "Cassette Note Color", cassette_note_color);
    edit_text!(cx, "Title Text Color", title_text_color);
    edit_text!(cx, "Title Base Color", title_base_color);
    edit_text!(cx, "Title Accent Color", title_accent_color);
    edit_text!(cx, "Icon", icon);
    edit_check!(cx, "Interlude", interlude);
    edit_text!(cx, "Wipe", wipe);
    edit_text!(cx, "Cassette Song", cassette_song);
    edit_text!(cx, "Postcard Sound ID", postcard_sound_id);

    edit_text!(cx, "Color Grade", color_grade);
    edit_check!(cx, "Dreaming", dreaming);
    edit_text!(cx, "Intro Type", intro_type);
    edit_text!(cx, "Bloom Base", bloom_base);
    edit_text!(cx, "Bloom Strength", bloom_strength);
    edit_text!(cx, "Darkness Alpha", darkness_alpha);
    edit_text!(cx, "Core Mode", core_mode);

    edit_check!(cx, "Heart Is End", heart_is_end);
    edit_text!(cx, "Inventory", inventory);
    edit_text!(cx, "Start Level", start_level);
    edit_check!(cx, "Seeker Slowdown", seeker_slowdown);
    edit_check!(cx, "Theo In Bubble", theo_in_bubble);
    edit_check!(
        cx,
        "Ignore Level Audio Layer Data",
        ignore_level_audio_layer_data
    );

    edit_text!(cx, "Ambience", ambience);
    edit_text!(cx, "Music", music);
}

fn map_deleter(cx: &mut Context, map: MapID) {
    let app = cx.data::<AppState>().unwrap();
    if app.modules[&app.loaded_maps[&map].cache.path.module]
        .unpacked()
        .is_none()
    {
        return;
    }

    deleter(
        cx,
        "Delete Map",
        "Type the SID to confirm. This cannot be undone!",
        move |cx, text| {
            cx.data::<AppState>()
                .unwrap()
                .loaded_maps
                .get(&map)
                .unwrap()
                .cache
                .path
                .sid
                == text
        },
        move |cx| {
            cx.emit(AppEvent::MapEvent {
                map: Some(map),
                event: MapEvent::Delete,
            })
        },
    );
}
