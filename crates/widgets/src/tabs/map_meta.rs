use arborio_state::data::action::MapAction;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::{MapEvent, MapStateData, MapStateUpdate};
use arborio_state::data::sid::SIDFields;
use arborio_state::data::{EventPhase, MapID};
use arborio_state::lenses::{CurrentMapImplLens, StaticerLens};
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::{tweak_attr_check, tweak_attr_text, tweak_attr_text_dropdown};
use arborio_widgets_common::common::{ModelContainer, ModelEvent};
use arborio_widgets_common::confirm_delete::deleter;
use std::cell::RefCell;

pub fn build_map_meta_tab(cx: &mut Context, map: MapID) {
    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        sid_editor(cx, map);
        meta_tweaker(cx, map);
        map_deleter(cx, map);
    })
    .id("map_meta_tab");
}

const WIPE_OPTIONS: [&str; 10] = [
    "Celeste.CurtainWipe",
    "Celeste.AngledWipe",
    "Celeste.DreamWipe",
    "Celeste.KeyDoorWipe",
    "Celeste.WindWipe",
    "Celeste.DropWipe",
    "Celeste.FallWipe",
    "Celeste.MountainWipe",
    "Celeste.HeartWipe",
    "Celeste.StarfieldWipe",
];

const POSTCARD_SOUND_OPTIONS: [&str; 8] = [
    "event:/ui/main/postcard_ch1",
    "event:/ui/main/postcard_ch2",
    "event:/ui/main/postcard_ch3",
    "event:/ui/main/postcard_ch4",
    "event:/ui/main/postcard_ch5",
    "event:/ui/main/postcard_ch6",
    "event:/ui/main/postcard_csides",
    "event:/new_content/ui/main/postcard_variants",
];

const COLOR_GRADE_OPTIONS: [&str; 10] = [
    "none",
    "cold",
    "credits",
    "feelingdown",
    "golden",
    "hot",
    "oldsite",
    "panicattack",
    "reflection",
    "templevoid",
];

const INTRO_TYPE_OPTIONS: [&str; 10] = [
    "Transition",
    "Respawn",
    "WalkInRight",
    "WalkInLeft",
    "Jump",
    "WakeUp",
    "Fall",
    "TempleMirrorVoid",
    "None",
    "ThinkForABit",
];

const CORE_MODE_OPTIONS: [&str; 3] = ["None", "Hot", "Cold"];

const INVENTORY_OPTIONS: [&str; 7] = [
    "Default",
    "CH6End",
    "Core",
    "OldSite",
    "Prologue",
    "TheSummit",
    "Farewell",
];

const MUSIC_OPTIONS: [&str; 82] = [
    "event:/music/menu/level_select",
    "event:/music/menu/credits",
    "event:/music/menu/complete_area",
    "event:/music/menu/complete_summit",
    "event:/music/menu/complete_bside",
    "event:/game/00_prologue/intro_vignette",
    "event:/music/lvl0/intro",
    "event:/music/lvl0/bridge",
    "event:/music/lvl0/title_ping",
    "event:/music/lvl1/main",
    "event:/music/lvl1/theo",
    "event:/music/lvl2/beginning",
    "event:/music/lvl2/mirror",
    "event:/music/lvl2/dreamblock_sting_pt1",
    "event:/music/lvl2/dreamblock_sting_pt2",
    "event:/music/lvl2/evil_madeline",
    "event:/music/lvl2/chase",
    "event:/music/lvl2/phone_loop",
    "event:/music/lvl2/phone_end",
    "event:/music/lvl2/awake",
    "event:/music/lvl3/intro",
    "event:/music/lvl3/explore",
    "event:/music/lvl3/clean",
    "event:/music/lvl3/clean_extended",
    "event:/music/lvl3/oshiro_theme",
    "event:/music/lvl3/oshiro_chase",
    "event:/music/lvl4/main",
    "event:/music/lvl4/heavy_winds",
    "event:/music/lvl4/minigame",
    "event:/music/lvl5/normal",
    "event:/music/lvl5/middle_temple",
    "event:/music/lvl5/mirror",
    "event:/music/lvl5/mirror_cutscene",
    "event:/music/lvl6/madeline_and_theo",
    "event:/music/lvl6/starjump",
    "event:/music/lvl6/the_fall",
    "event:/music/lvl6/badeline_fight",
    "event:/music/lvl6/badeline_glitch",
    "event:/music/lvl6/badeline_acoustic",
    "event:/music/lvl6/main",
    "event:/music/lvl6/secret_room",
    "event:/music/lvl7/main",
    "event:/music/lvl7/final_ascent",
    "event:/music/lvl8/main",
    "event:/music/lvl9/main",
    "event:/classic/pico8_mus_00",
    "event:/classic/pico8_mus_01",
    "event:/classic/pico8_mus_02",
    "event:/classic/pico8_mus_03",
    "event:/classic/sfx61",
    "event:/classic/sfx62",
    "event:/classic/pico8_boot",
    "event:/music/remix/01_forsaken_city",
    "event:/music/remix/02_old_site",
    "event:/music/remix/03_resort",
    "event:/music/remix/04_cliffside",
    "event:/music/remix/05_mirror_temple",
    "event:/music/remix/06_reflection",
    "event:/music/remix/07_summit",
    "event:/music/remix/09_core",
    "event:/music/cassette/01_forsaken_city",
    "event:/music/cassette/02_old_site",
    "event:/music/cassette/03_resort",
    "event:/music/cassette/04_cliffside",
    "event:/music/cassette/05_mirror_temple",
    "event:/music/cassette/06_reflection",
    "event:/music/cassette/07_summit",
    "event:/music/cassette/09_core",
    "event:/new_content/music/lvl10/part01",
    "event:/new_content/music/lvl10/part02",
    "event:/new_content/music/lvl10/part03",
    "event:/new_content/music/lvl10/intermission_heartgroove",
    "event:/new_content/music/lvl10/intermission_powerpoint",
    "event:/new_content/music/lvl10/reconciliation",
    "event:/new_content/music/lvl10/cassette_rooms",
    "event:/new_content/music/lvl10/final_run",
    "event:/new_content/music/lvl10/cinematic/end",
    "event:/new_content/music/lvl10/cinematic/end_intro",
    "event:/new_content/music/lvl10/cinematic/bird_crash_first",
    "event:/new_content/music/lvl10/cinematic/bird_crash_second",
    "event:/new_content/music/lvl10/granny_farewell",
    "event:/new_content/music/lvl10/golden_room",
];

