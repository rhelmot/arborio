use dialog::DialogBox;
use log::error;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Formatter;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time;
use vizia::prelude::*;

use crate::assets::{next_uuid, Interned, InternedMap};
use crate::auto_saver::AutoSaver;
use crate::celeste_mod::aggregate::ModuleAggregate;
use crate::celeste_mod::discovery;
use crate::celeste_mod::everest_yaml::{EverestModuleVersion, EverestYaml};
use crate::celeste_mod::module::{CelesteModule, CelesteModuleKind};
use crate::celeste_mod::walker::{ConfigSource, FolderSource};
use crate::map_struct::{
    CelesteMap, CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, CelesteMapLevelUpdate,
    CelesteMapStyleground, MapID, MapPath,
};
use crate::tools::{Tool, ToolSpec};
use crate::units::*;
use crate::widgets::list_palette::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use crate::widgets::tabs::config_editor::{AnyConfig, ConfigSearchResult};

const UNDO_BUFFER_SIZE: usize = 1000;

crate::uuid_cls!(EventPhase);

#[derive(Lens)]
pub struct AppState {
    pub config: AutoSaver<AppConfig>,

    pub modules: InternedMap<CelesteModule>,
    pub modules_version: u32,
    pub palettes: HashMap<MapID, ModuleAggregate>,
    pub omni_palette: ModuleAggregate,
    // TODO merge all these fuckers
    pub loaded_maps: HashMap<MapID, CelesteMap>,
    pub loaded_maps_path_to_id: HashMap<MapPath, MapID>, // except for you. you can stay.
    pub loaded_maps_id_to_path: HashMap<MapID, MapPath>,
    pub loaded_maps_undo_buffer: HashMap<MapID, VecDeque<MapEvent>>,
    pub loaded_maps_redo_buffer: HashMap<MapID, VecDeque<MapEvent>>,
    pub loaded_maps_event_phase: HashMap<MapID, EventPhase>,

    pub current_tab: usize,
    pub tabs: Vec<AppTab>,
    pub poison_tab: usize,

    pub current_toolspec: ToolSpec,
    pub current_tool: RefCell<Option<Box<dyn Tool>>>,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_bg_tile: TileSelectable,
    pub current_entity: EntitySelectable,
    pub current_trigger: TriggerSelectable,
    pub current_decal: DecalSelectable,
    pub current_objtile: u32,
    pub objtiles_transform: MapToScreen,

    pub draw_interval: f32,
    pub snap: bool,

    pub last_draw: RefCell<time::Instant>, // mutable to draw
    pub progress: Progress,
}

#[derive(Serialize, Deserialize, Default, Lens, Debug)]
pub struct AppConfig {
    pub celeste_root: Option<PathBuf>,
    pub last_filepath: PathBuf,
    pub user_name: String,
}

#[allow(clippy::large_enum_variant)] // this is very rarely passed around by value
#[derive(PartialEq, Eq, Debug, Lens, Clone)]
pub enum AppTab {
    CelesteOverview,
    ProjectOverview(Interned),
    Map(MapTab),
    ConfigEditor(ConfigEditorTab),
    Logs,
}

#[derive(Debug, Lens, Clone)]
pub struct ConfigEditorTab {
    pub nonce: u32,
    pub search_scope: SearchScope,
    pub search_type: ConfigSearchType,
    pub search_filter: ConfigSearchFilter,
    pub search_results: Vec<ConfigSearchResult>,
    pub selected_result: usize,
    pub attribute_filter: String,
    pub editing_config: Option<AnyConfig>,
    pub error_message: String,
}

impl Default for ConfigEditorTab {
    fn default() -> Self {
        Self {
            nonce: next_uuid(),
            search_scope: SearchScope::AllOpenMods,
            search_type: ConfigSearchType::Entities,
            search_filter: ConfigSearchFilter::All,
            search_results: vec![],
            selected_result: 0,
            attribute_filter: "originX,originY".to_owned(),
            editing_config: None,
            error_message: "".to_owned(),
        }
    }
}

impl PartialEq for ConfigEditorTab {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce
    }
}

impl Eq for ConfigEditorTab {}

#[derive(Debug, PartialEq, Eq, Data, Clone)]
pub enum SearchScope {
    AllMods,
    AllOpenMods,
    AllOpenMaps,
    Mod(Interned),
    Map(MapPath),
}

