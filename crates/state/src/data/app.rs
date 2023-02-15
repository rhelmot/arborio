use crate::data::action::{MapAction, RoomAction, StylegroundSelection};
use arborio_maploader::map_struct::{CelesteMap, CelesteMapEntity};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::discovery::LoaderThreadMessage;
use arborio_modloader::module::{CelesteModule, MapPath, ModuleID, CELESTE_MODULE_ID};
use arborio_modloader::selectable::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time;

use crate::auto_saver::AutoSaver;
use crate::data::config_editor::{
    AnyConfig, ConfigSearchFilter, ConfigSearchResult, ConfigSearchType, SearchScope,
};
use crate::data::project_map::{LevelState, MapEvent, MapState, ProjectEvent};
use crate::data::selection::AppSelection;
use crate::data::tabs::{AppTab, MapTab};
use crate::data::{AppConfig, AppConfigSetter, ArborioRecord, EventPhase, Layer, MapID, Progress};
use crate::tools::{Tool, ToolSpec};

#[derive(Lens)]
pub struct AppState {
    pub config: AutoSaver<AppConfig>,
    pub loading_tx: Sender<LoaderThreadMessage>,
    pub sugar_mod: Option<PathBuf>,

    pub modules: HashMap<ModuleID, CelesteModule>,
    pub modules_lookup: HashMap<String, ModuleID>,
    pub modules_version: u32,
    pub omni_palette: ModuleAggregate,
    pub loaded_maps: HashMap<MapID, MapState>,
    pub loaded_maps_lookup: HashMap<MapPath, MapID>,

    pub current_tab: usize,
    pub tabs: Vec<AppTab>,
    pub poison_tab: usize,

    pub current_toolspec: ToolSpec,
    pub current_tool: RefCell<Option<Box<dyn Tool>>>,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_fg_tile_other: String,
    pub current_bg_tile: TileSelectable,
    pub current_bg_tile_other: String,
    pub current_entity: EntitySelectable,
    pub current_entity_other: String,
    pub current_trigger: TriggerSelectable,
    pub current_trigger_other: String,
    pub current_decal: DecalSelectable,
    pub current_decal_other: String,
    pub current_objtile: u32,
    pub objtiles_transform: MapToScreen,

    pub last_draw: RefCell<time::Instant>, // mutable to draw
    pub progress: Progress,
    pub logs: Vec<ArborioRecord>,
    pub error_message: String,
}

#[derive(Debug)]
pub enum AppEvent {
    Log {
        message: ArborioRecord,
    },
    Progress {
        progress: Progress,
    },
    SetClipboard {
        contents: String,
    },
    EditSettings {
        setter: AppConfigSetter,
    },
    SetModules {
        modules: HashMap<ModuleID, CelesteModule>,
    },
    UpdateModules {
        modules: HashMap<ModuleID, Option<CelesteModule>>,
    },
    OpenModuleOverviewTab {
        module: ModuleID,
    },
    OpenMap {
        path: MapPath,
    },
    LoadMap {
        path: MapPath,
        map: RefCell<Option<Box<CelesteMap>>>,
    },
    OpenInstallationTab,
    OpenConfigEditorTab,
    OpenLogsTab,
    SelectTab {
        idx: usize,
    },
    CloseTab {
        idx: usize,
    },
    NewMod,
    MovePreview {
        tab: usize,
        pos: MapPointStrict,
    },
    Pan {
        tab: usize,
        delta: MapVectorPrecise,
    },
    Zoom {
        tab: usize,
        delta: f32,
        focus: MapPointPrecise,
    },
    PanObjectTiles {
        delta: MapVectorPrecise,
    },
    ZoomObjectTiles {
        delta: f32,
        focus: MapPointPrecise,
    },
    SelectTool {
        spec: ToolSpec,
    },
    SelectSearchScope {
        tab: usize,
        scope: SearchScope,
    },
    SelectSearchType {
        tab: usize,
        ty: ConfigSearchType,
    },
    SelectSearchFilter {
        tab: usize,
        filter: ConfigSearchFilter,
    },
    SelectSearchFilterAttributes {
        tab: usize,
        filter: String,
    },
    PopulateConfigSearchResults {
        tab: usize,
        results: Vec<ConfigSearchResult>,
    },
    SelectConfigSearchResult {
        tab: usize,
        idx: usize,
    },
    EditConfig {
        tab: usize,
        config: Box<AnyConfig>,
    },
    EditPreviewEntity {
        tab: usize,
        entity: CelesteMapEntity,
    },
    SetConfigErrorMessage {
        tab: usize,
        message: String,
    },
    SelectStyleground {
        tab: usize,
        styleground: Option<StylegroundSelection>,
    },
    SelectRoom {
        tab: usize,
        idx: usize,
    },
    SelectLayer {
        layer: Layer,
    },
    SelectPaletteTile {
        fg: bool,
        tile: TileSelectable,
    },
    SelectPaletteTileOther {
        fg: bool,
        other: String,
    },
    SelectPaletteObjectTile {
        tile: u32,
    },
    SelectPaletteEntity {
        entity: EntitySelectable,
    },
    SelectPaletteEntityOther {
        other: String,
    },
    SelectPaletteTrigger {
        trigger: TriggerSelectable,
    },
    SelectPaletteTriggerOther {
        other: String,
    },
    SelectPaletteDecal {
        decal: DecalSelectable,
    },
    SelectPaletteDecalOther {
        other: String,
    },
    ClearSelection {
        tab: usize,
    },
    SelectObjects {
        tab: usize,
        selection: HashSet<AppSelection>,
    },
    DeselectObjects {
        tab: usize,
        selection: HashSet<AppSelection>,
    },
    MapEvent {
        map: Option<MapID>,
        event: MapEvent,
    },
    ProjectEvent {
        project: Option<ModuleID>,
        event: ProjectEvent,
    },
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AppInternalEvent {
    SelectMeEntity { id: i32, trigger: bool },
    SelectMeDecal { id: u32, fg: bool },
    SelectMeRoom { idx: usize },
}

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if let Some(app_event) = event.take() {
            self.apply(cx, app_event);
        };
    }
}

