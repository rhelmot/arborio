use crate::data::action::{apply_map_action, MapAction};
use crate::data::app::{step_modules_lookup, AppEvent, AppState};
use crate::data::tabs::AppTab;
use crate::data::{save, EventPhase, MapID, UNDO_BUFFER_SIZE};
use arborio_maploader::map_struct::{
    CelesteMap, CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, CelesteMapMeta,
    CelesteMapMetaAudioState, CelesteMapMetaMode, CelesteMapStyleground, FieldEntry,
};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::everest_yaml::EverestModuleVersion;
use arborio_modloader::module::CelesteModuleKind;
use arborio_modloader::module::{MapPath, ModuleID};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;

#[derive(Lens)]
pub struct MapState {
    pub data: MapStateData,
    pub cache: MapStateCache,
}

#[derive(Lens, Clone)]
pub struct MapStateData {
    pub filler: Vec<MapRectStrict>,
    pub background_color: Option<String>,
    pub foregrounds: Vec<CelesteMapStyleground>,
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<LevelState>,

    pub fg_tiles: String,
    pub bg_tiles: String,
    pub animated_tiles: String,
    pub sprites: String,
    pub portraits: String,
    pub cassette_note_color: String,
    pub title_text_color: String,
    pub title_base_color: String,
    pub title_accent_color: String,
    pub icon: String,
    pub interlude: bool,
    pub wipe: String,
    pub cassette_song: String,
    pub postcard_sound_id: String,

    pub color_grade: String,
    pub dreaming: bool,
    pub intro_type: String,
    pub bloom_base: f32,
    pub bloom_strength: f32,
    pub darkness_alpha: f32,
    pub core_mode: String,

    pub heart_is_end: bool,
    pub inventory: String,
    pub start_level: String,
    pub seeker_slowdown: bool,
    pub theo_in_bubble: bool,
    pub ignore_level_audio_layer_data: bool,

    pub ambience: String,
    pub music: String,
}

pub struct MapStateCache {
    pub dirty: bool,
    pub path: MapPath,
    pub undo_buffer: VecDeque<MapAction>,
    pub redo_buffer: VecDeque<MapAction>,
    pub event_phase: EventPhase,
    pub palette: ModuleAggregate,
}

#[derive(Debug, Default)]
pub struct MapStateUpdate {
    pub fg_tiles: Option<String>,
    pub bg_tiles: Option<String>,
    pub animated_tiles: Option<String>,
    pub sprites: Option<String>,
    pub portraits: Option<String>,
    pub cassette_note_color: Option<String>,
    pub title_text_color: Option<String>,
    pub title_base_color: Option<String>,
    pub title_accent_color: Option<String>,
    pub icon: Option<String>,
    pub interlude: Option<bool>,
    pub wipe: Option<String>,
    pub cassette_song: Option<String>,
    pub postcard_sound_id: Option<String>,

    pub color_grade: Option<String>,
    pub dreaming: Option<bool>,
    pub intro_type: Option<String>,
    pub bloom_base: Option<f32>,
    pub bloom_strength: Option<f32>,
    pub darkness_alpha: Option<f32>,
    pub core_mode: Option<String>,

    pub heart_is_end: Option<bool>,
    pub inventory: Option<String>,
    pub start_level: Option<String>,
    pub seeker_slowdown: Option<bool>,
    pub theo_in_bubble: Option<bool>,
    pub ignore_level_audio_layer_data: Option<bool>,

    pub ambience: Option<String>,
    pub music: Option<String>,
}

impl MapStateData {
    pub fn styles(&self, fg: bool) -> &Vec<CelesteMapStyleground> {
        if fg {
            &self.foregrounds
        } else {
            &self.backgrounds
        }
    }

    pub fn styles_mut(&mut self, fg: bool) -> &mut Vec<CelesteMapStyleground> {
        if fg {
            &mut self.foregrounds
        } else {
            &mut self.backgrounds
        }
    }

    pub fn level_at(&self, pt: MapPointStrict) -> Option<usize> {
        for (idx, room) in self.levels.iter().enumerate() {
            if room.data.bounds.contains(pt) {
                return Some(idx);
            }
        }
        None
    }

