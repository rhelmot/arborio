use arborio_utils::vizia::prelude::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use arborio_gfxloader::atlas_img::MultiAtlas;
use arborio_gfxloader::autotiler::{Autotiler, Tileset};
use arborio_maploader::map_struct::CelesteMapMeta;
use arborio_utils::interned::{intern_str, Interned, InternedMap};
use arborio_walker::{open_module, ConfigSourceTrait};

use crate::config::{EntityConfig, StylegroundConfig, TriggerConfig};
use crate::module::{CelesteModule, ModuleID};
use crate::selectable::{DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable};

#[derive(Lens, Clone)]
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
    pub fn new(
        modules: &HashMap<ModuleID, CelesteModule>,
        modules_lookup: &HashMap<String, ModuleID>,
        map_meta: &Option<CelesteMapMeta>,
        current_module: ModuleID,
        emit_logs: bool,
    ) -> Self {
        if let Some(mymod) = modules.get(&current_module) {
            for dep in mymod.everest_metadata.dependencies.iter() {
                if dep.name == "Everest" {
                    continue;
                }
                if modules_lookup.get(&dep.name).is_none() {
                    log::warn!(
                        "{} missing dependency {}",
                        &modules.get(&current_module).unwrap().everest_metadata.name,
                        &dep.name
                    );
                }
            }
        }

        Self::new_core(
            map_meta,
            dep_mods(modules, modules_lookup, current_module),
            emit_logs,
        )
    }

    pub fn new_omni(modules: &HashMap<ModuleID, CelesteModule>, emit_logs: bool) -> Self {
        Self::new_core(
            &None,
            modules
                .values()
                .map(|y| (y.everest_metadata.name.as_str(), y)),
            emit_logs,
        )
    }

    fn new_core<'a>(
        map_meta: &Option<CelesteMapMeta>,
        deps: impl Clone + Iterator<Item = (&'a str, &'a CelesteModule)>,
        emit_logs: bool,
    ) -> Self {
        let gameplay_atlas = MultiAtlas::from(build_palette_map(
            "Gameplay Atlas",
            deps.clone(),
            |module| module.gameplay_atlas.sprites_map.iter(),
            emit_logs,
        ));
        let mut autotilers = build_palette_map(
            "Tiler Config",
            deps.clone(),
            |module| module.tilers.iter(),
            emit_logs,
        );
        let entity_config = build_palette_map(
            "Entity Config",
            deps.clone(),
            |module| module.entity_config.iter(),
            emit_logs,
        );
        let trigger_config = build_palette_map(
            "Trigger Config",
            deps.clone(),
            |module| module.trigger_config.iter(),
            emit_logs,
        );
        let styleground_config = build_palette_map(
            "Styleground Config",
            deps.clone(),
            |module| module.styleground_config.iter(),
            emit_logs,
        );

        if let Some(fg_xml) = map_meta
            .as_ref()
            .and_then(|meta| meta.bg_tiles.as_ref())
            .map(|s| s.as_str())
        {
            if let Some(tiler) = lookup_tiler(fg_xml, deps.clone()) {
                autotilers.insert("fg".into(), Arc::new(tiler));
            }
        }
        if let Some(bg_xml) = map_meta
            .as_ref()
            .and_then(|meta| meta.bg_tiles.as_ref())
            .map(|s| s.as_str())
        {
            if let Some(tiler) = lookup_tiler(bg_xml, deps.clone()) {
                autotilers.insert("bg".into(), Arc::new(tiler));
            }
        }
        if deps.clone().count() != 0 && !autotilers.contains_key("fg") {
            autotilers.insert(
                "fg".into(),
                Arc::new(lookup_tiler("Graphics/ForegroundTiles.xml", deps.clone()).unwrap()),
            );
        }
        if deps.clone().count() != 0 && !autotilers.contains_key("bg") {
            autotilers.insert(
                "bg".into(),
                Arc::new(lookup_tiler("Graphics/BackgroundTiles.xml", deps.clone()).unwrap()),
            );
        }

        let fg_tiles_palette = autotilers
            .get("fg")
            .map_or_else(Vec::new, |tiler| extract_tiles_palette(tiler));
        let bg_tiles_palette = autotilers
            .get("bg")
            .map_or_else(Vec::new, |tiler| extract_tiles_palette(tiler));
        let entities_palette = extract_entities_palette(&entity_config);
        let triggers_palette = extract_triggers_palette(&trigger_config);
        let decals_palette = gameplay_atlas
            .iter_paths()
            .filter_map(|path| path.strip_prefix("decals/").map(intern_str))
            .sorted()
            .map(DecalSelectable)
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
        if deps.count() != 0 {
            result.sanity_check();
        }
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

fn lookup_tiler<'a>(
    xml: &str,
    deps: impl Clone + Iterator<Item = (&'a str, &'a CelesteModule)>,
) -> Option<Autotiler> {
    for (depname, dep) in deps {
        if let Some(root) = &dep.filesystem_root {
            let mut config = open_module(root).unwrap();
            if let Some(fp) = config.get_file(Path::new(xml)) {
                match Tileset::new(fp, "tilesets/") {
                    Ok(t) => return Some(t),
                    Err(e) => {
                        log::error!("{}:{}: {}", depname, xml, e);
                    }
                }
            }
        }
    }
    None
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
            c.templates.iter().enumerate().map(move |(idx, t)| {
                (
                    &t.name,
                    EntitySelectable {
                        entity: intern_str(&c.entity_name),
                        template: idx,
                    },
                )
            })
        })
        .filter(|es| *es.1.entity != "default" && *es.1.entity != "trigger")
        .sorted_by_key(|es| es.0)
        .map(|es| es.1)
        .collect()
}

