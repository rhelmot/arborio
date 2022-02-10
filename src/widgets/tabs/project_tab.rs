use crate::app_state::{AppEvent, AppState};
use crate::celeste_mod::walker::{open_module, ConfigSourceTrait};
use crate::map_struct::CelesteMap;
use crate::{assets, map_struct, MapID};
use dialog::DialogBox;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::PathBuf;
use vizia::*;

pub fn build_project_tab(cx: &mut Context, project: &str) {
    let module = cx.data::<AppState>().unwrap().modules.get(project).unwrap();
    let module_root = module.filesystem_root.clone();
    let maps = module.maps.to_vec();
    for map in maps.into_iter() {
        let map2 = map.clone();
        let project = project.to_owned();
        let module_root = module_root.clone();
        VStack::new(cx, move |cx| {
            Label::new(cx, &map2).class("map_title");
        })
        .class("map_overview_card")
        .on_press(move |cx| {
            let project = project.clone();
            let map = map.clone();
            if let Some(module_root) = module_root.clone() {
                cx.spawn(move |cx| {
                    let project = project.clone();
                    let map = map.clone();
                    if let Some(map_struct) = load_map(module_root.clone(), project, map) {
                        cx.emit(AppEvent::Load {
                            map: RefCell::new(Some(Box::new(map_struct))),
                        });
                    }
                })
            }
        });
    }
}

fn load_map(module_root: PathBuf, project: String, map: String) -> Option<CelesteMap> {
    let mut config = if let Some(config) = open_module(&module_root) {
        config
    } else {
        dialog::Message::new("Module has disappeared!").show();
        return None;
    };
    let mut reader =
        if let Some(reader) = config.get_file(&PathBuf::from("Maps").join(map.clone() + ".bin")) {
            reader
        } else {
            dialog::Message::new("Map file has disappeared!").show();
            return None;
        };
    let mut file = vec![];
    if let Err(e) = reader.read_to_end(&mut file) {
        dialog::Message::new(format!("Could not read file: {}", e)).show();
        return None;
    };
    let (_, binfile) = match celeste::binel::parser::take_file(file.as_slice()) {
        Ok(binel) => binel,
        _ => {
            dialog::Message::new("Not a Celeste map").show();
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
            dialog::Message::new(format!("Data validation error: {}", e)).show();
            return None;
        }
    };

    Some(map)
}