impl AppState {
    pub fn new(tx: Sender<LoaderThreadMessage>) -> AppState {
        let mut cfg: AppConfig = confy::load("arborio", "arborio").unwrap_or_default();
        if !cfg
            .celeste_root
            .as_ref()
            .map(|root| root.is_dir())
            .unwrap_or_default()
        {
            cfg.celeste_root = None;
        }
        let cfg = AutoSaver::new(cfg, |cfg: &mut AppConfig| {
            confy::store("arborio", "arborio", cfg)
                .unwrap_or_else(|e| panic!("Failed to save config file: {e}"));
        });

        AppState {
            config: cfg,
            loading_tx: tx,
            sugar_mod: None,
            current_tab: 0,
            poison_tab: usize::MAX,
            tabs: vec![AppTab::CelesteOverview],
            loaded_maps: HashMap::new(),
            loaded_maps_lookup: HashMap::new(),
            current_toolspec: ToolSpec::Selection,
            current_tool: RefCell::new(None),
            current_fg_tile: TileSelectable::default(),
            current_fg_tile_other: "".to_owned(),
            current_bg_tile: TileSelectable::default(),
            current_bg_tile_other: "".to_owned(),
            current_entity: EntitySelectable::default(),
            current_entity_other: "".to_owned(),
            current_trigger: TriggerSelectable::default(),
            current_trigger_other: "".to_owned(),
            current_decal: DecalSelectable::default(),
            current_decal_other: "".to_owned(),
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::FgTiles,
            current_objtile: 0,
            objtiles_transform: MapToScreen::identity(),

            modules: HashMap::new(),
            modules_lookup: HashMap::new(),
            modules_version: 0,
            omni_palette: ModuleAggregate::new(
                &HashMap::new(),
                &HashMap::new(),
                &None,
                *CELESTE_MODULE_ID,
                false,
            ),
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
            logs: vec![],
            error_message: "".to_owned(),
        }
    }

    // a debugging stopgap
    pub fn map_tab_check(&self) -> bool {
        matches!(self.tabs.get(self.current_tab), Some(AppTab::Map(_)))
    }