impl std::fmt::Display for SearchScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchScope::AllMods => write!(f, "All Installed Mods"),
            SearchScope::AllOpenMods => write!(f, "All Open Mods"),
            SearchScope::AllOpenMaps => write!(f, "All Open Maps"),
            SearchScope::Mod(s) => write!(f, "{}", s),
            SearchScope::Map(s) => write!(f, "{}", s.sid),
        }
    }
}

impl SearchScope {
    pub fn filter_map(&self, id: &MapPath, collected_targets: &[SearchScope]) -> bool {
        match self {
            SearchScope::AllMods => true,
            SearchScope::AllOpenMods => collected_targets.contains(&SearchScope::Mod(id.module)),
            SearchScope::AllOpenMaps => collected_targets.contains(&SearchScope::Map(id.clone())),
            SearchScope::Mod(m) => id.module == *m,
            SearchScope::Map(m) => id == m,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Data)]
pub enum ConfigSearchType {
    Entities,
    Triggers,
    Stylegrounds,
}

impl std::fmt::Display for ConfigSearchType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Lens, Clone, PartialEq, Eq, Data)]
pub enum ConfigSearchFilter {
    All,
    NoConfig,
    NoDrawConfig,
    NoAttrConfig,
    Matches(String),
}

impl std::fmt::Display for ConfigSearchFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSearchFilter::All => write!(f, "All"),
            ConfigSearchFilter::NoConfig => write!(f, "Unconfigured"),
            ConfigSearchFilter::NoDrawConfig => write!(f, "No draw config"),
            ConfigSearchFilter::NoAttrConfig => write!(f, "Missing attribute config"),
            ConfigSearchFilter::Matches(_) => write!(f, "Search by name..."),
        }
    }
}

#[derive(Clone, Debug, Lens)]
pub struct MapTab {
    pub id: MapID,
    pub nonce: u32,
    pub current_room: usize,
    pub current_selected: Option<AppSelection>,
    pub styleground_selected: Option<StylegroundSelection>,
    pub transform: MapToScreen,
    pub preview_pos: MapPointStrict,
}

impl PartialEq for MapTab {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce
    }
}

impl Eq for MapTab {}

