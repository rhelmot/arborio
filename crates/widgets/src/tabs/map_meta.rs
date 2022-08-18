use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::MapEvent;
use arborio_state::data::MapID;
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::common::{ModelContainer, ModelEvent};
use arborio_widgets_common::confirm_delete::deleter;
use lazy_static::lazy_static;
use regex::Regex;
use std::cell::RefCell;

pub fn build_map_meta_tab(cx: &mut Context, map: MapID) {
    sid_editor(cx, map);
    map_deleter(cx, map);
}

#[derive(Default)]
enum Side {
    #[default]
    A,
    B,
    C,
}

impl Side {
    pub fn parse(mode_str: &str) -> Option<Self> {
        Some(match mode_str {
            "A" | "" => Side::A,
            "B" | "H" => Side::B,
            "C" | "X" => Side::C,
            _ => return None,
        })
    }
}

impl ToString for Side {
    fn to_string(&self) -> String {
        match self {
            Side::A => "A".to_owned(),
            Side::B => "B".to_owned(),
            Side::C => "C".to_owned(),
        }
    }
}

struct SIDFields<'a> {
    order: usize,
    mode: Side,
    name: &'a str,
    campaign: &'a str,
}

lazy_static! {
    static ref SID: Regex = Regex::new(r#"^(?P<campaign>[^/\\]+([/\\][^/\\]+))?[/\\](?:(?P<order>[0-9]+)(?P<side>[ABCHX]?)-)?(?P<name>.+?)(?:-(?P<sideAlt>[ABCHX]?))?$"#).unwrap();
}

impl<'a> SIDFields<'a> {
    pub fn parse(sid: &'a str) -> Result<Self, String> {
        if let Some(parsed) = SID.captures(sid) {
            let campaign = parsed.name("campaign");
            let order = parsed.name("order");
            let side = parsed.name("side").or_else(|| parsed.name("sideAlt"));
            let name = parsed.name("name");
            match (campaign, order, side, name) {
                (Some(campaign), Some(order), Some(side), Some(name))
                if !campaign.as_str().is_empty() && !order.as_str().is_empty() && !name.as_str().is_empty() => {
                    Some(Self {
                        order: order.as_str().parse().unwrap(),
                        mode: Side::parse(side.as_str()).unwrap(),
                        name: name.as_str(),
                        campaign: campaign.as_str(),
                    })
                }
                _ => None
            }
        } else {
            None
        }.ok_or_else(|| "Failed to parse SID. Must be in the format <username>/<campaign>/<order><side>-<name>, e.g. rhelmot/mymap/1B-Creekside".to_owned())
    }
}

pub fn sid_editor(cx: &mut Context, map: MapID) {
    VStack::new(cx, move |cx| {
        let val = cx
            .data::<AppState>()
            .unwrap()
            .loaded_maps
            .get(&map)
            .unwrap()
            .path
            .sid
            .clone();
        let modid = cx
            .data::<AppState>()
            .unwrap()
            .loaded_maps
            .get(&map)
            .unwrap()
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
    if app.modules[&app.loaded_maps[&map].path.module]
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
