use dialog::DialogBox;
use include_dir::include_dir;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::atlas_img;
use crate::atlas_img::MultiAtlas;
use crate::auto_saver::AutoSaver;
use crate::autotiler;
use crate::autotiler::Autotiler;
use crate::config::entity_config::{EntityConfig, TriggerConfig};
use crate::config::module::CelesteModule;
use crate::config::walker::{ConfigSource, EmbeddedSource, FolderSource};
use crate::widgets::palette_widget::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub celeste_root: PathBuf,
}

lazy_static! {
    pub static ref CONFIG: Mutex<AutoSaver<Config>> = {
        let cfg: Config = confy::load("arborio").unwrap_or_default();
        let mut cfg = AutoSaver::new(cfg, |cfg: &mut Config| {
            confy::store("arborio", &cfg)
                .unwrap_or_else(|e| panic!("Failed to save config file: {}", e));
        });
        if cfg.celeste_root.as_os_str().is_empty() {
            let celeste_path = PathBuf::from(
                dialog::FileSelection::new("Celeste Installation")
                    .title("Please choose Celeste.exe")
                    .path(".")
                    .mode(dialog::FileSelectionMode::Open)
                    .show()
                    .unwrap_or_else(|_| panic!("Can't run arborio without a Celeste.exe!"))
                    .unwrap_or_else(|| panic!("Can't run arborio without a Celeste.exe!")),
            );
            cfg.borrow_mut().celeste_root = celeste_path.parent().unwrap().to_path_buf();
        };
        Mutex::new(cfg)
    };
    pub static ref MODULES: HashMap<String, CelesteModule> = {
        let mut result = HashMap::new();
        let mut celeste_module =
            FolderSource::new(CONFIG.lock().unwrap().celeste_root.join("Content")).unwrap();

        result.insert(
            "Celeste".to_owned(),
            CelesteModule::new(&mut celeste_module),
        );
        result.insert(
            "Arborio".to_owned(),
            CelesteModule::new(&mut EmbeddedSource()),
        );

        result
    };
    pub static ref GAMEPLAY_ATLAS: MultiAtlas<'static> = {
        let mut multi_atlas = MultiAtlas::new();
        for module in MODULES.values() {
            multi_atlas.add(&module.gameplay_atlas);
        }
        multi_atlas
    };
    pub static ref AUTOTILERS: HashMap<String, &'static Autotiler> = {
        MODULES
            .iter()
            .flat_map(|(name, module)| module.tilers.iter())
            .map(|(name, tiler)| (name.clone(), tiler))
            .collect()
    };
    pub static ref FG_TILES: &'static Autotiler = { AUTOTILERS.get("fg").unwrap() };
    pub static ref BG_TILES: &'static Autotiler = { AUTOTILERS.get("bg").unwrap() };
    pub static ref ENTITY_CONFIG: HashMap<&'static str, &'static EntityConfig> = {
        MODULES
            .iter()
            .flat_map(|(name, module)| module.entity_config.iter())
            .map(|(name, config)| (name.as_str(), config))
            .collect()
    };
    pub static ref TRIGGER_CONFIG: HashMap<&'static str, &'static TriggerConfig> = {
        MODULES
            .iter()
            .flat_map(|(name, module)| module.trigger_config.iter())
            .map(|(name, config)| (name.as_str(), config))
            .collect()
    };
    pub static ref FG_TILES_PALETTE: Vec<TileSelectable> = { extract_tiles_palette(&FG_TILES) };
    pub static ref BG_TILES_PALETTE: Vec<TileSelectable> = { extract_tiles_palette(&BG_TILES) };
    pub static ref ENTITIES_PALETTE: Vec<EntitySelectable> =
        { extract_entities_palette(&ENTITY_CONFIG) };
    pub static ref TRIGGERS_PALETTE: Vec<TriggerSelectable> =
        { extract_triggers_palette(&TRIGGER_CONFIG) };
    pub static ref DECALS_PALETTE: Vec<DecalSelectable> = {
        GAMEPLAY_ATLAS
            .iter_paths()
            .filter_map(|path| {
                if path.starts_with("decals/") {
                    Some(path.trim_start_matches("decals/"))
                } else {
                    None
                }
            })
            .map(DecalSelectable::new)
            .collect()
    };

    static ref INTERNSHIP: elsa::sync::FrozenMap<&'static str, &'static str> = {
        elsa::sync::FrozenMap::new()
    };
}

pub fn intern(s: &str) -> &'static str {
    // not sure why this API is missing so much
    if let Some(res) = INTERNSHIP.get(s) {
        res
    } else {
        let mine = Box::leak(Box::new(s.to_owned()));
        INTERNSHIP.insert(mine, mine)
    }
}

fn extract_tiles_palette(map: &'static HashMap<char, autotiler::Tileset>) -> Vec<TileSelectable> {
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
    config: &HashMap<&str, &EntityConfig>,
) -> Vec<EntitySelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates.iter().enumerate().map(move |(idx, _)| EntitySelectable {
                entity: intern(&c.entity_name),
                template: idx,
            })
        })
        .filter(|es| es.entity != "default")
        .sorted_by_key(|es| es.template)
        .collect()
}

fn extract_triggers_palette(
    config: &HashMap<&str, &TriggerConfig>,
) -> Vec<TriggerSelectable> {
    config
        .iter()
        .flat_map(|(_, c)| {
            c.templates.iter().enumerate().map(move |(idx, _)| TriggerSelectable {
                trigger: intern(&c.trigger_name),
                template: idx,
            })
        })
        .filter(|es| es.trigger != "default")
        .sorted_by_key(|es| es.template)
        .collect()
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
    assert_ne!(BG_TILES.len(), 0);
    assert!(ENTITY_CONFIG.get("default").is_some());
    assert_ne!(AUTOTILERS.len(), 0);
}

pub fn get_entity_config(entity_name: &str, trigger: bool) -> &'static EntityConfig {
    if trigger {
        ENTITY_CONFIG.get("trigger").unwrap()
    } else {
        ENTITY_CONFIG
            .get(entity_name)
            .unwrap_or_else(|| ENTITY_CONFIG.get("default").unwrap())
    }
}