impl Data for AppTab {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl ToString for AppTab {
    fn to_string(&self) -> String {
        match self {
            AppTab::CelesteOverview => "All Mods".to_owned(),
            AppTab::ProjectOverview(s) => format!("{} - Overview", s),
            AppTab::Map(_) => "TODO".to_string(),
            AppTab::ConfigEditor(_) => "Config Editor".to_owned(),
            AppTab::Logs => "Logs".to_owned(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, enum_iterator::IntoEnumIterator)]
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

impl Data for Layer {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
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

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    ObjectTile(TilePoint),
    EntityBody(i32, bool),
    EntityNode(i32, usize, bool),
    Decal(u32, bool),
}

#[derive(Serialize, Deserialize)]
pub enum AppSelectable {
    InRoom(Vec<AppInRoomSelectable>),
    Rooms(Vec<CelesteMapLevel>),
}

#[derive(Serialize, Deserialize)]
pub enum AppInRoomSelectable {
    FgTiles(TilePoint, TileGrid<char>),
    BgTiles(TilePoint, TileGrid<char>),
    ObjectTiles(TilePoint, TileGrid<i32>),
    Entity(CelesteMapEntity, bool),
    Decal(CelesteMapDecal, bool),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Data)]
pub struct StylegroundSelection {
    pub fg: bool,
    pub idx: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Progress {
    pub progress: i32,
    pub status: String,
}

impl Data for Progress {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug)]
pub enum AppEvent {
    Progress {
        progress: Progress,
    },
    SetClipboard {
        contents: String,
    },
    SetConfigPath {
        path: PathBuf,
    },
    SetLastPath {
        path: PathBuf,
    },
    SetModules {
        modules: Mutex<InternedMap<CelesteModule>>,
    },
    OpenModuleOverviewTab {
        module: Interned,
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
    DeleteMod {
        project: Interned,
    },
    SetModName {
        project: Interned,
        name: String,
    },
    SetModVersion {
        project: Interned,
        version: EverestModuleVersion,
    },
    SetModPath {
        project: Interned,
        path: PathBuf,
    },
    NewMap {
        project: Interned,
    },
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
        config: AnyConfig,
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
    SelectPaletteObjectTile {
        tile: u32,
    },
    SelectPaletteEntity {
        entity: EntitySelectable,
    },
    SelectPaletteTrigger {
        trigger: TriggerSelectable,
    },
    SelectPaletteDecal {
        decal: DecalSelectable,
    },
    SelectObject {
        tab: usize,
        selection: Option<AppSelection>,
    },
    MapEvent {
        map: MapID,
        event: RefCell<Option<MapEvent>>,
        merge_phase: EventPhase,
    },
    Undo {
        map: MapID,
    },
    Redo {
        map: MapID,
    },
}

// HERE LIVES THE UNDO/REDOABLES
// guidelines:
// - should all be ABSOLUTE, or can be made absolute through mutation before apply
//   (so undo/redo phased merging works)
// - should only require a single reference to do their jobs, e.g. to the map or to the room
// - should all have a precise inverse, so history tracking is easy
// - events with the same phase should completely supersede each other!!

#[derive(Debug)]
pub enum MapEvent {
    AddStyleground {
        loc: StylegroundSelection,
        style: Box<CelesteMapStyleground>,
    },
    UpdateStyleground {
        loc: StylegroundSelection,
        style: Box<CelesteMapStyleground>,
    },
    RemoveStyleground {
        loc: StylegroundSelection,
    },
    MoveStyleground {
        loc: StylegroundSelection,
        target: StylegroundSelection,
    },
    AddRoom {
        idx: Option<usize>, // made absolute through mutation
        room: Box<CelesteMapLevel>,
        selectme: bool,
    },
    DeleteRoom {
        idx: usize,
    },
    RoomEvent {
        idx: usize,
        event: RoomEvent,
    },
    Batched {
        events: Vec<MapEvent>, // must be COMPLETELY orthogonal!!!
    },
}

#[derive(Debug)]
pub enum RoomEvent {
    MoveRoom {
        bounds: MapRectStrict,
    },
    UpdateRoomMisc {
        update: Box<CelesteMapLevelUpdate>,
    },
    TileUpdate {
        fg: bool,
        offset: TilePoint,
        data: TileGrid<char>,
    },
    ObjectTileUpdate {
        offset: TilePoint,
        data: TileGrid<i32>,
    },
    EntityAdd {
        entity: Box<CelesteMapEntity>,
        trigger: bool,
        selectme: bool,
        genid: bool,
    },
    EntityUpdate {
        entity: Box<CelesteMapEntity>,
        trigger: bool,
    },
    EntityRemove {
        id: i32,
        trigger: bool,
    },
    DecalAdd {
        fg: bool,
        decal: Box<CelesteMapDecal>,
        selectme: bool,
        genid: bool,
    },
    DecalUpdate {
        fg: bool,
        decal: Box<CelesteMapDecal>,
    },
    DecalRemove {
        fg: bool,
        id: u32,
    },
}

#[derive(Debug)]
#[non_exhaustive]
#[allow(clippy::enum_variant_names)]
pub enum AppInternalEvent {
    SelectMeEntity { id: i32, trigger: bool },
    SelectMeDecal { id: u32, fg: bool },
    SelectMeRoom { idx: usize },
}

impl Model for AppState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| {
            self.apply(cx, app_event);
        });
    }
}

impl AppState {
    pub fn new() -> AppState {
        let mut cfg: AppConfig = confy::load("arborio").unwrap_or_default();
        if !cfg
            .celeste_root
            .as_ref()
            .map(|root| root.is_dir())
            .unwrap_or_default()
        {
            cfg.celeste_root = None;
        }
        let cfg = AutoSaver::new(cfg, |cfg: &mut AppConfig| {
            confy::store("arborio", &cfg)
                .unwrap_or_else(|e| panic!("Failed to save celeste_mod file: {}", e));
        });

        AppState {
            config: cfg,
            current_tab: 0,
            poison_tab: usize::MAX,
            tabs: vec![AppTab::CelesteOverview],
            loaded_maps: HashMap::new(),
            loaded_maps_path_to_id: HashMap::new(),
            loaded_maps_id_to_path: HashMap::new(),
            loaded_maps_undo_buffer: HashMap::new(),
            loaded_maps_redo_buffer: HashMap::new(),
            loaded_maps_event_phase: HashMap::new(),
            current_toolspec: ToolSpec::Selection,
            current_tool: RefCell::new(None),
            current_fg_tile: TileSelectable::default(),
            current_bg_tile: TileSelectable::default(),
            current_entity: EntitySelectable::default(),
            current_trigger: TriggerSelectable::default(),
            current_decal: DecalSelectable::default(),
            draw_interval: 4.0,
            snap: true,
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::FgTiles,
            current_objtile: 0,
            objtiles_transform: MapToScreen::identity(),

            modules: InternedMap::new(),
            modules_version: 0,
            palettes: HashMap::new(),
            omni_palette: ModuleAggregate::new(
                &InternedMap::new(),
                &CelesteMap::new(),
                "Celeste".into(),
                false,
            ),
            progress: Progress {
                progress: 100,
                status: "".to_owned(),
            },
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

    pub fn current_palette_unwrap(&self) -> &ModuleAggregate {
        if let Some(AppTab::Map(result)) = self.tabs.get(self.current_tab) {
            self.palettes.get(&result.id).expect("stale reference")
        } else {
            panic!("misuse of current_palette_unwrap");
        }
    }

    pub fn current_map_ref(&self) -> Option<&CelesteMap> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps.get(&maptab.id)
        } else {
            None
        }
    }

