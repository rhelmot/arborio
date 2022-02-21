use crate::app_state::{AppEvent, AppState, AppTab};
use crate::assets::Interned;
use crate::celeste_mod::walker::{open_module, ConfigSourceTrait};
use crate::map_struct::CelesteMap;
use crate::{map_struct, MapID};
use dialog::DialogBox;
use std::cell::RefCell;
use std::io::Read;
use std::path::PathBuf;
use vizia::*;

pub fn build_project_tab(cx: &mut Context, project: Interned) {
    let module = cx
        .data::<AppState>()
        .unwrap()
        .modules
        .get(*project)
        .unwrap();
    let module_root = module.filesystem_root.clone();
    let mut maps = module.maps.to_vec();
    maps.sort();
    for map in maps.into_iter() {
        let module_root = module_root.clone();
        VStack::new(cx, move |cx| {
            Label::new(cx, *map).class("map_title");
        })
        .class("map_overview_card")
        .on_press(move |cx| {
            for (idx, tab) in cx.data::<AppState>().unwrap().tabs.iter().enumerate() {
                if matches!(tab, AppTab::Map(maptab) if maptab.id.sid == map) {
                    cx.emit(AppEvent::SelectTab { idx });
                    return;
                }
            }
            if let Some(module_root) = module_root.clone() {
                cx.spawn(move |cx| {
                    let project = project;
                    let map = map;
                    if let Some(map_struct) = load_map(module_root.clone(), project, map) {
                        cx.emit(AppEvent::Load {
                            map: RefCell::new(Some(Box::new(map_struct))),
                        })
                        .unwrap();
                    }
                })
            }
        });
    }
}

fn load_map(module_root: PathBuf, project: Interned, map: Interned) -> Option<CelesteMap> {
    let mut config = if let Some(config) = open_module(&module_root) {
        config
    } else {
        dialog::Message::new("Module has disappeared!")
            .show()
            .unwrap();
        return None;
    };
    let mut reader = if let Some(reader) =
        config.get_file(&PathBuf::from("Maps").join(map.to_string() + ".bin"))
    {
        reader
    } else {
        dialog::Message::new("Map file has disappeared!")
            .show()
            .unwrap();
        return None;
    };
    let mut file = vec![];
    if let Err(e) = reader.read_to_end(&mut file) {
        dialog::Message::new(format!("Could not read file: {}", e))
            .show()
            .unwrap();
        return None;
    };
    let (_, binfile) = match celeste::binel::parser::take_file(file.as_slice()) {
        Ok(binel) => binel,
        _ => {
            dialog::Message::new("Not a Celeste map").show().unwrap();
            return None;
        }
    };
    let map = match map_struct::from_binfile(
        MapID {
            module: project,
            sid: map,
        },
        binfile,
    ) {
        Ok(map) => map,
        Err(e) => {
            dialog::Message::new(format!("Data validation error: {}", e))
                .show()
                .unwrap();
            return None;
        }
    };

    Some(map)
}
