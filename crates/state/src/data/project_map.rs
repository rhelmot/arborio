use crate::data::app::{step_modules_lookup, AppEvent, AppState};
use crate::data::tabs::AppTab;
use crate::data::{save, EventPhase, MapID, UNDO_BUFFER_SIZE};
use arborio_maploader::action::{apply_map_action, MapAction, RoomAction};
use arborio_maploader::map_struct::CelesteMap;
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::everest_yaml::EverestModuleVersion;
use arborio_modloader::module::CelesteModuleKind;
use arborio_modloader::module::{MapPath, ModuleID};
use arborio_utils::vizia::prelude::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Lens)]
pub struct MapState {
    pub map: CelesteMap,
    pub path: MapPath,
    pub undo_buffer: VecDeque<MapAction>,
    pub redo_buffer: VecDeque<MapAction>,
    pub event_phase: EventPhase,
    pub palette: ModuleAggregate,
}

impl MapID {
    pub fn action(&self, phase: EventPhase, action: MapAction) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(*self),
            event: MapEvent::Action {
                event: RefCell::new(Some(action)),
                merge_phase: phase,
            },
        }
    }

    pub fn room_action(&self, room: usize, phase: EventPhase, action: RoomAction) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(*self),
            event: MapEvent::Action {
                event: RefCell::new(Some(MapAction::RoomAction {
                    idx: room,
                    event: action,
                })),
                merge_phase: phase,
            },
        }
    }
}