    pub fn apply(&mut self, patch: &mut MapStateUpdate) {
        if let Some(x) = patch.fg_tiles.as_mut() {
            std::mem::swap(&mut self.fg_tiles, x);
        }
        if let Some(x) = patch.bg_tiles.as_mut() {
            std::mem::swap(&mut self.bg_tiles, x);
        }
        if let Some(x) = patch.animated_tiles.as_mut() {
            std::mem::swap(&mut self.animated_tiles, x);
        }
        if let Some(x) = patch.sprites.as_mut() {
            std::mem::swap(&mut self.sprites, x);
        }
        if let Some(x) = patch.portraits.as_mut() {
            std::mem::swap(&mut self.portraits, x);
        }
        if let Some(x) = patch.cassette_note_color.as_mut() {
            std::mem::swap(&mut self.cassette_note_color, x);
        }
        if let Some(x) = patch.title_text_color.as_mut() {
            std::mem::swap(&mut self.title_text_color, x);
        }
        if let Some(x) = patch.title_base_color.as_mut() {
            std::mem::swap(&mut self.title_base_color, x);
        }
        if let Some(x) = patch.title_accent_color.as_mut() {
            std::mem::swap(&mut self.title_accent_color, x);
        }
        if let Some(x) = patch.icon.as_mut() {
            std::mem::swap(&mut self.icon, x);
        }
        if let Some(x) = patch.interlude.as_mut() {
            std::mem::swap(&mut self.interlude, x);
        }
        if let Some(x) = patch.wipe.as_mut() {
            std::mem::swap(&mut self.wipe, x);
        }
        if let Some(x) = patch.cassette_song.as_mut() {
            std::mem::swap(&mut self.cassette_song, x);
        }
        if let Some(x) = patch.postcard_sound_id.as_mut() {
            std::mem::swap(&mut self.postcard_sound_id, x);
        }

        if let Some(x) = patch.color_grade.as_mut() {
            std::mem::swap(&mut self.color_grade, x);
        }
        if let Some(x) = patch.dreaming.as_mut() {
            std::mem::swap(&mut self.dreaming, x);
        }
        if let Some(x) = patch.intro_type.as_mut() {
            std::mem::swap(&mut self.intro_type, x);
        }
        if let Some(x) = patch.bloom_base.as_mut() {
            std::mem::swap(&mut self.bloom_base, x);
        }
        if let Some(x) = patch.bloom_strength.as_mut() {
            std::mem::swap(&mut self.bloom_strength, x);
        }
        if let Some(x) = patch.darkness_alpha.as_mut() {
            std::mem::swap(&mut self.darkness_alpha, x);
        }
        if let Some(x) = patch.core_mode.as_mut() {
            std::mem::swap(&mut self.core_mode, x);
        }

        if let Some(x) = patch.heart_is_end.as_mut() {
            std::mem::swap(&mut self.heart_is_end, x);
        }
        if let Some(x) = patch.inventory.as_mut() {
            std::mem::swap(&mut self.inventory, x);
        }
        if let Some(x) = patch.start_level.as_mut() {
            std::mem::swap(&mut self.start_level, x);
        }
        if let Some(x) = patch.seeker_slowdown.as_mut() {
            std::mem::swap(&mut self.seeker_slowdown, x);
        }
        if let Some(x) = patch.theo_in_bubble.as_mut() {
            std::mem::swap(&mut self.theo_in_bubble, x);
        }
        if let Some(x) = patch.ignore_level_audio_layer_data.as_mut() {
            std::mem::swap(&mut self.ignore_level_audio_layer_data, x);
        }

        if let Some(x) = patch.ambience.as_mut() {
            std::mem::swap(&mut self.ambience, x);
        }
        if let Some(x) = patch.music.as_mut() {
            std::mem::swap(&mut self.music, x);
        }
    }
}

impl MapState {
    pub fn styles(&self, fg: bool) -> &Vec<CelesteMapStyleground> {
        self.data.styles(fg)
    }

    pub fn styles_mut(&mut self, fg: bool) -> &mut Vec<CelesteMapStyleground> {
        self.data.styles_mut(fg)
    }

