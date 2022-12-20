use crate::data::action::StylegroundSelection;
use crate::data::config_editor::{
    AnyConfig, ConfigSearchFilter, ConfigSearchResult, ConfigSearchType, SearchScope,
};
use crate::data::selection::AppSelection;
use crate::data::MapID;
use arborio_maploader::map_struct::CelesteMapEntity;
use arborio_modloader::module::ModuleID;
use arborio_utils::units::{MapPointStrict, MapToScreen};
use arborio_utils::uuid::next_uuid;
use arborio_utils::vizia::prelude::*;

#[allow(clippy::large_enum_variant)] // this is very rarely passed around by value
#[derive(PartialEq, Eq, Debug, Lens, Clone)]
pub enum AppTab {
    CelesteOverview,
    ProjectOverview(ModuleID),
    Map(MapTab),
    ConfigEditor(ConfigEditorTab),
    Logs,
    MapMeta(MapID),
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
    pub preview_entity: CelesteMapEntity,
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
            preview_entity: CelesteMapEntity {
                id: 0,
                name: "".to_string(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                attributes: Default::default(),
                nodes: vec![],
            },
        }
    }
}

impl PartialEq for ConfigEditorTab {
    fn eq(&self, other: &Self) -> bool {
        self.nonce == other.nonce
    }
}

impl Eq for ConfigEditorTab {}

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
