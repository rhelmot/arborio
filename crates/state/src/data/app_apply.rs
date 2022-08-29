use crate::data::app::{build_modules_lookup, step_modules_lookup, AppEvent, AppState};
use crate::data::project_map::MapState;
use crate::data::tabs::{AppTab, ConfigEditorTab, MapTab};
use crate::data::{load_map, trigger_module_load, trigger_palette_update, MapID};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::everest_yaml::{EverestModuleVersion, EverestYaml};
use arborio_modloader::module::{CelesteModule, ModuleID};
use arborio_utils::units::*;
use arborio_utils::uuid::next_uuid;
use arborio_utils::vizia::prelude::*;
use arborio_walker::{ConfigSource, FolderSource};
use log::Level;
use std::cell::RefCell;
use std::ops::DerefMut;

impl AppState {
    pub fn apply(&mut self, cx: &mut EventContext, event: &AppEvent) {
        match event {
            // global events
            AppEvent::Log { message } => {
                self.logs.push(message.clone());
                if message.level <= Level::Error {
                    self.error_message = message.message.clone();
                }
            }
            AppEvent::Progress { progress } => {
                self.progress = progress.clone();
            }
            AppEvent::SetClipboard { contents } => {
                cx.set_clipboard(contents.clone())
                    .unwrap_or_else(|e| log::error!("Failed to copy: {}", e));
            }
            AppEvent::OpenModuleOverviewTab { module } => {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::ProjectOverview(m) if m == module) {
                        cx.emit(AppEvent::SelectTab { idx: i });
                        return;
                    }
                }
                self.tabs.push(AppTab::ProjectOverview(*module));
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len() - 1,
                });
            }
            AppEvent::OpenInstallationTab => {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::CelesteOverview) {
                        cx.emit(AppEvent::SelectTab { idx: i });
                        return;
                    }
                }
                self.tabs.push(AppTab::CelesteOverview);
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len() - 1,
                });
            }
            AppEvent::OpenLogsTab => {
                self.error_message.clear();
                for (i, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::Logs) {
                        cx.emit(AppEvent::SelectTab { idx: i });
                        return;
                    }
                }
                self.tabs.push(AppTab::Logs);
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len() - 1,
                });
            }
            AppEvent::OpenConfigEditorTab => {
                self.tabs
                    .push(AppTab::ConfigEditor(ConfigEditorTab::default()));
                cx.emit(AppEvent::SelectTab {
                    idx: self.tabs.len() - 1,
                });
            }
            AppEvent::OpenMap { path } => {
                let mut found = false;
                for (idx, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::Map(maptab) if &self.loaded_maps.get(&maptab.id).unwrap().cache.path == path)
                    {
                        cx.emit(AppEvent::SelectTab { idx });
                        found = true;
                        break;
                    }
                }
                if !found {
                    if let Some(module) = self.modules.get(&path.module) {
                        if let Some(module_root) = module.filesystem_root.clone() {
                            let path = path.clone();
                            cx.spawn(move |cx| {
                                if let Some(map_struct) = load_map(&module_root, &path.sid) {
                                    cx.emit(AppEvent::LoadMap {
                                        path: path.clone(),
                                        map: RefCell::new(Some(Box::new(map_struct))),
                                    })
                                    .unwrap();
                                }
                            })
                        }
                    }
                }
            }
            AppEvent::LoadMap { path, map } => {
                if let Some(map) = map.borrow_mut().take() {
                    let id = if let Some(id) = self.loaded_maps_lookup.get(path) {
                        *id
                    } else {
                        MapID::new()
                    };
                    if !self.loaded_maps.contains_key(&id) {
                        self.tabs.push(AppTab::Map(MapTab {
                            nonce: next_uuid(),
                            id,
                            current_room: 0,
                            current_selected: None,
                            styleground_selected: None,
                            transform: MapToScreen::identity(),
                            preview_pos: MapPointStrict::zero(),
                        }));
                        cx.emit(AppEvent::SelectTab {
                            idx: self.tabs.len() - 1,
                        });
                    }

                    let palette = ModuleAggregate::new(
                        &self.modules,
                        &self.modules_lookup,
                        &map.meta,
                        path.module,
                        true,
                    );

                    self.loaded_maps
                        .insert(id, MapState::new(*map, path.clone(), palette));
                    self.loaded_maps_lookup.insert(path.clone(), id);
                }
            }
            AppEvent::SetConfigPath { path } => {
                self.config.borrow_mut().celeste_root = Some(path.clone());
                trigger_module_load(cx, path.clone());
            }
            AppEvent::SetLastPath { path } => {
                self.config.borrow_mut().last_filepath = path.clone();
            }
            AppEvent::SetModules { modules } => {
                let mut r = modules.lock();
                std::mem::swap(r.deref_mut(), &mut self.modules);
                self.modules_lookup = build_modules_lookup(&self.modules);
                self.modules_version += 1;
                self.omni_palette = trigger_palette_update(
                    &self.modules,
                    &self.modules_lookup,
                    &mut self.loaded_maps,
                );
            }
            AppEvent::NewMod => {
                let mut number = 1;
                'outer: loop {
                    let name = format!("untitled-{}", number);
                    let path = self
                        .config
                        .celeste_root
                        .clone()
                        .unwrap()
                        .join("Mods")
                        .join(&name);
                    for module in self.modules.values() {
                        if module.everest_metadata.name == name || path.exists() {
                            number += 1;
                            continue 'outer;
                        }
                    }

                    let everest_data = EverestYaml {
                        name,
                        version: EverestModuleVersion(vec![0, 0, 0]),
                        dll: None,
                        dependencies: vec![],
                    };
                    if let Err(e) = std::fs::create_dir(&path) {
                        log::error!("Could not create mod: {}", e);
                        break;
                    }
                    everest_data.save(&path);

                    let mut module_src = ConfigSource::Dir(FolderSource::new(&path).unwrap());
                    let mut module = CelesteModule::new(Some(path), everest_data);
                    module.load(&mut module_src);

                    let module_id = ModuleID::new();
                    step_modules_lookup(
                        &mut self.modules_lookup,
                        &self.modules,
                        module_id,
                        &module,
                    );
                    self.modules.insert(module_id, module);
                    self.modules_version += 1;

                    cx.emit(AppEvent::OpenModuleOverviewTab { module: module_id });
                    break;
                }
            }
            AppEvent::SelectTool { spec } => {
                if let Some(mut tool) = self.current_tool.borrow_mut().take() {
                    for event in tool.switch_off(self, cx) {
                        cx.emit(event);
                    }
                }
                self.current_toolspec = *spec;
                *self.current_tool.borrow_mut() = Some(spec.switch_on(self));
            }
            AppEvent::SelectLayer { layer } => {
                self.current_layer = *layer;
            }
            AppEvent::SelectPaletteTile { fg, tile } => {
                if *fg {
                    self.current_fg_tile = *tile;
                } else {
                    self.current_bg_tile = *tile;
                }
            }
            AppEvent::SelectPaletteObjectTile { tile } => {
                self.current_objtile = *tile;
            }
            AppEvent::SelectPaletteEntity { entity } => {
                self.current_entity = *entity;
            }
            AppEvent::SelectPaletteTrigger { trigger } => {
                self.current_trigger = *trigger;
            }
            AppEvent::SelectPaletteDecal { decal } => {
                self.current_decal = *decal;
            }
            AppEvent::PanObjectTiles { delta } => {
                // TODO limits
                self.objtiles_transform = self.objtiles_transform.pre_translate(*delta);
            }
            AppEvent::ZoomObjectTiles { delta, focus } => {
                self.objtiles_transform = self
                    .objtiles_transform
                    .pre_translate(focus.to_vector())
                    .pre_scale(*delta, *delta)
                    .pre_translate(-focus.to_vector());
            }

            // tab events
            AppEvent::SelectTab { idx } => {
                if *idx < self.tabs.len() {
                    if let Some(mut tool) = self.current_tool.borrow_mut().take() {
                        for event in tool.switch_off(self, cx) {
                            cx.emit(event);
                        }
                    }
                    self.current_tab = *idx;
                    if let Some(AppTab::Map(_)) = self.tabs.get(*idx) {
                        *self.current_tool.borrow_mut() =
                            Some(self.current_toolspec.switch_on(self));
                    }
                }
            }
            AppEvent::CloseTab { idx } => {
                self.poison_tab = *idx;
                self.garbage_collect();
            }
            AppEvent::Pan { tab, delta } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.transform = map_tab.transform.pre_translate(*delta);
                }
            }
            AppEvent::Zoom { tab, delta, focus } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    // TODO scale stepping, high and low limits
                    map_tab.transform = map_tab
                        .transform
                        .pre_translate(focus.to_vector())
                        .pre_scale(*delta, *delta)
                        .pre_translate(-focus.to_vector());
                }
            }
            AppEvent::MovePreview { tab, pos } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.preview_pos = *pos;
                }
            }
            AppEvent::SelectRoom { tab, idx } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.current_room = *idx;
                    if let Some(room) = self.current_room_ref() {
                        room.cache.borrow_mut().render_cache_valid = false;
                    }
                }
            }
            AppEvent::SelectObject { tab, selection } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.current_selected = *selection;
                    if let Some(room) = self.current_room_ref() {
                        room.cache.borrow_mut().render_cache_valid = false;
                    }
                }
            }
            AppEvent::SelectStyleground { tab, styleground } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(*tab) {
                    map_tab.styleground_selected = *styleground;
                }
            }
            AppEvent::SelectSearchScope { tab, scope } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.search_scope = scope.clone();
                }
            }
            AppEvent::SelectSearchFilter { tab, filter } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.search_filter = filter.clone();
                }
            }
            AppEvent::SelectSearchFilterAttributes { tab, filter } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.attribute_filter = filter.clone();
                }
            }
            AppEvent::SelectSearchType { tab, ty } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.search_type = *ty;
                }
            }
            AppEvent::PopulateConfigSearchResults { tab, results } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.search_results = results.clone();
                    ctab.selected_result = 0;
                }
            }
            AppEvent::SelectConfigSearchResult { tab, idx } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.selected_result = *idx;
                    if let Some(result) = ctab.search_results.get(*idx) {
                        ctab.editing_config = Some(result.get_config(&self.omni_palette));
                    } else {
                        ctab.editing_config = None;
                    }
                }
            }
            AppEvent::EditConfig { tab, config } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.editing_config = Some(config.as_ref().to_owned())
                }
            }
            AppEvent::SetConfigErrorMessage { tab, message } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.error_message = message.to_owned();
                }
            }
            AppEvent::MapEvent { map, event } => {
                self.apply_map_event(cx, *map, event);
            }
            AppEvent::ProjectEvent { project, event } => {
                self.apply_project_event(cx, *project, event);
                self.modules_version += 1;
            }
        }
    }
}