    pub fn level_at(&self, pt: MapPointStrict) -> Option<usize> {
        self.data.level_at(pt)
    }

    pub fn new(x: CelesteMap, path: MapPath, palette: ModuleAggregate) -> Self {
        //let side = path.sid.parse::<SIDFields>().map(|f| f.mode).unwrap_or_default().idx();
        let mut result = Self {
            data: MapStateData {
                filler: x.filler,
                background_color: x.background_color,
                foregrounds: x.foregrounds,
                backgrounds: x.backgrounds,
                levels: x.levels.into_iter().map(|x| x.into()).collect(),
                fg_tiles: "".to_owned(),
                bg_tiles: "".to_owned(),
                animated_tiles: "".to_owned(),
                sprites: "".to_owned(),
                portraits: "".to_owned(),
                cassette_note_color: "".to_owned(),
                title_text_color: "".to_owned(),
                title_base_color: "".to_owned(),
                title_accent_color: "".to_owned(),
                icon: "".to_owned(),
                interlude: false,
                wipe: "".to_string(),
                cassette_song: "".to_string(),
                postcard_sound_id: "".to_string(),
                color_grade: "".to_string(),
                dreaming: false,
                intro_type: "".to_string(),
                bloom_base: 0.0,
                bloom_strength: 0.0,
                darkness_alpha: 0.0,
                core_mode: "".to_string(),
                heart_is_end: false,
                inventory: "".to_string(),
                start_level: "".to_string(),
                seeker_slowdown: false,
                theo_in_bubble: false,
                ignore_level_audio_layer_data: false,
                ambience: "".to_string(),
                music: "".to_string(),
            },
            cache: MapStateCache {
                dirty: false,
                path,
                undo_buffer: Default::default(),
                redo_buffer: Default::default(),
                event_phase: EventPhase::null(),
                palette,
            },
        };
        if let Some(meta) = x.meta {
            result.data.apply(&mut MapStateUpdate::new(meta, None))
        }
        result
    }
}

impl From<MapStateData> for CelesteMap {
    fn from(sself: MapStateData) -> Self {
        let meta = sself.clone_meta();
        CelesteMap {
            filler: sself.filler,
            background_color: sself.background_color,
            foregrounds: sself.foregrounds,
            backgrounds: sself.backgrounds,
            levels: sself.levels.into_iter().map(|x| x.into()).collect(),
            meta: Some(meta),
        }
    }
}

impl MapStateData {
    pub fn clone_meta(&self) -> CelesteMapMeta {
        CelesteMapMeta {
            override_aside_meta: Some(true),
            color_grade: Some(self.color_grade.clone()),
            dreaming: Some(self.dreaming),
            fg_tiles: Some(self.fg_tiles.clone()),
            bg_tiles: Some(self.bg_tiles.clone()),
            animated_tiles: Some(self.animated_tiles.clone()),
            sprites: Some(self.sprites.clone()),
            portraits: Some(self.portraits.clone()),
            intro_type: Some(self.intro_type.clone()),
            cassette_note_color: Some(self.cassette_note_color.clone()),
            title_text_color: Some(self.title_text_color.clone()),
            title_base_color: Some(self.title_base_color.clone()),
            title_accent_color: Some(self.title_accent_color.clone()),
            icon: Some(self.icon.clone()),
            interlude: Some(self.interlude),
            wipe: Some(self.wipe.clone()),
            bloom_base: Some(self.bloom_base),
            bloom_strength: Some(self.bloom_strength),
            darkness_alpha: Some(self.darkness_alpha),
            cassette_song: Some(self.cassette_song.clone()),
            core_mode: Some(self.core_mode.clone()),
            postcard_sound_id: Some(self.postcard_sound_id.clone()),
            mode: Some(CelesteMapMetaMode {
                heart_is_end: Some(self.heart_is_end),
                inventory: Some(self.inventory.clone()),
                start_level: Some(self.start_level.clone()),
                seeker_slowdown: Some(self.seeker_slowdown),
                theo_in_bubble: Some(self.theo_in_bubble),
                ignore_level_audio_layer_data: Some(self.ignore_level_audio_layer_data),
                audio_state: Some(CelesteMapMetaAudioState {
                    ambience: self.ambience.clone(),
                    music: self.music.clone(),
                }),
            }),
            modes: None,
        }
    }
}