impl AppState {
    pub fn apply_map_event(&mut self, cx: &mut EventContext, map: Option<MapID>, event: &MapEvent) {
        let map = if let Some(map) = map.or_else(|| self.current_map_id()) {
            map
        } else {
            return;
        };
        let state = if let Some(state) = self.loaded_maps.get_mut(&map) {
            state
        } else {
            log::error!("Internal error: event referring to unloaded map");
            return;
        };
        let module = if let Some(module) = self.modules.get_mut(&state.path.module) {
            module
        } else {
            log::error!("Internal error: loaded map referrring to unloaded module");
            return;
        };

        match event {
            MapEvent::Action { event, merge_phase } => {
                if let Some(event) = event.borrow_mut().take() {
                    match apply_map_action(cx, &mut state.map, event) {
                        Ok(undo) => {
                            cx.needs_redraw();
                            state.map.dirty = true;
                            if state.undo_buffer.len() == UNDO_BUFFER_SIZE {
                                state.undo_buffer.pop_front();
                            }
                            if state.undo_buffer.back().is_none()
                                || state.event_phase != *merge_phase
                            {
                                state.undo_buffer.push_back(undo);
                            }
                            state.event_phase = *merge_phase;
                            state.redo_buffer.clear();
                        }
                        Err(e) => {
                            log::error!("Internal error: map event: {}", e);
                        }
                    }
                } else {
                    log::error!("Internal error: MapAction being applied twice?");
                }
            }
            MapEvent::Undo => {
                if let Some(event) = state.undo_buffer.pop_back() {
                    match apply_map_action(cx, &mut state.map, event) {
                        Ok(opposite) => {
                            cx.needs_redraw();
                            state.map.dirty = true;
                            state.redo_buffer.push_back(opposite);
                            state.event_phase = EventPhase::null();
                        }
                        Err(e) => {
                            log::error!("Internal error: Failed to undo: {}", e);
                        }
                    }
                }
            }
            MapEvent::Redo => {
                if let Some(event) = state.redo_buffer.pop_back() {
                    match apply_map_action(cx, &mut state.map, event) {
                        Ok(opposite) => {
                            cx.needs_redraw();
                            state.map.dirty = true;
                            state.undo_buffer.push_back(opposite);
                            state.event_phase = EventPhase::null();
                        }
                        Err(e) => {
                            log::error!("Internal error: Failed to redo: {}", e);
                        }
                    }
                }
            }
            MapEvent::Save => match save(module, &state.path, &state.map) {
                Ok(_) => state.map.dirty = false,
                Err(e) => log::error!("Failed to save: {}", e),
            },
            MapEvent::SetName { sid } => {
                let current_sid = &state.path.sid;
                let root = if let Some(root) = module.unpacked() {
                    root
                } else {
                    log::error!("Internal error: tried to rename a packed map");
                    return;
                };

                let old_path = root.join("Maps").join(current_sid).with_extension("bin");
                let new_path = root.join("Maps").join(sid).with_extension("bin");
                let mut index = None;
                for (idx, other_sid) in module.maps.iter().enumerate() {
                    if other_sid == current_sid {
                        index = Some(idx);
                        break;
                    }
                }
                let index = if let Some(index) = index {
                    index
                } else {
                    log::error!("Internal error: rename map: maps list desync");
                    return;
                };
                if let Err(e) = std::fs::create_dir_all(new_path.parent().unwrap()) {
                    log::error!("Internal error: rename map: create_dir_all: {}", e);
                    return;
                }
                if let Err(e) = std::fs::rename(old_path, new_path) {
                    log::error!("Internal error: rename map: rename: {}", e);
                    return;
                }

                module.maps[index] = sid.clone();
                state.path.sid = sid.clone();
                self.modules_version += 1;
            }
            MapEvent::OpenMeta => {
                for (idx, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::MapMeta(m) if *m == map) {
                        cx.emit(AppEvent::SelectTab { idx });
                        return;
                    }
                }
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len(),
                });
                self.tabs.push(AppTab::MapMeta(map));
            }
            MapEvent::Delete => {
                if let Some(root) = module.unpacked() {
                    // TODO: there's no fucking way this is the best way to do this
                    let idx = match module
                        .maps
                        .iter()
                        .enumerate()
                        .filter_map(|(i, sid)| (sid == &state.path.sid).then_some(i))
                        .next()
                    {
                        Some(i) => i,
                        None => {
                            log::error!(
                                "Internal error: map to delete is not part of parent module"
                            );
                            return;
                        }
                    };
                    let old_path = root
                        .join("Maps")
                        .join(&state.path.sid)
                        .with_extension("bin");
                    if let Err(e) = std::fs::remove_file(old_path) {
                        log::error!("Failed to delete map: {}", e);
                        return;
                    }
                    module.maps.remove(idx);
                    self.loaded_maps.remove(&map);
                    self.modules_version += 1;

                    self.garbage_collect();
                } else {
                    log::error!("Internal error: tried to delete a packed map");
                }
            }
        }
    }

    pub fn apply_project_event(
        &mut self,
        cx: &mut EventContext,
        project: Option<ModuleID>,
        event: &ProjectEvent,
    ) {
        let project = match project.or_else(|| self.current_project_id()) {
            Some(project) => project,
            None => return,
        };
        let state = if let Some(state) = self.modules.get_mut(&project) {
            state
        } else {
            log::error!("Internal error: event referring to unloaded map");
            return;
        };

        match event {
            ProjectEvent::SetName { name } => {
                self.modules_lookup.remove(&state.everest_metadata.name);
                state.everest_metadata.name = name.clone();
                step_modules_lookup(
                    &mut self.modules_lookup,
                    &self.modules,
                    project,
                    self.modules.get(&project).unwrap(),
                );
                let state = self.modules.get_mut(&project).unwrap();
                state
                    .everest_metadata
                    .save(state.filesystem_root.as_ref().unwrap());
            }
            ProjectEvent::SetVersion { version } => {
                state.everest_metadata.version = version.clone();
                state
                    .everest_metadata
                    .save(state.filesystem_root.as_ref().unwrap());
            }
            ProjectEvent::SetPath { path } => {
                if let Err(e) = std::fs::rename(state.filesystem_root.as_ref().unwrap(), path) {
                    log::error!(
                        "Could not move {} to {}: {}",
                        &state.everest_metadata.name,
                        path.to_string_lossy(),
                        e
                    );
                } else {
                    state.filesystem_root = Some(path.clone());
                }
            }
            ProjectEvent::NewMap => {
                if !matches!(state.module_kind(), CelesteModuleKind::Directory) {
                    log::error!(
                        "Cannot make a new map in {}: not a directory-loaded mod",
                        &state.everest_metadata.name
                    );
                    return;
                }
                let mut new_id = 0;
                let new_sid = 'outer2: loop {
                    new_id += 1;
                    let new_sid = format!(
                        "{}/{}/{}-untitled",
                        self.config.user_name,
                        state
                            .filesystem_root
                            .as_ref()
                            .unwrap()
                            .file_name()
                            .unwrap()
                            .to_string_lossy(),
                        new_id
                    );
                    for old_sid in state.maps.iter() {
                        if **old_sid == new_sid {
                            continue 'outer2;
                        }
                    }
                    break new_sid;
                };
                state.create_map(new_sid.clone());
                cx.emit(AppEvent::OpenMap {
                    path: MapPath {
                        module: project,
                        sid: new_sid,
                    },
                });
            }
            ProjectEvent::Delete => {
                if !matches!(state.module_kind(), CelesteModuleKind::Builtin) {
                    let module = self.modules.remove(&project).unwrap();
                    // TODO can we restore the modules which were cast aside for this one?
                    self.modules_lookup.remove(&module.everest_metadata.name);
                    let path = module.filesystem_root.unwrap();
                    std::fs::remove_dir_all(path).expect("Failed to delete mod from filesystem");
                    self.garbage_collect();
                } else {
                    log::error!("Cannot delete built-in module");
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ProjectEvent {
    SetName { name: String },
    SetVersion { version: EverestModuleVersion },
    SetPath { path: PathBuf },
    NewMap,
    Delete,
}

#[derive(Debug)]
pub enum MapEvent {
    Undo,
    Redo,
    Save,
    OpenMeta,
    Delete,
    SetName {
        sid: String,
    },
    Action {
        event: RefCell<Option<MapAction>>,
        merge_phase: EventPhase,
    },
}
