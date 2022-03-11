use itertools::Itertools;
use std::collections::HashMap;
use std::iter;
use std::path::Path;
use std::sync::Arc;
use vizia::*;

use crate::assets::{intern_str, Interned, InternedMap};
use crate::atlas_img::MultiAtlas;
use crate::autotiler::{Autotiler, Tileset};
use crate::celeste_mod::entity_config::{EntityConfig, StylegroundConfig, TriggerConfig};
use crate::celeste_mod::module::CelesteModule;
use crate::celeste_mod::walker::{open_module, ConfigSourceTrait};
use crate::logging::*;
use crate::widgets::list_palette::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use crate::CelesteMap;

#[derive(Lens)]
pub struct ModuleAggregate {
    pub gameplay_atlas: MultiAtlas,
    pub autotilers: InternedMap<Arc<Autotiler>>,
    pub entity_config: InternedMap<Arc<EntityConfig>>,
    pub trigger_config: InternedMap<Arc<TriggerConfig>>,
    pub styleground_config: InternedMap<Arc<StylegroundConfig>>,

    pub fg_tiles_palette: Vec<TileSelectable>,
    pub bg_tiles_palette: Vec<TileSelectable>,
    pub entities_palette: Vec<EntitySelectable>,
    pub triggers_palette: Vec<TriggerSelectable>,
    pub decals_palette: Vec<DecalSelectable>,
}

impl Model for ModuleAggregate {}

impl ModuleAggregate {
    pub fn new(modules: &InternedMap<CelesteModule>, map: &CelesteMap) -> LogResult<Self> {
        let mut log = LogBuf::new();
        let current_module = map.id.module;
        for dep in modules
            .get(&current_module)
            .unwrap()
            .everest_metadata
            .dependencies
            .iter()
        {
            if *dep.name == "Everest" {
                continue;
            }
            if modules.get(&dep.name).is_none() {
                log.push(log!(
                    Warning,
                    "{} missing dependency {}",
                    current_module,
                    &dep.name
                ));
            }
        }
        let gameplay_atlas = MultiAtlas::from(
            build_palette_map(
                "Gameplay Atlas",
                dep_mods(modules, current_module),
                |module| module.gameplay_atlas.sprites_map.iter(),
            )
            .offload(&mut log),
        );
        let mut autotilers = build_palette_map(
            "Tiler Config",
            dep_mods(modules, current_module),
            |module| module.tilers.iter(),
        )
        .offload(&mut log);
        let entity_config = build_palette_map(
            "Entity Config",
            dep_mods(modules, current_module),
            |module| module.entity_config.iter(),
        )
        .offload(&mut log);
        let trigger_config = build_palette_map(
            "Trigger Config",
            dep_mods(modules, current_module),
            |module| module.trigger_config.iter(),
        )
        .offload(&mut log);
        let styleground_config = build_palette_map(
            "Styleground Config",
            dep_mods(modules, current_module),
            |module| module.styleground_config.iter(),
        )
        .offload(&mut log);

        let fg_xml = map
            .meta
            .as_ref()
            .and_then(|meta| meta.fg_tiles.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Graphics/ForegroundTiles.xml");
        let bg_xml = map
            .meta
            .as_ref()
            .and_then(|meta| meta.bg_tiles.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Graphics/BackgroundTiles.xml");
        for (_, dep) in dep_mods(modules, current_module) {
            if let Some(root) = &dep.filesystem_root {
                let mut config = open_module(root).unwrap();
                if let Some(fp) = config.get_file(Path::new(fg_xml)) {
                    autotilers.insert(
                        "fg".into(),
                        Arc::new(
                            Tileset::new(fp, "tilesets/")
                                .expect("Could not parse ForegroundTiles.xml"),
                        ),
                    );
                }
                if let Some(fp) = config.get_file(Path::new(bg_xml)) {
                    autotilers.insert(
                        "bg".into(),
                        Arc::new(
                            Tileset::new(fp, "tilesets/")
                                .expect("Could not parse BackgroundTiles.xml"),
                        ),
                    );
                }
            }
        }

        let fg_tiles_palette = extract_tiles_palette(autotilers.get("fg").unwrap());
        let bg_tiles_palette = extract_tiles_palette(autotilers.get("bg").unwrap());
        let entities_palette = extract_entities_palette(&entity_config);
        let triggers_palette = extract_triggers_palette(&trigger_config);
        let decals_palette = gameplay_atlas
            .iter_paths()
            .filter_map(|path| path.strip_prefix("decals/").map(intern_str))
            .sorted()
            .map(DecalSelectable::new)
            .collect();

        let result = Self {
            gameplay_atlas,
            autotilers,
            entity_config,
            trigger_config,
            styleground_config,

            fg_tiles_palette,
            bg_tiles_palette,
            entities_palette,
            triggers_palette,
            decals_palette,
        };
        result.sanity_check();
        log.done(result)
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

fn extract_entities_palette(config: &InternedMap<Arc<EntityConfig>>) -> Vec<EntitySelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates
                .iter()
                .enumerate()
                .map(move |(idx, _)| EntitySelectable {
                    entity: c.entity_name,
                    template: idx,
                })
        })
        .filter(|es| *es.entity != "default" && *es.entity != "trigger")
        .sorted_by_key(|es| es.template)
        .collect()
}

fn extract_triggers_palette(config: &InternedMap<Arc<TriggerConfig>>) -> Vec<TriggerSelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates
                .iter()
                .enumerate()
                .map(move |(idx, _)| TriggerSelectable {
                    trigger: c.trigger_name,
                    template: idx,
                })
        })
        .filter(|es| *es.trigger != "default")
        .sorted_by_key(|es| es.template)
        .collect()
}

fn dep_mods(
    modules: &InternedMap<CelesteModule>,
    current_module: Interned,
) -> impl Iterator<Item = (&str, &CelesteModule)> {
    iter::once(("Arborio", modules.get("Arborio").unwrap()))
        .chain(iter::once(("Celeste", modules.get("Celeste").unwrap())))
        .chain(
            modules
                .get(&current_module)
                .unwrap()
                .everest_metadata
                .dependencies
                .iter()
                .filter(|dep| *dep.name != "Celeste" && *dep.name != "Everest")
                .filter_map(|dep| modules.get(&dep.name).map(|module| (*dep.name, module))),
        )
        .chain(iter::once((
            *current_module,
            modules.get(&current_module).unwrap(),
        )))
}

fn build_palette_map<'a, T: 'a + Clone, I: 'a + Iterator<Item = (&'a Interned, &'a T)>>(
    // T will pretty much always be an arc
    what: &'static str,
    dep_mods: impl Iterator<Item = (&'a str, &'a CelesteModule)>,
    mapper: impl Fn(&'a CelesteModule) -> I,
) -> LogResult<InternedMap<T>> {
    let mut log = LogBuf::new();
    let mut result = HashMap::new();
    let mut result_source = HashMap::new();
    for (dep_name, dep_mod) in dep_mods {
        for (res_name, res) in mapper(dep_mod) {
            if result.insert(*res_name, res.clone()).is_some() {
                log.push(log!(
                    Warning,
                    "{} {}: {} overriding {}",
                    what,
                    res_name,
                    dep_name,
                    result_source[&*res_name]
                ));
            }
            result_source.insert(*res_name, dep_name);
        }
    }
    log.done(result)
}