fn extract_triggers_palette(config: &InternedMap<Arc<TriggerConfig>>) -> Vec<TriggerSelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates.iter().enumerate().map(move |(idx, t)| {
                (
                    &t.name,
                    TriggerSelectable {
                        trigger: intern_str(&c.trigger_name),
                        template: idx,
                    },
                )
            })
        })
        .filter(|es| *es.1.trigger != "default")
        .sorted_by_key(|es| es.0)
        .map(|es| es.1)
        .collect()
}

fn dep_mods<'a>(
    modules: &'a HashMap<ModuleID, CelesteModule>,
    modules_lookup: &'a HashMap<String, ModuleID>,
    current_module: ModuleID,
) -> impl Clone + Iterator<Item = (&'a str, &'a CelesteModule)> {
    let x = modules_lookup;
    let y = modules;
    fn get<'a>(
        x: &'a HashMap<String, ModuleID>,
        y: &'a HashMap<ModuleID, CelesteModule>,
        s: &str,
    ) -> Option<&'a CelesteModule> {
        x.get(s).and_then(|id| y.get(id))
    }
    let a = get(x, y, "Arborio").into_iter().map(|m| ("Arborio", m));
    let b = get(x, y, "Celeste").into_iter().map(|m| ("Celeste", m));
    let c = modules.get(&current_module).into_iter().flat_map(move |m| {
        m.everest_metadata
            .dependencies
            .iter()
            .filter(|dep| dep.name != "Celeste" && dep.name != "Everest")
            .filter_map(move |dep| {
                get(x, y, &dep.name).map(|module| (module.everest_metadata.name.as_str(), module))
            })
    });
    let d = modules
        .get(&current_module)
        .into_iter()
        .filter(|m| m.everest_metadata.name != "Celeste")
        .map(|m| (m.everest_metadata.name.as_str(), m));

    a.chain(b).chain(c).chain(d)
}

fn build_palette_map<'a, T: 'a + Clone, I: 'a + Iterator<Item = (&'a Interned, &'a T)>>(
    // T will pretty much always be an arc
    what: &'static str,
    dep_mods: impl Iterator<Item = (&'a str, &'a CelesteModule)>,
    mapper: impl Fn(&'a CelesteModule) -> I,
    emit_logs: bool,
) -> InternedMap<T> {
    let mut result = HashMap::new();
    let mut result_source = HashMap::new();
    for (dep_name, dep_mod) in dep_mods {
        for (res_name, res) in mapper(dep_mod) {
            if result.insert(*res_name, res.clone()).is_some() && emit_logs {
                log::warn!(
                    "{} {}: {} overriding {}",
                    what,
                    res_name,
                    dep_name,
                    result_source[res_name]
                );
            }
            result_source.insert(*res_name, dep_name);
        }
    }
    result
}