    // intended mainly for use in tools. can we maybe do better?
    pub fn map_tab_unwrap(&self) -> &MapTab {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            result
        } else {
            panic!("misuse of map_tab_unwrap");
        }
    }

    pub(crate) fn rebuild_modules_bookkeeping(&mut self) {
        // bump binding version
        self.modules_version += 1;

        // rebuild modules lookup
        self.modules_lookup.clear();
        for (id, module) in self.modules.iter() {
            step_modules_lookup(&mut self.modules_lookup, &self.modules, *id, module);
        }

        // rebuild palettes
        for state in self.loaded_maps.values_mut() {
            state.cache.palette = ModuleAggregate::new(
                &self.modules,
                &self.modules_lookup,
                &Some(state.data.clone_meta()),
                state.cache.path.module,
                true,
            );
        }
        self.omni_palette = ModuleAggregate::new_omni(&self.modules, false) // discard logs
    }

    pub fn current_project_id(&self) -> Option<ModuleID> {
        match self.tabs.get(self.current_tab) {
            Some(AppTab::ProjectOverview(id)) => Some(*id),
            Some(AppTab::Map(maptab)) => {
                Some(self.loaded_maps.get(&maptab.id).unwrap().cache.path.module)
            }
            _ => None,
        }
    }

    pub fn current_palette_unwrap(&self) -> &ModuleAggregate {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            &self
                .loaded_maps
                .get(&result.id)
                .expect("stale reference")
                .cache
                .palette
        } else if let Some(AppTab::ConfigEditor(_)) = self.tabs.get(self.current_tab) {
            &self.omni_palette
        } else {
            panic!("misuse of current_palette_unwrap");
        }
    }

    pub fn current_map_id(&self) -> Option<MapID> {
        if let Some(tab) = self.tabs.get(self.current_tab) {
            match tab {
                AppTab::Map(maptab) => Some(maptab.id),
                AppTab::MapMeta(mapid) => Some(*mapid),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn current_map_ref(&self) -> Option<&MapState> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps.get(&maptab.id)
        } else {
            None
        }
    }

    pub fn current_room_ref(&self) -> Option<&LevelState> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps
                .get(&maptab.id)
                .and_then(|map| map.data.levels.get(maptab.current_room))
        } else {
            None
        }
    }

    pub fn garbage_collect(&mut self) {
        // destroy any tabs related to resources which no longer exist or are marked for closure
        // compute the new current-tab index
        let mut idx = 0;
        let mut current_delta: usize = 0;
        self.tabs.retain(|tab| {
            let closure = |idx: usize, tab: &AppTab| -> bool {
                if idx == self.poison_tab {
                    return false;
                }

                match tab {
                    AppTab::ProjectOverview(project) => self.modules.contains_key(project),
                    AppTab::Map(MapTab { id, .. }) | AppTab::MapMeta(id) => {
                        if let Some(x) = self.loaded_maps.get(id) {
                            self.modules.contains_key(&x.cache.path.module)
                        } else {
                            false
                        }
                    }
                    _ => true,
                }
            };

            let result = closure(idx, tab);
            if !result && self.current_tab >= idx {
                current_delta += 1;
            }
            idx += 1;
            result
        });
        self.current_tab = self.current_tab.saturating_sub(current_delta);
        self.poison_tab = usize::MAX;

        // collect a list of maps which need to be retained in memory based on open tabs
        let mut open_maps = HashSet::new();
        for tab in &self.tabs {
            match tab {
                AppTab::Map(MapTab { id, .. }) => {
                    open_maps.insert(*id);
                }
                AppTab::MapMeta(id) => {
                    open_maps.insert(*id);
                }
                _ => {}
            }
        }
        self.loaded_maps.retain(|id, _| open_maps.contains(id));
        self.loaded_maps_lookup
            .retain(|_, id| open_maps.contains(id));
    }

    pub fn map_action(&self, event: Vec<MapAction>, merge_phase: EventPhase) -> AppEvent {
        AppEvent::MapEvent {
            map: Some(self.map_tab_unwrap().id),
            event: MapEvent::Action { merge_phase, event },
        }
    }

    pub fn map_action_unique(&self, event: Vec<MapAction>) -> AppEvent {
        self.map_action(event, EventPhase::new())
    }

    pub fn room_action(&self, event: RoomAction, merge_phase: EventPhase) -> AppEvent {
        self.room_action_explicit(event, merge_phase, self.map_tab_unwrap().current_room)
    }

    pub fn room_action_explicit(
        &self,
        event: RoomAction,
        merge_phase: EventPhase,
        room: usize,
    ) -> AppEvent {
        self.map_action(
            vec![MapAction::RoomAction { idx: room, event }],
            merge_phase,
        )
    }

    // pub fn room_event_unique(&self, event: RoomAction) -> AppEvent {
    //     self.room_action(event, EventPhase::new())
    // }

    // pub fn room_event_unique_explicit(&self, event: RoomAction, room: usize) -> AppEvent {
    //     self.room_action_explicit(event, EventPhase::new(), room)
    // }

    pub fn batch_action(
        &self,
        events: impl IntoIterator<Item = MapAction>,
        merge_phase: EventPhase,
    ) -> AppEvent {
        self.map_action(events.into_iter().collect(), merge_phase)
    }

    pub fn batch_action_unique(&self, events: impl IntoIterator<Item = MapAction>) -> AppEvent {
        self.batch_action(events, EventPhase::new())
    }
}

pub fn step_modules_lookup(
    lookup: &mut HashMap<String, ModuleID>,
    modules: &HashMap<ModuleID, CelesteModule>,
    id: ModuleID,
    module: &CelesteModule,
) {
    match lookup.entry(module.everest_metadata.name.clone()) {
        Entry::Occupied(mut e) => {
            let path_existing = modules.get(e.get()).unwrap().filesystem_root.as_ref();
            let path_new = module.filesystem_root.as_ref();
            let ext_existing = path_existing
                .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                .and_then(|ext| ext.to_str());
            let ext_new = path_new
                .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                .and_then(|ext| ext.to_str());
            if ext_existing == Some("zip") && ext_new == Some("") {
                log::info!(
                    "Conflict between {} and {}, picked latter",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
                e.insert(id);
            } else if ext_existing == Some("") && ext_new == Some("zip") {
                log::info!(
                    "Conflict between {} and {}, picked former",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
            } else {
                log::warn!(
                    "Conflict between {} and {}, picked latter",
                    path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                    path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                );
            }
        }
        Entry::Vacant(v) => {
            v.insert(id);
        }
    }
}
