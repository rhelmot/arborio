use std::collections::HashMap;
use std::iter;
use std::rc::Rc;
use std::sync::Arc;
use itertools::Itertools;
use vizia::*;

use crate::atlas_img::MultiAtlas;
use crate::autotiler::{Autotiler, Tileset};
use crate::celeste_mod::entity_config::{EntityConfig, TriggerConfig};
use crate::celeste_mod::module::CelesteModule;
use crate::widgets::palette_widget::{DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable};
use crate::assets;

#[derive(Lens)]
pub struct ModuleAggregate {
    pub gameplay_atlas: MultiAtlas,
    pub autotilers: HashMap<String, Arc<Autotiler>>,
    pub entity_config: HashMap<&'static str, Arc<EntityConfig>>,
    pub trigger_config: HashMap<&'static str, Arc<TriggerConfig>>,

    pub fg_tiles_palette: Vec<TileSelectable>,
    pub bg_tiles_palette: Vec<TileSelectable>,
    pub entities_palette: Vec<EntitySelectable>,
    pub triggers_palette: Vec<TriggerSelectable>,
    pub decals_palette: Vec<DecalSelectable>,
}

impl Model for ModuleAggregate {}

impl ModuleAggregate {
    pub fn new(
        modules: &HashMap<String, CelesteModule>,
        current_module: &str
    ) -> Self {
        // TODO: warning on missing dependencies
        let dep_mods = || {
            modules.get(current_module).unwrap()
                .everest_metadata.dependencies.iter()
                .filter_map(|dep| modules.get(&dep.name).map(|module| (dep.name.as_str(), module)))
                .chain(iter::once((current_module, modules.get("Arborio").unwrap())))
                .chain(iter::once((current_module, modules.get("Celeste").unwrap())))
                .chain(iter::once((current_module, modules.get(current_module).unwrap())))
        };
        let gameplay_atlas = {
            let mut multi_atlas = MultiAtlas::new();
            for (_, module) in dep_mods() {
                multi_atlas.add(&module.gameplay_atlas);
            }
            multi_atlas
        };
        let mut autotilers: HashMap<String, Arc<Autotiler>> = dep_mods()
            .flat_map(|(name, module)| module.tilers.iter())
            .map(|(name, tiler)| (name.clone(), tiler.clone()))
            .collect();
        autotilers.insert("fg".to_owned(), if let Some(fg) = modules.get(current_module).unwrap().tilers.get("fg") {
            fg.clone()
        } else {
            modules.get("Celeste").unwrap().tilers.get("fg").unwrap().clone()
        });
        autotilers.insert("bg".to_owned(), if let Some(fg) = modules.get(current_module).unwrap().tilers.get("bg") {
            fg.clone()
        } else {
            modules.get("Celeste").unwrap().tilers.get("bg").unwrap().clone()
        });

        let entity_config: HashMap<&'static str, Arc<EntityConfig>> = dep_mods()
            .flat_map(|(_, module)| module.entity_config.iter())
            .map(|(name, config)| (assets::intern(name), config.clone()))
            .collect();
        let trigger_config: HashMap<&'static str, Arc<TriggerConfig>> = dep_mods()
            .flat_map(|(_, module)| module.trigger_config.iter())
            .map(|(name, config)| (assets::intern(name), config.clone()))
            .collect();

        let fg_tiles_palette = extract_tiles_palette(autotilers.get("fg").unwrap());
        let bg_tiles_palette = extract_tiles_palette(autotilers.get("bg").unwrap());
        let entities_palette = extract_entities_palette(&entity_config);
        let triggers_palette = extract_triggers_palette(&trigger_config);
        let decals_palette = gameplay_atlas
            .iter_paths()
            .filter_map(|path| {
                if path.starts_with("decals/") {
                    Some(path.trim_start_matches("decals/"))
                } else {
                    None
                }
            })
            .sorted()
            .map(DecalSelectable::new)
            .collect();

        let result = Self {
            gameplay_atlas,
            autotilers,
            entity_config,
            trigger_config,

            fg_tiles_palette,
            bg_tiles_palette,
            entities_palette,
            triggers_palette,
            decals_palette,
        };
        result.sanity_check();
        result
    }

    pub fn sanity_check(&self) {
        assert!(self.autotilers.get("fg").is_some());
        assert!(self.autotilers.get("bg").is_some());
        assert!(!self.autotilers.get("fg").unwrap().is_empty());
        assert!(!self.autotilers.get("bg").unwrap().is_empty());
        assert!(self.entity_config.get("default").is_some());
        assert!(self.entity_config.get("trigger").is_some());
        assert!(self.trigger_config.get("default").is_some());
        assert!(!self.decals_palette.is_empty());
        assert!(!self.entities_palette.is_empty());
        assert!(!self.triggers_palette.is_empty());
        assert!(!self.fg_tiles_palette.is_empty());
        assert!(!self.bg_tiles_palette.is_empty());
    }

    pub fn get_entity_config(&self, entity_name: &str, trigger: bool) -> &Arc<EntityConfig> {
        if trigger {
            self.entity_config.get("trigger").unwrap()
        } else {
            self.entity_config
                .get(entity_name)
                .unwrap_or_else(|| self.entity_config.get("default").unwrap())
        }
    }
}

fn extract_tiles_palette(map: &HashMap<char, Tileset>) -> Vec<TileSelectable> {
    let mut vec: Vec<TileSelectable> = map
        .iter()
        .map(|item| TileSelectable {
            id: *item.0,
            name: &item.1.name,
            texture: Some(&item.1.texture),
        })
        .filter(|ts| ts.id != 'z')
        .sorted_by_key(|ts| ts.id)
        .collect();
    vec.insert(0, TileSelectable::default());
    vec
}

fn extract_entities_palette(
    config: &HashMap<&str, Arc<EntityConfig>>,
) -> Vec<EntitySelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates.iter().enumerate().map(move |(idx, _)| EntitySelectable {
                entity: assets::intern(&c.entity_name),
                template: idx,
            })
        })
        .filter(|es| es.entity != "default")
        .sorted_by_key(|es| es.template)
        .collect()
}

fn extract_triggers_palette(
    config: &HashMap<&str, Arc<TriggerConfig>>,
) -> Vec<TriggerSelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates.iter().enumerate().map(move |(idx, _)| TriggerSelectable {
                trigger: assets::intern(&c.trigger_name),
                template: idx,
            })
        })
        .filter(|es| es.trigger != "default")
        .sorted_by_key(|es| es.template)
        .collect()
}
