use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::MapEvent;
use arborio_state::data::sid::SIDFields;
use arborio_state::data::MapID;
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::{ModelContainer, ModelEvent};
use arborio_widgets_common::confirm_delete::deleter;
use std::cell::RefCell;

pub fn build_map_meta_tab(cx: &mut Context, map: MapID) {
    sid_editor(cx, map);
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
