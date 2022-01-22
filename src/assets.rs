use lazy_static::lazy_static;
use include_dir::include_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use dialog::DialogBox;
use itertools::Itertools;

use crate::atlas_img;
use crate::autotiler;
use crate::atlas_img::SpriteReference;
use crate::entity_config::EntityConfig;
use crate::auto_saver::AutoSaver;
use crate::palette_widget::{EntitySelectable, TileSelectable, DecalSelectable};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub celeste_root: PathBuf,
}

const EMBEDDED_CONFIG: include_dir::Dir = include_dir!("conf");

lazy_static! {
    pub static ref CONFIG: Mutex<AutoSaver<Config>> = {
        let cfg: Config = confy::load("arborio").unwrap_or_default();
        let mut cfg = AutoSaver::new(cfg, |cfg: &mut Config| {
            confy::store("arborio", &cfg).unwrap_or_else(|e| panic!("Failed to save config file: {}", e));
        });
        if cfg.celeste_root.as_os_str().is_empty() {
            let celeste_path = PathBuf::from(
                dialog::FileSelection::new("Celeste Installation")
                    .title("Please choose Celeste.exe")
                    .path(".")
                    .mode(dialog::FileSelectionMode::Open)
                    .show()
                    .unwrap_or_else(|_| panic!("Can't run arborio without a Celeste.exe!"))
                    .unwrap_or_else(|| panic!("Can't run arborio without a Celeste.exe!")));
            cfg.borrow_mut().celeste_root = celeste_path.parent().unwrap().to_path_buf();
        };
        Mutex::new(cfg)
    };
    pub static ref GAMEPLAY_ATLAS: atlas_img::Atlas = {
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/Atlases/Gameplay.meta");
        let atlas = atlas_img::Atlas::load(path.as_path());

        atlas.unwrap_or_else(|e| panic!("Failed to load gameplay atlas: {}", e))
    };
    pub static ref FG_TILES: HashMap<char, autotiler::Tileset> = {
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/ForegroundTiles.xml");
        let fg_tiles = autotiler::Tileset::load(&path, &GAMEPLAY_ATLAS, "tilesets/");

        fg_tiles.unwrap_or_else(|e| panic!("Failed to load ForegroundTiles.xml: {}", e))
    };
    pub static ref BG_TILES: HashMap<char, autotiler::Tileset> = {
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/BackgroundTiles.xml");
        let bg_tiles = autotiler::Tileset::load(&path, &GAMEPLAY_ATLAS, "tilesets/");

        bg_tiles.unwrap_or_else(|e| panic!("Failed to load BackgroundTiles.xml: {}", e))
    };

    pub static ref ENTITY_CONFIG: HashMap<String, EntityConfig> = {
        EMBEDDED_CONFIG.get_dir("entities").unwrap().files().iter()
            .map(|f| {
                let mut config: EntityConfig = serde_yaml::from_str(f.contents_utf8().unwrap()).unwrap();
                if config.templates.len() == 0 {
                    config.templates.push(config.default_template())
                }
                (config.entity_name.clone(), config)
            }).collect()
    };
    pub static ref AUTOTILERS: HashMap<String, &'static HashMap<char, autotiler::Tileset>> = {
        let fgbg: Vec<(String, &'static HashMap<char, autotiler::Tileset>)> = vec![("fg".to_owned(), &FG_TILES), ("bg".to_owned(), &BG_TILES)];
        EMBEDDED_CONFIG.get_dir("tilers").unwrap().files().iter()
            .map(|f| (f.path().file_stem().unwrap().to_str().unwrap().to_owned(),
                Box::leak(Box::new(autotiler::Tileset::parse(f.contents_utf8().unwrap(), &GAMEPLAY_ATLAS, "")
                    .unwrap_or_else(|e| panic!("Failed to load {:?}: {}", f.path(), e)))) as &'static HashMap<char, autotiler::Tileset>
            ))
            .chain(fgbg.into_iter())
            .collect()
    };

    pub static ref FG_TILES_PALETTE: Vec<TileSelectable> = {
        extract_tiles_palette(&FG_TILES)
    };

    pub static ref BG_TILES_PALETTE: Vec<TileSelectable> = {
        extract_tiles_palette(&BG_TILES)
    };

    pub static ref ENTITIES_PALETTE: Vec<EntitySelectable> = {
        extract_entities_palette(&ENTITY_CONFIG)
    };

    pub static ref DECALS_PALETTE: Vec<DecalSelectable> = {
        GAMEPLAY_ATLAS.iter_paths().filter(|path| path.starts_with("decals/")).map(|path| DecalSelectable::new(path)).collect()
    };
}

fn extract_tiles_palette(map: &'static HashMap<char, autotiler::Tileset>) -> Vec<TileSelectable> {
    let mut vec: Vec<TileSelectable> = map.iter().map(|item| TileSelectable {
        id: *item.0,
        name: &item.1.name,
        texture: Some(item.1.texture),
    })
        .filter(|ts| ts.id != 'z')
        .sorted_by_key(|ts| ts.id)
        .collect();
    vec.insert(0, TileSelectable::default());
    vec
}

fn extract_entities_palette(config: &'static HashMap<String, EntityConfig>) -> Vec<EntitySelectable> {
    config.iter()
        .map(|c| c.1.templates.iter().map(move |t| EntitySelectable { config: c.1, template: t }))
        .flatten()
        .filter(|es| es.config.entity_name != "default")
        .sorted_by_key(|es| &es.template.name)
        .collect()
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
    assert_ne!(BG_TILES.len(), 0);
    assert!(ENTITY_CONFIG.get("default").is_some());
    assert_ne!(AUTOTILERS.len(), 0);
}
