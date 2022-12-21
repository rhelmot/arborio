use crate::data::app::{build_modules_lookup, step_modules_lookup, AppEvent, AppState};
use crate::data::config_editor::ConfigSearchResult;
use crate::data::project_map::{MapEvent, MapState};
use crate::data::tabs::{AppTab, ConfigEditorTab, MapTab};
use crate::data::{load_map, trigger_module_load, trigger_palette_update, AppConfigSetter, MapID};
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
    pub fn apply(&mut self, cx: &mut EventContext, event: AppEvent) {
        match event {
            // global events
            AppEvent::Log { message } => {
                if message.level <= Level::Error {
                    self.error_message = message.message.clone();
                }
                self.logs.push(message);
            }
            AppEvent::Progress { progress } => {
                self.progress = progress;
            }
            AppEvent::SetClipboard { contents } => {
                cx.set_clipboard(contents)
                    .unwrap_or_else(|e| log::error!("Failed to copy: {}", e));
            }
            AppEvent::OpenModuleOverviewTab { module } => {
                for (i, tab) in self.tabs.iter().enumerate() {
                    if matches!(tab, AppTab::ProjectOverview(m) if *m == module) {
                        cx.emit(AppEvent::SelectTab { idx: i });
                        return;
                    }
                }
                self.tabs.push(AppTab::ProjectOverview(module));
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
                    if matches!(tab, AppTab::Map(maptab) if self.loaded_maps.get(&maptab.id).unwrap().cache.path == path)
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
                    let id = self
                        .loaded_maps_lookup
                        .get(&path)
                        .copied()
                        .unwrap_or_else(MapID::new);
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
                    self.loaded_maps_lookup.insert(path, id);
                }
            }
            AppEvent::EditSettings { setter } => {
                if let AppConfigSetter::CelesteRoot(Some(root)) = &setter {
                    trigger_module_load(cx, root.clone());
                }
                setter.apply(&mut self.config.borrow_mut());
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
                self.current_toolspec = spec;
                *self.current_tool.borrow_mut() = Some(spec.switch_on(self));
            }
            AppEvent::SelectLayer { layer } => {
                self.current_layer = layer;
            }
            AppEvent::SelectPaletteTile { fg, tile } => {
                if fg {
                    self.current_fg_tile = tile;
                } else {
                    self.current_bg_tile = tile;
                }
            }
            AppEvent::SelectPaletteObjectTile { tile } => {
                self.current_objtile = tile;
            }
            AppEvent::SelectPaletteEntity { entity } => {
                self.current_entity = entity;
            }
            AppEvent::SelectPaletteTrigger { trigger } => {
                self.current_trigger = trigger;
            }
            AppEvent::SelectPaletteDecal { decal } => {
                self.current_decal = decal;
            }
            AppEvent::PanObjectTiles { delta } => {
                // TODO limits
                self.objtiles_transform = self.objtiles_transform.pre_translate(delta);
            }
            AppEvent::ZoomObjectTiles { delta, focus } => {
                self.objtiles_transform = self
                    .objtiles_transform
                    .pre_translate(focus.to_vector())
                    .pre_scale(delta, delta)
                    .pre_translate(-focus.to_vector());
            }

            // tab events
            AppEvent::SelectTab { idx } => {
                if idx < self.tabs.len() {
                    if let Some(mut tool) = self.current_tool.borrow_mut().take() {
                        for event in tool.switch_off(self, cx) {
                            cx.emit(event);
                        }
                    }
                    self.current_tab = idx;
                    if let Some(AppTab::Map(_)) = self.tabs.get(idx) {
                        *self.current_tool.borrow_mut() =
                            Some(self.current_toolspec.switch_on(self));
                    }
                }
            }
            AppEvent::CloseTab { idx } => {
                self.poison_tab = idx;
                self.garbage_collect();
            }
            AppEvent::Pan { tab, delta } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    map_tab.transform = map_tab.transform.pre_translate(delta);
                }
            }
            AppEvent::Zoom { tab, delta, focus } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    // TODO scale stepping, high and low limits
                    map_tab.transform = map_tab
                        .transform
                        .pre_translate(focus.to_vector())
                        .pre_scale(delta, delta)
                        .pre_translate(-focus.to_vector());
                }
            }
            AppEvent::MovePreview { tab, pos } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    map_tab.preview_pos = pos;
                }
            }
            AppEvent::SelectRoom { tab, idx } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    map_tab.current_room = idx;
                    if let Some(room) = self.current_room_ref() {
                        room.cache.borrow_mut().render_cache_valid = false;
                    }
                }
            }
            AppEvent::SelectObject { tab, selection } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    map_tab.current_selected = selection;
                    if let Some(room) = self.current_room_ref() {
                        room.cache.borrow_mut().render_cache_valid = false;
                    }
                }
            }
            AppEvent::SelectStyleground { tab, styleground } => {
                if let Some(AppTab::Map(map_tab)) = self.tabs.get_mut(tab) {
                    map_tab.styleground_selected = styleground;
                }
            }
            AppEvent::SelectSearchScope { tab, scope } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.search_scope = scope;
                }
            }
            AppEvent::SelectSearchFilter { tab, filter } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.search_filter = filter;
                }
            }
            AppEvent::SelectSearchFilterAttributes { tab, filter } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.attribute_filter = filter;
                }
            }
            AppEvent::SelectSearchType { tab, ty } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.search_type = ty;
                }
            }
            AppEvent::PopulateConfigSearchResults { tab, results } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.search_results = results;
                    ctab.selected_result = 0;
                }
            }
            AppEvent::SelectConfigSearchResult { tab, idx } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.selected_result = idx;
                    if let Some(result) = ctab.search_results.get(idx) {
                        ctab.editing_config = Some(result.get_config(&self.omni_palette));
                        if let ConfigSearchResult::Entity(e) = result {
                            let vec = e.examples.lock();
                            ctab.preview_entity = vec
                                .get(rand::random::<usize>() % vec.len())
                                .unwrap()
                                .0
                                .clone();
                            let offset =
                                RoomVector::new(ctab.preview_entity.x, ctab.preview_entity.y);
                            ctab.preview_entity.x = 0;
                            ctab.preview_entity.y = 0;
                            for node in ctab.preview_entity.nodes.iter_mut() {
                                node.x -= offset.x;
                                node.y -= offset.y;
                            }
                        }
                    } else {
                        ctab.editing_config = None;
                    }
                } else {
                    log::error!(
                        "Internal error: SelectConfigSearchResult: not a config editor tab"
                    );
                }
            }
            AppEvent::EditConfig { tab, config } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.editing_config = Some(*config)
                }
            }
            AppEvent::SetConfigErrorMessage { tab, message } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(tab) {
                    ctab.error_message = message;
                }
            }
            AppEvent::MapEvent { map, event } => {
                let mut needs_tool_cycle =
                    matches!(event, MapEvent::Undo | MapEvent::Redo | MapEvent::Save);
                if needs_tool_cycle {
                    let tool = self.current_tool.borrow_mut().take();
                    if let Some(mut tool) = tool {
                        for event in tool.switch_off(self, cx) {
                            self.apply(cx, event);
                        }
                    } else {
                        needs_tool_cycle = false;
                    }
                }
                self.apply_map_event(cx, map, event);
                if needs_tool_cycle {
                    *self.current_tool.borrow_mut() = Some(self.current_toolspec.switch_on(self));
                }
            }
            AppEvent::ProjectEvent { project, event } => {
                self.apply_project_event(cx, project, event);
                self.modules_version += 1;
            }
            AppEvent::EditPreviewEntity { tab, entity } => {
                if let Some(AppTab::ConfigEditor(ConfigEditorTab { preview_entity, .. })) =
                    self.tabs.get_mut(tab)
                {
                    *preview_entity = entity;
                    cx.needs_redraw();
                } else {
                    log::error!("Internal error: EditPreviewEntity targeted at nonexistent or non-config tab")
                }
            }
        }
    }
}
