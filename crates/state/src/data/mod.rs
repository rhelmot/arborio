pub mod action;
pub mod app;
pub mod app_apply;
pub mod config_editor;
pub mod project_map;
pub mod selection;
pub mod sid;
pub mod tabs;

use app::AppEvent;
use log::Level;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::path::{Path, PathBuf};

use crate::data::action::{MapAction, RoomAction};
use crate::data::project_map::MapEvent;
use arborio_maploader::map_struct::{save_as, CelesteMap};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::discovery;
use arborio_modloader::module::{CelesteModule, CelesteModuleKind, MapPath, ModuleID};
use arborio_utils::uuid_cls;
use arborio_utils::vizia::prelude::*;
use project_map::MapState;

const UNDO_BUFFER_SIZE: usize = 1000;

uuid_cls!(EventPhase);

#[derive(Serialize, Deserialize, Lens, Debug, Setter)]
pub struct AppConfig {
    pub celeste_root: Option<PathBuf>,
    pub last_filepath: PathBuf,
    pub user_name: String,

    pub draw_interval: f32,
    pub snap: bool,
    pub advanced: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            celeste_root: None,
            last_filepath: Default::default(),
            user_name: "me".to_owned(),
            draw_interval: 8.0,
            snap: true,
            advanced: false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, enum_iterator::Sequence)]
pub enum Layer {
    FgTiles,
    BgTiles,
    FgDecals,
    BgDecals,
    Entities,
    Triggers,
    ObjectTiles,
    All,
}

impl Layer {
    pub fn name(&self) -> &'static str {
        match self {
            Layer::FgTiles => "Foreground Tiles",
            Layer::BgTiles => "Background Tiles",
            Layer::Entities => "Entities",
            Layer::Triggers => "Triggers",
            Layer::FgDecals => "Foreground Decals",
            Layer::BgDecals => "Background Decals",
            Layer::ObjectTiles => "Object Tiles",
            Layer::All => "All Layers",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Progress {
    pub progress: i32,
    pub status: String,
}

pub fn trigger_module_load(cx: &mut EventContext, path: PathBuf) {
    cx.spawn(move |cx| {
        let mut result = HashMap::new();
        discovery::load_all(&path, &mut result, |p, s| {
            cx.emit(AppEvent::Progress {
                progress: Progress {
                    progress: (p * 100.0) as i32,
                    status: s,
                },
            })
            .unwrap();
        });
        cx.emit(AppEvent::Progress {
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
        })
        .unwrap();
        cx.emit(AppEvent::SetModules {
            modules: Mutex::new(result),
        })
        .unwrap();
    })
}

pub fn trigger_palette_update(
    modules: &HashMap<ModuleID, CelesteModule>,
    modules_lookup: &HashMap<String, ModuleID>,
    maps: &mut HashMap<MapID, MapState>,
) -> ModuleAggregate {
    for state in maps.values_mut() {
        state.cache.palette = ModuleAggregate::new(
            modules,
            modules_lookup,
            &Some(state.data.clone_meta()),
            state.cache.path.module,
            true,
        );
    }
    // discard logs here
    ModuleAggregate::new_omni(modules, false)
}

fn load_map(module_root: &Path, sid: &str) -> Option<CelesteMap> {
    match CelesteModule::load_map_static(module_root, sid) {
        Ok(m) => Some(m),
        Err(e) => {
            log::error!("Failed to load map: {}", e);
            None
        }
    }
}

fn save(module: &CelesteModule, path: &MapPath, map: &CelesteMap) -> Result<(), io::Error> {
    if !matches!(module.module_kind(), CelesteModuleKind::Directory) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Can only save maps loaded from unpacked mods",
        ));
    }

    if let Some(root) = &module.filesystem_root {
        if root.is_dir() {
            return save_as(
                map,
                &path.sid,
                &root
                    .join("Maps")
                    .join(path.sid.clone())
                    .with_extension("bin"),
            );
        }
    }

    Err(io::Error::new(io::ErrorKind::Other, "Unknown error???"))
}

uuid_cls!(MapID);

impl MapID {
    pub fn action(&self, phase: EventPhase, action: MapAction) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(*self),
            event: MapEvent::Action {
                event: vec![action],
                merge_phase: phase,
            },
        }
    }

    pub fn room_action(&self, room: usize, phase: EventPhase, action: RoomAction) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(*self),
            event: MapEvent::Action {
                event: vec![MapAction::RoomAction {
                    idx: room,
                    event: action,
                }],
                merge_phase: phase,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArborioRecord {
    pub level: Level,
    pub message: String,
}
