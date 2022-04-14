use crate::app_state::{AppEvent, AppState, AppTab};
use crate::assets::Interned;
use crate::celeste_mod::walker::{open_module, ConfigSourceTrait};
use crate::map_struct::{from_reader, CelesteMap};
use crate::MapID;
use dialog::DialogBox;
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use vizia::*;

pub fn build_project_tab(cx: &mut Context, project: Interned) {
    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        VStack::new(cx, move |cx| {
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
                .class("btn_highlight")
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
        })
        .id("maps_container");
    });
}

fn load_map(module_root: PathBuf, project: Interned, map: Interned) -> Option<CelesteMap> {
    match load_map_inner(&module_root, project, map) {
        Ok(m) => Some(m),
        Err(e) => {
            dialog::Message::new(e.to_string())
                .title("Failed to load map")
                .show()
                .unwrap();
            None
        }
    }
}

pub fn load_map_inner(
    module_root: &Path,
    project: Interned,
    map: Interned,
) -> Result<CelesteMap, io::Error> {
    let mut config = if let Some(config) = open_module(module_root) {
        config
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Module has disappeared!",
        ));
    };
    let reader = if let Some(reader) =
        config.get_file(&PathBuf::from("Maps").join(map.to_string() + ".bin"))
    {
        reader
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Map file has disappeared!",
        ));
    };

    from_reader(
        MapID {
            sid: map,
            module: project,
        },
        reader,
    )
}