    pub fn current_room_ref(&self) -> Option<&CelesteMapLevel> {
        if let Some(AppTab::Map(maptab)) = self.tabs.get(self.current_tab) {
            self.loaded_maps
                .get(&maptab.id)
                .and_then(|map| map.levels.get(maptab.current_room))
        } else {
            None
        }
    }

    pub fn apply(&mut self, cx: &mut EventContext, event: &AppEvent) {
        match event {
            // global events
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
                    if matches!(tab, AppTab::Map(maptab) if self.loaded_maps_id_to_path.get(&maptab.id).unwrap() == path)
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
                    let id = if let Some(id) = self.loaded_maps_path_to_id.get(path) {
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
                    if let Entry::Vacant(e) = self.palettes.entry(id) {
                        e.insert(ModuleAggregate::new(&self.modules, &map, path.module, true));
                    }

                    self.loaded_maps.insert(id, *map);
                    self.loaded_maps_id_to_path.insert(id, path.clone());
                    self.loaded_maps_path_to_id.insert(path.clone(), id);
                    self.loaded_maps_event_phase.insert(id, EventPhase::null());
                    self.loaded_maps_undo_buffer
                        .insert(id, VecDeque::with_capacity(UNDO_BUFFER_SIZE));
                    self.loaded_maps_redo_buffer
                        .insert(id, VecDeque::with_capacity(UNDO_BUFFER_SIZE));
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
                let mut r = modules.lock().unwrap();
                std::mem::swap(r.deref_mut(), &mut self.modules);
                self.modules_version += 1;
                self.omni_palette = trigger_palette_update(
                    &mut self.palettes,
                    &self.modules,
                    &self.loaded_maps,
                    &self.loaded_maps_id_to_path,
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
                    for (ident, module) in self.modules.iter() {
                        if **ident == name.as_str()
                            || *module.everest_metadata.name == name.as_str()
                            || path.exists()
                        {
                            number += 1;
                            continue 'outer;
                        }
                    }

                    let everest_data = EverestYaml {
                        name: name.clone().into(),
                        version: EverestModuleVersion(vec![0, 0, 0]),
                        dll: None,
                        dependencies: vec![],
                    };
                    std::fs::create_dir(&path).unwrap();
                    everest_data.save(&path);

                    let mut module_src = ConfigSource::Dir(FolderSource::new(&path).unwrap());
                    let mut module = CelesteModule::new(Some(path), everest_data);
                    module.load(&mut module_src);
                    let module_id = name.into();

                    self.modules.insert(module_id, module);
                    self.modules_version += 1;

                    cx.emit(AppEvent::OpenModuleOverviewTab { module: module_id });
                    break;
                }
            }
            AppEvent::DeleteMod { project } => {
                if let Some(path) = self.modules.remove(project).and_then(|m| m.filesystem_root) {
                    self.modules_version += 1;
                    std::fs::remove_dir_all(path).expect("Failed to delete mod from filesystem");
                    self.garbage_collect();
                }
            }
            AppEvent::SetModName { project, name } => {
                if let Some(module) = self.modules.get_mut(project) {
                    module.everest_metadata.name = name.clone().into();
                    module
                        .everest_metadata
                        .save(module.filesystem_root.as_ref().unwrap());
                    self.modules_version += 1;
                }
            }
            AppEvent::SetModVersion { project, version } => {
                if let Some(module) = self.modules.get_mut(project) {
                    module.everest_metadata.version = version.clone();
                    module
                        .everest_metadata
                        .save(module.filesystem_root.as_ref().unwrap());
                    self.modules_version += 1;
                }
            }
            AppEvent::SetModPath { project, path } => {
                if let Some(module) = self.modules.get_mut(project) {
                    if let Err(e) = std::fs::rename(module.filesystem_root.as_ref().unwrap(), path)
                    {
                        error!(
                            "Could not move {} to {}: {}",
                            &module.everest_metadata.name,
                            path.to_string_lossy(),
                            e
                        );
                    } else {
                        module.filesystem_root = Some(path.clone());
                        self.modules_version += 1;
                    }
                }
            }
            AppEvent::NewMap { project } => {
                if let Some(module) = self.modules.get_mut(project) {
                    assert!(matches!(module.module_kind(), CelesteModuleKind::Directory));
                    let mut new_id = 0;
                    let new_sid = 'outer2: loop {
                        new_id += 1;
                        let new_sid = format!(
                            "{}/{}/untitled-{}",
                            self.config.user_name,
                            module
                                .filesystem_root
                                .as_ref()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_string_lossy(),
                            new_id
                        );
                        for old_sid in module.maps.iter() {
                            if **old_sid == new_sid {
                                continue 'outer2;
                            }
                        }
                        break new_sid;
                    };
                    module.create_map(new_sid.clone());
                    cx.emit(AppEvent::OpenMap {
                        path: MapPath {
                            module: *project,
                            sid: new_sid,
                        },
                    });
                    self.modules_version += 1;
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
                    ctab.editing_config = Some(config.to_owned())
                }
            }
            AppEvent::SetConfigErrorMessage { tab, message } => {
                if let Some(AppTab::ConfigEditor(ctab)) = self.tabs.get_mut(*tab) {
                    ctab.error_message = message.to_owned();
                }
            }
            AppEvent::MapEvent {
                map,
                event,
                merge_phase,
            } => {
                if let Some(event) = event.borrow_mut().take() {
                    if let Some(map_obj) = self.loaded_maps.get_mut(map) {
                        match Self::apply_map_event(cx, map_obj, event) {
                            Ok(undo) => {
                                cx.needs_redraw();
                                map_obj.dirty = true;
                                let buf = self.loaded_maps_undo_buffer.get_mut(map).unwrap();
                                if buf.len() == UNDO_BUFFER_SIZE {
                                    buf.pop_front();
                                }
                                if buf.back().is_none()
                                    || self.loaded_maps_event_phase.get(map).unwrap() != merge_phase
                                {
                                    buf.push_back(undo);
                                }
                                *self.loaded_maps_event_phase.get_mut(map).unwrap() = *merge_phase;
                                self.loaded_maps_redo_buffer.get_mut(map).unwrap().clear();
                            }
                            Err(e) => {
                                log::error!("Internal error: map event: {}", e);
                            }
                        }
                    } else {
                        log::error!(
                            "Internal error: event referred to a map which does not exist {:?}",
                            map
                        );
                    }
                } else {
                    log::error!("Internal error: MapEvent being applied twice?");
                }
            }
            AppEvent::Undo { map } => {
                if let Some(map_obj) = self.loaded_maps.get_mut(map) {
                    let undo_buf = self.loaded_maps_undo_buffer.get_mut(map).unwrap();
                    let redo_buf = self.loaded_maps_redo_buffer.get_mut(map).unwrap();
                    if let Some(event) = undo_buf.pop_back() {
                        match Self::apply_map_event(cx, map_obj, event) {
                            Ok(opposite) => {
                                cx.needs_redraw();
                                map_obj.dirty = true;
                                redo_buf.push_back(opposite);
                                self.loaded_maps_event_phase
                                    .insert(*map, EventPhase::null());
                            }
                            Err(e) => {
                                log::error!("Internal error: Failed to undo: {}", e);
                            }
                        }
                    }
                }
            }
            AppEvent::Redo { map } => {
                if let Some(map_obj) = self.loaded_maps.get_mut(map) {
                    let undo_buf = self.loaded_maps_undo_buffer.get_mut(map).unwrap();
                    let redo_buf = self.loaded_maps_redo_buffer.get_mut(map).unwrap();
                    if let Some(event) = redo_buf.pop_back() {
                        match Self::apply_map_event(cx, map_obj, event) {
                            Ok(opposite) => {
                                cx.needs_redraw();
                                map_obj.dirty = true;
                                undo_buf.push_back(opposite);
                                self.loaded_maps_event_phase
                                    .insert(*map, EventPhase::null());
                            }
                            Err(e) => {
                                log::error!("Internal error: Failed to redo: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn apply_map_event(
        cx: &mut EventContext,
        map: &mut CelesteMap,
        event: MapEvent,
    ) -> Result<MapEvent, String> {
        match event {
            MapEvent::Batched { events } => Ok(MapEvent::Batched {
                events: events
                    .into_iter()
                    .map(|ev| Self::apply_map_event(cx, map, ev))
                    .collect::<Result<Vec<MapEvent>, String>>()?,
            }),
            MapEvent::AddStyleground { loc, style } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx <= vec.len() {
                    vec.insert(loc.idx, *style);
                    Ok(MapEvent::RemoveStyleground { loc })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::UpdateStyleground { loc, mut style } => {
                if let Some(style_ref) = map.styles_mut(loc.fg).get_mut(loc.idx) {
                    std::mem::swap(style_ref, &mut style);
                    Ok(MapEvent::UpdateStyleground { loc, style })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::RemoveStyleground { loc } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx < vec.len() {
                    let style = vec.remove(loc.idx);
                    Ok(MapEvent::AddStyleground {
                        loc,
                        style: Box::new(style),
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::MoveStyleground { loc, target } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx < vec.len() {
                    let style = vec.remove(loc.idx);
                    let vec = map.styles_mut(target.fg);
                    let real_target = if target.idx <= vec.len() { target } else { loc };
                    let vec = map.styles_mut(real_target.fg);
                    vec.insert(real_target.idx, style);
                    Ok(MapEvent::MoveStyleground {
                        loc: real_target,
                        target: loc,
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::AddRoom {
                idx,
                mut room,
                selectme,
            } => {
                let idx = idx.unwrap_or(map.levels.len());
                if room.name.is_empty() || map.levels.iter().any(|iroom| room.name == iroom.name) {
                    room.name = pick_new_name(map);
                }
                if idx <= map.levels.len() {
                    map.levels.insert(idx, *room);
                    if selectme {
                        cx.emit_custom(
                            Event::new(AppInternalEvent::SelectMeRoom { idx })
                                .propagate(Propagation::Subtree),
                        );
                    }

                    Ok(MapEvent::DeleteRoom { idx })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::DeleteRoom { idx } => {
                if idx <= map.levels.len() {
                    let room = map.levels.remove(idx);
                    Ok(MapEvent::AddRoom {
                        idx: Some(idx),
                        room: Box::new(room),
                        selectme: false,
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapEvent::RoomEvent { idx, event } => {
                if let Some(room) = map.levels.get_mut(idx) {
                    room.cache.borrow_mut().render_cache_valid = false;
                    Ok(MapEvent::RoomEvent {
                        idx,
                        event: Self::apply_room_event(cx, room, event)?,
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
        }
    }

    fn apply_room_event(
        cx: &mut EventContext,
        room: &mut CelesteMapLevel,
        event: RoomEvent,
    ) -> Result<RoomEvent, String> {
        match event {
            RoomEvent::UpdateRoomMisc { mut update } => {
                room.apply(&mut update);
                Ok(RoomEvent::UpdateRoomMisc { update })
            }
            RoomEvent::MoveRoom { mut bounds } => {
                if room.bounds.size != bounds.size {
                    room.solids.resize((bounds.size / 8).cast_unit(), '0');
                    room.bg.resize((bounds.size / 8).cast_unit(), '0');
                    room.object_tiles.resize((bounds.size / 8).cast_unit(), -1);
                    room.cache.borrow_mut().render_cache = None;
                }
                std::mem::swap(&mut room.bounds, &mut bounds);
                Ok(RoomEvent::MoveRoom { bounds })
            }
            RoomEvent::TileUpdate {
                fg,
                offset,
                mut data,
            } => {
                let target = if fg { &mut room.solids } else { &mut room.bg };
                apply_tiles(&offset, &mut data, target, '\0');
                Ok(RoomEvent::TileUpdate { fg, offset, data })
            }
            RoomEvent::ObjectTileUpdate { offset, mut data } => {
                apply_tiles(&offset, &mut data, &mut room.object_tiles, -2);
                Ok(RoomEvent::ObjectTileUpdate { offset, data })
            }
            RoomEvent::EntityAdd {
                mut entity,
                trigger,
                selectme,
                genid,
            } => {
                let id = if genid {
                    let id = room.next_id();
                    entity.id = id;
                    id
                } else if room.entity(entity.id, trigger).is_some() {
                    return Err("Entity/trigger already exists".to_owned());
                } else {
                    entity.id
                };
                if trigger {
                    room.triggers.push(*entity);
                } else {
                    room.entities.push(*entity)
                }
                if selectme {
                    cx.emit_custom(
                        Event::new(AppInternalEvent::SelectMeEntity { id, trigger })
                            .propagate(Propagation::Subtree),
                    );
                }
                Ok(RoomEvent::EntityRemove { id, trigger })
            }
            RoomEvent::EntityUpdate {
                mut entity,
                trigger,
            } => {
                if let Some(e) = room.entity_mut(entity.id, trigger) {
                    std::mem::swap(e, &mut entity);
                    Ok(RoomEvent::EntityUpdate { entity, trigger })
                } else {
                    Err("No such entity".to_owned())
                }
            }
            RoomEvent::EntityRemove { id, trigger } => {
                let entities = if trigger {
                    &mut room.triggers
                } else {
                    &mut room.entities
                };
                for (idx, entity) in entities.iter_mut().enumerate() {
                    if entity.id == id {
                        let entity = entities.remove(idx);
                        return Ok(RoomEvent::EntityAdd {
                            entity: Box::new(entity),
                            trigger,
                            selectme: false,
                            genid: true,
                        });
                    }
                }
                Err("No such entity".to_owned())
                // while i < entities.len() {
                //     if entities[i].id == *id {
                //         entities.remove(i);
                //     } else {
                //         i += 1;
                //     }
                // }
            }
            RoomEvent::DecalAdd {
                fg,
                mut decal,
                selectme,
                genid,
            } => {
                let id = if genid {
                    let id = next_uuid();
                    decal.id = id;
                    id
                } else if room.decal(decal.id, fg).is_some() {
                    return Err("Decal already exists".to_owned());
                } else {
                    decal.id
                };
                let decals = if fg {
                    &mut room.fg_decals
                } else {
                    &mut room.bg_decals
                };
                decals.push(*decal);
                if selectme {
                    cx.emit_custom(
                        Event::new(AppInternalEvent::SelectMeDecal { id, fg })
                            .propagate(Propagation::Subtree),
                    );
                }
                Ok(RoomEvent::DecalRemove { fg, id })
            }
            RoomEvent::DecalUpdate { fg, mut decal } => {
                if let Some(decal_dest) = room.decal_mut(decal.id, fg) {
                    std::mem::swap(decal_dest, &mut decal);
                    Ok(RoomEvent::DecalUpdate { fg, decal })
                } else {
                    Err("No such decal".to_owned())
                }
            }
            RoomEvent::DecalRemove { fg, id } => {
                // tfw drain_filter is unstable
                let decals = if fg {
                    &mut room.fg_decals
                } else {
                    &mut room.bg_decals
                };
                for (idx, decal) in decals.iter_mut().enumerate() {
                    if decal.id == id {
                        let decal = decals.remove(idx);
                        return Ok(RoomEvent::DecalAdd {
                            fg,
                            decal: Box::new(decal),
                            selectme: false,
                            genid: false,
                        });
                    }
                }
                Err("No such decal".to_owned())
                // while i < decals.len() {
                //     if decals[i].id == *id {
                //         decals.remove(i);
                //         any = true;
                //     } else {
                //         i += 1;
                //     }
                // }
            }
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
                    AppTab::Map(maptab) => self
                        .modules
                        .contains_key(&self.loaded_maps_id_to_path.get(&maptab.id).unwrap().module),
                    _ => true,
                }
            };

            let result = closure(idx, tab);
            if !result && self.current_tab > idx {
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
            #[allow(clippy::single_match)] // we will want more arms in the future
            match tab {
                AppTab::Map(maptab) => {
                    open_maps.insert(maptab.id);
                }
                _ => {}
            }
        }
        self.loaded_maps.retain(|id, _| open_maps.contains(id));
        self.loaded_maps_path_to_id
            .retain(|_, id| open_maps.contains(id));
        self.loaded_maps_id_to_path
            .retain(|id, _| open_maps.contains(id));
        self.loaded_maps_undo_buffer
            .retain(|id, _| open_maps.contains(id));
        self.loaded_maps_redo_buffer
            .retain(|id, _| open_maps.contains(id));
        self.loaded_maps_event_phase
            .retain(|id, _| open_maps.contains(id));
        self.palettes.retain(|id, _| open_maps.contains(id));
    }

    pub fn map_event(&self, event: MapEvent, merge_phase: EventPhase) -> AppEvent {
        AppEvent::MapEvent {
            map: self.map_tab_unwrap().id,
            merge_phase,
            event: RefCell::new(Some(event)),
        }
    }

    pub fn map_event_unique(&self, event: MapEvent) -> AppEvent {
        self.map_event(event, EventPhase::new())
    }

    pub fn room_event(&self, event: RoomEvent, merge_phase: EventPhase) -> AppEvent {
        self.room_event_explicit(event, merge_phase, self.map_tab_unwrap().current_room)
    }

    pub fn room_event_explicit(
        &self,
        event: RoomEvent,
        merge_phase: EventPhase,
        room: usize,
    ) -> AppEvent {
        self.map_event(MapEvent::RoomEvent { idx: room, event }, merge_phase)
    }

    // pub fn room_event_unique(&self, event: RoomEvent) -> AppEvent {
    //     self.room_event(event, EventPhase::new())
    // }

    // pub fn room_event_unique_explicit(&self, event: RoomEvent, room: usize) -> AppEvent {
    //     self.room_event_explicit(event, EventPhase::new(), room)
    // }

    pub fn batch_event(
        &self,
        events: impl IntoIterator<Item = MapEvent>,
        merge_phase: EventPhase,
    ) -> AppEvent {
        self.map_event(
            MapEvent::Batched {
                events: events.into_iter().collect(),
            },
            merge_phase,
        )
    }

    pub fn batch_event_unique(&self, events: impl IntoIterator<Item = MapEvent>) -> AppEvent {
        self.batch_event(events, EventPhase::new())
    }
}

pub fn apply_tiles<T: Copy + Eq>(
    offset: &TilePoint,
    data: &mut TileGrid<T>,
    target: &mut TileGrid<T>,
    ignore: T,
) -> bool {
    let mut dirty = false;
    let mut line_start = *offset;
    let mut cur = line_start;
    for (idx, tile) in data.tiles.iter_mut().enumerate() {
        if *tile != ignore {
            if let Some(tile_ref) = target.get_mut(cur) {
                if *tile_ref != *tile {
                    std::mem::swap(tile_ref, tile);
                    dirty = true;
                }
            }
        }
        if (idx + 1) % data.stride == 0 {
            line_start += TileVector::new(0, 1);
            cur = line_start;
        } else {
            cur += TileVector::new(1, 0);
        }
    }
    dirty
}

pub fn trigger_module_load(cx: &mut EventContext, path: PathBuf) {
    cx.spawn(move |cx| {
        let mut result = InternedMap::new();
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
    palettes: &mut HashMap<MapID, ModuleAggregate>,
    modules: &InternedMap<CelesteModule>,
    maps: &HashMap<MapID, CelesteMap>,
    map_paths: &HashMap<MapID, MapPath>,
) -> ModuleAggregate {
    for (id, pal) in palettes.iter_mut() {
        *pal = ModuleAggregate::new(
            modules,
            maps.get(id).unwrap(),
            map_paths.get(id).unwrap().module,
            true,
        );
    }
    // discard logs here
    ModuleAggregate::new_omni(modules, false)
}

pub fn pick_new_name(map: &CelesteMap) -> String {
    let all_names = map
        .levels
        .iter()
        .map(|room| &room.name)
        .collect::<HashSet<_>>();
    for ch in 'a'..='z' {
        if !all_names.contains(&format!("{}-00", ch)) {
            if ch == 'a' {
                return "a-00".to_string();
            } else {
                let ch = (ch as u8 - 1) as char;
                for num in 0..=99 {
                    let result = format!("{}-{:02}", ch, num);
                    if !all_names.contains(&result) {
                        return result;
                    }
                }
            }
        }
    }

    let mut num = 0;
    loop {
        let result = format!("lvl_{}", num);
        if !all_names.contains(&result) {
            break result;
        } else {
            num += 1;
        }
    }
}

fn load_map(module_root: &Path, sid: &str) -> Option<CelesteMap> {
    match CelesteModule::load_map_static(module_root, sid) {
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
