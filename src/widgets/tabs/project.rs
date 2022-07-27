use crate::app_state::{AppEvent, AppState, AppTab};
use crate::assets::Interned;
use crate::celeste_mod::module::CelesteModuleKind;
use crate::celeste_mod::walker::{open_module, ConfigSourceTrait};
use crate::lenses::StaticerLens;
use crate::map_struct::{from_reader, CelesteMap};
use crate::widgets::common::label_with_pencil;
use crate::MapID;
use dialog::DialogBox;
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use vizia::*;

pub fn build_project_tab(cx: &mut Context, project: Interned) {
    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        VStack::new(cx, move |cx| {
            Binding::new(cx, AppState::modules_version, move |cx, _| {
                build_title(cx, project);
                build_map_list(cx, project);
            });
        })
        .id("maps_container");
    });
}

fn build_title(cx: &mut Context, project: Interned) {
    let module = cx
        .data::<AppState>()
        .unwrap()
        .modules
        .get(*project)
        .unwrap();
    let module_root = module.filesystem_root.clone();
    let module_name = module.everest_metadata.name.clone().to_string();
    let module_version = module.everest_metadata.version.clone();

    let editable = matches!(module.module_kind(), CelesteModuleKind::Directory);
    label_with_pencil(
        cx,
        StaticerLens::new(module_name),
        |_, _| true,
        move |cx, value| {
            cx.emit(AppEvent::SetModName {
                project,
                name: value,
            });
        },
        editable,
    )
    .class("project_name")
    .class("pencilable");
    HStack::new(cx, move |cx| {
        label_with_pencil(
            cx,
            StaticerLens::new(module_version),
            |_, _| true,
            move |cx, value| {
                cx.emit(AppEvent::SetModVersion {
                    project,
                    version: value,
                });
            },
            editable,
        )
        .class("project_version")
        .class("pencilable");
        Label::new(cx, " - ");
        label_with_pencil(
            cx,
            StaticerLens::new(
                module_root
                    .clone()
                    .map(|path| path.to_str().unwrap().to_owned())
                    .unwrap_or_else(|| "<built-in>".to_owned()),
            ),
            move |_, val| {
                val.starts_with(
                    module_root
                        .as_ref()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                )
            },
            move |cx, val| {
                cx.emit(AppEvent::SetModPath {
                    project,
                    path: val.into(),
                });
            },
            editable,
        )
        .class("project_path")
        .class("pencilable");
    });
}

fn build_map_list(cx: &mut Context, project: Interned) {
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
