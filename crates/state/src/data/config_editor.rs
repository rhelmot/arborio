use crate::data::action::StylegroundSelection;
use arborio_maploader::map_struct::{Attribute, CelesteMapEntity, CelesteMapStyleground};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::config::{EntityConfig, StylegroundConfig, TriggerConfig};
use arborio_modloader::module::{MapPath, ModuleID};
use arborio_utils::interned::Interned;
use arborio_utils::vizia::prelude::*;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::data::app::AppState;

#[derive(Debug, PartialEq, Eq, Data, Clone)]
pub enum SearchScope {
    AllMods,
    AllOpenMods,
    AllOpenMaps,
    Mod(ModuleID),
    Map(MapPath),
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

    pub fn text<C: DataContext>(&self, cx: &mut C) -> String {
        match self {
            SearchScope::AllMods => "All Mods".to_owned(),
            SearchScope::AllOpenMods => "All Open Projects".to_owned(),
            SearchScope::AllOpenMaps => "All Open Maps".to_owned(),
            SearchScope::Mod(m) => {
                let app = cx.data::<AppState>().unwrap();
                match app.modules.get(m) {
                    Some(module) => module.everest_metadata.name.clone(),
                    None => "<dead project ref>".to_owned(),
                }
            }
            SearchScope::Map(m) => m.sid.clone(),
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

#[derive(Debug, Clone)]
pub enum ConfigSearchResult {
    Entity(EntityConfigSearchResult),
    Trigger(TriggerConfigSearchResult),
    Styleground(StylegroundConfigSearchResult),
}

#[derive(Debug, Clone, Lens)]
pub enum AnyConfig {
    Entity(EntityConfig),
    Trigger(TriggerConfig),
    Styleground(StylegroundConfig),
}

impl ConfigSearchResult {
    pub fn name(&self) -> &str {
        match self {
            ConfigSearchResult::Entity(e) => &e.name,
            ConfigSearchResult::Trigger(t) => &t.name,
            ConfigSearchResult::Styleground(s) => &s.name,
        }
    }

    pub fn examples_len(&self) -> usize {
        match self {
            ConfigSearchResult::Entity(e) => e.examples.lock().len(),
            ConfigSearchResult::Trigger(t) => t.examples.lock().len(),
            ConfigSearchResult::Styleground(s) => s.examples.lock().len(),
        }
    }

    pub fn example_attrs(&self) -> impl '_ + Iterator<Item = (String, Attribute)> {
        let e = if let ConfigSearchResult::Entity(e) = self {
            Some(e)
        } else {
            None
        };
        let t = if let ConfigSearchResult::Trigger(e) = self {
            Some(e)
        } else {
            None
        };
        let s = if let ConfigSearchResult::Styleground(e) = self {
            Some(e)
        } else {
            None
        };

        let e = e
            .into_iter()
            .flat_map(|e| e.examples.lock().clone().into_iter())
            .flat_map(|(e, _, _)| e.attributes.into_iter());
        let t = t
            .into_iter()
            .flat_map(|e| e.examples.lock().clone().into_iter())
            .flat_map(|(e, _, _)| e.attributes.into_iter());
        let s = s
            .into_iter()
            .flat_map(|e| e.examples.lock().clone().into_iter())
            .flat_map(|(e, _, _)| e.attributes.into_iter());

        e.chain(t).chain(s)
    }

    pub fn display_list(&self) -> String {
        format!("{} ({})", self.name(), self.examples_len())
    }

    pub fn get_config(&self, palette: &ModuleAggregate) -> AnyConfig {
        match self {
            ConfigSearchResult::Entity(e) => AnyConfig::Entity(
                palette
                    .entity_config
                    .get(&e.name)
                    .map(|a| a.as_ref().clone())
                    .unwrap_or_else(|| EntityConfig::new(&e.name)),
            ),
            ConfigSearchResult::Trigger(e) => AnyConfig::Trigger(
                palette
                    .trigger_config
                    .get(&e.name)
                    .map(|a| a.as_ref().clone())
                    .unwrap_or_else(|| TriggerConfig::new(&e.name)),
            ),
            ConfigSearchResult::Styleground(e) => AnyConfig::Styleground(
                palette
                    .styleground_config
                    .get(&e.name)
                    .map(|a| a.as_ref().clone())
                    .unwrap_or_else(|| StylegroundConfig::new(&e.name)),
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntityConfigSearchResult {
    pub name: Interned,
    pub examples: Arc<parking_lot::Mutex<Vec<(CelesteMapEntity, MapPath, usize)>>>,
}

#[derive(Clone, Debug)]
pub struct TriggerConfigSearchResult {
    pub name: Interned,
    pub examples: Arc<parking_lot::Mutex<Vec<(CelesteMapEntity, MapPath, usize)>>>,
}

#[derive(Clone, Debug)]
pub struct StylegroundConfigSearchResult {
    pub name: Interned,
    pub examples:
        Arc<parking_lot::Mutex<Vec<(CelesteMapStyleground, MapPath, StylegroundSelection)>>>,
}

impl EntityConfigSearchResult {
    pub fn new(name: Interned) -> Self {
        Self {
            name,
            examples: Arc::new(parking_lot::Mutex::new(vec![])),
        }
    }
}

impl PartialEq for EntityConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for EntityConfigSearchResult {}

impl Hash for EntityConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl TriggerConfigSearchResult {
    pub fn new(name: Interned) -> Self {
        Self {
            name,
            examples: Arc::new(parking_lot::Mutex::new(vec![])),
        }
    }
}

impl PartialEq for TriggerConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TriggerConfigSearchResult {}

impl Hash for TriggerConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl StylegroundConfigSearchResult {
    pub fn new(name: Interned) -> Self {
        Self {
            name,
            examples: Arc::new(parking_lot::Mutex::new(vec![])),
        }
    }
}

impl PartialEq for StylegroundConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for StylegroundConfigSearchResult {}

impl Hash for StylegroundConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