const CASSETTE_MUSIC_OPTIONS: [&str; 9] = [
    "event:/music/cassette/01_forsaken_city",
    "event:/music/cassette/02_old_site",
    "event:/music/cassette/03_resort",
    "event:/music/cassette/04_cliffside",
    "event:/music/cassette/05_mirror_temple",
    "event:/music/cassette/06_reflection",
    "event:/music/cassette/07_summit",
    "event:/music/cassette/09_core",
    "event:/new_content/music/lvl10/cassette_rooms",
];

const AMBIENCE_OPTIONS: [&str; 23] = [
    "event:/env/amb/00_prologue",
    "event:/env/amb/01_main",
    "event:/env/amb/02_awake",
    "event:/env/amb/02_dream",
    "event:/env/amb/03_exterior",
    "event:/env/amb/03_interior",
    "event:/env/amb/03_pico8_closeup",
    "event:/env/amb/04_main",
    "event:/env/amb/05_interior_dark",
    "event:/env/amb/05_interior_main",
    "event:/env/amb/05_mirror_sequence",
    "event:/env/amb/06_lake",
    "event:/env/amb/06_main",
    "event:/env/amb/06_prehug",
    "event:/env/amb/09_main",
    "event:/env/amb/worldmap",
    "event:/new_content/env/10_rain",
    "event:/new_content/env/10_electricity",
    "event:/new_content/env/10_endscene",
    "event:/new_content/env/10_rushingvoid",
    "event:/new_content/env/10_space_underwater",
    "event:/new_content/env/10_voidspiral",
    "event:/new_content/env/10_grannyclouds",
];

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
            Label::new(cx, "SID").class("label");
            Textbox::new(cx, ModelContainer::<String>::val).on_edit(|cx, val| {
                cx.emit(ModelEvent::Set(RefCell::new(Some(val))));
            });
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "").class("label");
            VStack::new(cx, move |cx| {
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
                                &format!(
                                    "Campaign: {} ({} other)",
                                    &fields.campaign, similar_count
                                ),
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

macro_rules! edit_text_dropdown {
    ($cx: expr, $label:expr, $attr:ident, $options:expr) => {
        tweak_attr_text_dropdown(
            $cx,
            $label,
            CurrentMapImplLens {}.then(MapStateData::$attr),
            StaticerLens::new($options.into_iter().map(|x| x.to_owned()).collect()),
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
    edit_text_dropdown!(cx, "Wipe", wipe, WIPE_OPTIONS);
    edit_text_dropdown!(cx, "Cassette Song", cassette_song, CASSETTE_MUSIC_OPTIONS);
    edit_text_dropdown!(
        cx,
        "Postcard Sound ID",
        postcard_sound_id,
        POSTCARD_SOUND_OPTIONS
    );

    edit_text_dropdown!(cx, "Color Grade", color_grade, COLOR_GRADE_OPTIONS);
    edit_check!(cx, "Dreaming", dreaming);
    edit_text_dropdown!(cx, "Intro Type", intro_type, INTRO_TYPE_OPTIONS);
    edit_text!(cx, "Bloom Base", bloom_base);
    edit_text!(cx, "Bloom Strength", bloom_strength);
    edit_text!(cx, "Darkness Alpha", darkness_alpha);
    edit_text_dropdown!(cx, "Core Mode", core_mode, CORE_MODE_OPTIONS);

    edit_check!(cx, "Heart Is End", heart_is_end);
    edit_text_dropdown!(cx, "Inventory", inventory, INVENTORY_OPTIONS);
    edit_text!(cx, "Start Level", start_level);
    edit_check!(cx, "Seeker Slowdown", seeker_slowdown);
    edit_check!(cx, "Theo In Bubble", theo_in_bubble);
    edit_check!(
        cx,
        "Ignore Level Audio Layer Data",
        ignore_level_audio_layer_data
    );

    edit_text_dropdown!(cx, "Ambience", ambience, AMBIENCE_OPTIONS);
    edit_text_dropdown!(cx, "Music", music, MUSIC_OPTIONS);
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