impl MapStateUpdate {
    pub fn new(x: CelesteMapMeta, mode_num: Option<usize>) -> Self {
        let CelesteMapMeta {
            override_aside_meta: _,
            color_grade,
            dreaming,
            fg_tiles,
            bg_tiles,
            animated_tiles,
            sprites,
            portraits,
            intro_type,
            cassette_note_color,
            title_text_color,
            title_base_color,
            title_accent_color,
            icon,
            interlude,
            wipe,
            bloom_base,
            bloom_strength,
            darkness_alpha,
            cassette_song,
            core_mode,
            postcard_sound_id,
            mode,
            modes,
        } = x;
        let mode = mode.unwrap_or_default();
        let modebase = mode_num
            .map(|m| modes.unwrap_or_default().remove(m))
            .unwrap_or_default();
        let audiostate = mode.audio_state.or(modebase.audio_state);
        let (music, ambience) = match audiostate {
            Some(CelesteMapMetaAudioState { ambience, music }) => (Some(music), Some(ambience)),
            None => (None, None),
        };
        Self {
            fg_tiles,
            bg_tiles,
            animated_tiles,
            sprites,
            portraits,
            cassette_note_color,
            title_text_color,
            title_base_color,
            title_accent_color,
            icon,
            interlude,
            wipe,
            cassette_song,
            postcard_sound_id,
            color_grade,
            dreaming,
            intro_type,
            bloom_base,
            bloom_strength,
            darkness_alpha,
            core_mode,
            heart_is_end: mode.heart_is_end.or(modebase.heart_is_end),
            inventory: mode.inventory.or(modebase.inventory),
            start_level: mode.start_level.or(modebase.start_level),
            seeker_slowdown: mode.seeker_slowdown.or(modebase.seeker_slowdown),
            theo_in_bubble: mode.theo_in_bubble.or(modebase.theo_in_bubble),
            ignore_level_audio_layer_data: mode
                .ignore_level_audio_layer_data
                .or(modebase.ignore_level_audio_layer_data),
            music,
            ambience,
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
        let module = if let Some(module) = self.modules.get_mut(&state.cache.path.module) {
            module
        } else {
            log::error!("Internal error: loaded map referring to unloaded module");
            return;
        };

        match event {
            MapEvent::Action { event, merge_phase } => {
                if let Some(event) = event.borrow_mut().take() {
                    match apply_map_action(cx, state, event) {
                        Ok(undo) => {
                            cx.needs_redraw();
                            state.cache.dirty = true;
                            if state.cache.undo_buffer.len() == UNDO_BUFFER_SIZE {
                                state.cache.undo_buffer.pop_front();
                            }
                            if state.cache.undo_buffer.back().is_none()
                                || state.cache.event_phase != *merge_phase
                            {
                                state.cache.undo_buffer.push_back(undo);
                            }
                            state.cache.event_phase = *merge_phase;
                            state.cache.redo_buffer.clear();
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
                if let Some(event) = state.cache.undo_buffer.pop_back() {
                    match apply_map_action(cx, state, event) {
                        Ok(opposite) => {
                            cx.needs_redraw();
                            state.cache.dirty = true;
                            state.cache.redo_buffer.push_back(opposite);
                            state.cache.event_phase = EventPhase::null();
                        }
                        Err(e) => {
                            log::error!("Internal error: Failed to undo: {}", e);
                        }
                    }
                }
            }
            MapEvent::Redo => {
                if let Some(event) = state.cache.redo_buffer.pop_back() {
                    match apply_map_action(cx, state, event) {
                        Ok(opposite) => {
                            cx.needs_redraw();
                            state.cache.dirty = true;
                            state.cache.undo_buffer.push_back(opposite);
                            state.cache.event_phase = EventPhase::null();
                        }
                        Err(e) => {
                            log::error!("Internal error: Failed to redo: {}", e);
                        }
                    }
                }
            }
            MapEvent::Save => match save(module, &state.cache.path, &state.data.clone().into()) {
                Ok(_) => state.cache.dirty = false,
                Err(e) => log::error!("Failed to save: {}", e),
            },
            MapEvent::SetName { sid } => {
                let current_sid = &state.cache.path.sid;
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
                state.cache.path.sid = sid.clone();
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
                        .filter_map(|(i, sid)| (sid == &state.cache.path.sid).then_some(i))
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
                        .join(&state.cache.path.sid)
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

#[derive(Debug, Default, Lens)]
pub struct LevelState {
    pub data: CelesteMapLevel,
    pub cache: RefCell<LevelStateCache>,
}

#[derive(Default)]
pub struct LevelStateCache {
    pub render_cache_valid: bool,
    pub render_cache: Option<vg::ImageId>,
    pub last_entity_idx: usize,
    pub last_decal_idx: usize,
}

impl From<CelesteMapLevel> for LevelState {
    fn from(data: CelesteMapLevel) -> Self {
        Self {
            data,
            cache: Default::default(),
        }
    }
}

impl Debug for LevelStateCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveMapLevelCache")
            .field("render_cache_valid", &self.render_cache_valid)
            .field("last_entity_idx", &self.last_entity_idx)
            .finish()
    }
}

impl Clone for LevelState {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            cache: RefCell::new(LevelStateCache::default()),
        }
    }
}

impl From<LevelState> for CelesteMapLevel {
    fn from(sself: LevelState) -> Self {
        sself.data
    }
}

impl LevelState {
    pub fn tile(&self, pt: TilePoint, foreground: bool) -> Option<char> {
        self.data.tile(pt, foreground)
    }

    pub fn next_id(&self) -> i32 {
        self.data.next_id()
    }

    pub fn occupancy_field(&self) -> TileGrid<FieldEntry> {
        self.data.occupancy_field()
    }

    pub fn room_bounds(&self) -> RoomRect {
        self.data.room_bounds()
    }

    pub fn entity(&self, id: i32, trigger: bool) -> Option<&CelesteMapEntity> {
        let entities = if trigger {
            &self.data.triggers
        } else {
            &self.data.entities
        };
        if let Some(e) = entities.get(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                return Some(e);
            }
        }
        for (idx, e) in entities.iter().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_entity_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn entity_mut(&mut self, id: i32, trigger: bool) -> Option<&mut CelesteMapEntity> {
        let entities = if trigger {
            &mut self.data.triggers
        } else {
            &mut self.data.entities
        };
        if let Some(e) = entities.get_mut(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                // hack around borrow checker
                let entities = if trigger {
                    &mut self.data.triggers
                } else {
                    &mut self.data.entities
                };
                return entities.get_mut(self.cache.borrow().last_entity_idx);
            }
        }
        let entities = if trigger {
            &mut self.data.triggers
        } else {
            &mut self.data.entities
        };
        for (idx, e) in entities.iter_mut().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_entity_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn decal(&self, id: u32, fg: bool) -> Option<&CelesteMapDecal> {
        let decals = if fg {
            &self.data.fg_decals
        } else {
            &self.data.bg_decals
        };
        if let Some(e) = decals.get(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                return Some(e);
            }
        }
        for (idx, e) in decals.iter().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_decal_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn decal_mut(&mut self, id: u32, fg: bool) -> Option<&mut CelesteMapDecal> {
        let decals = if fg {
            &mut self.data.fg_decals
        } else {
            &mut self.data.bg_decals
        };
        if let Some(e) = decals.get_mut(self.cache.borrow().last_decal_idx) {
            if e.id == id {
                // hack around borrow checker
                let decals = if fg {
                    &mut self.data.fg_decals
                } else {
                    &mut self.data.bg_decals
                };
                return decals.get_mut(self.cache.borrow().last_decal_idx);
            }
        }
        let decals = if fg {
            &mut self.data.fg_decals
        } else {
            &mut self.data.bg_decals
        };
        for (idx, e) in decals.iter_mut().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_decal_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn cache_entity_idx(&self, idx: usize) {
        self.cache.borrow_mut().last_entity_idx = idx;
    }

    pub fn cache_decal_idx(&self, idx: usize) {
        self.cache.borrow_mut().last_decal_idx = idx;
    }
}
