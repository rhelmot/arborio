use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use dialog::DialogBox;

use crate::atlas_img;
use crate::autotiler;
use crate::atlas_img::SpriteReference;
use crate::entity_config::EntityConfig;

use crate::auto_saver::AutoSaver;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub celeste_root: PathBuf,
}

const EMBEDDED_CONFIG: include_dir::Dir = include_dir::include_dir!("conf");

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
        let fg_tiles = autotiler::Tileset::load(&path, &GAMEPLAY_ATLAS);

        fg_tiles.unwrap_or_else(|e| panic!("Failed to load ForegroundTiles.xml: {}", e))
    };
    pub static ref BG_TILES: HashMap<char, autotiler::Tileset> = {
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/BackgroundTiles.xml");
        let bg_tiles = autotiler::Tileset::load(path.as_path(), &GAMEPLAY_ATLAS);

        bg_tiles.unwrap_or_else(|e| panic!("Failed to load BackgroundTiles.xml: {}", e))
    };

    pub static ref ENTITY_CONFIG: Mutex<HashMap<String, EntityConfig>> = {
        Mutex::new(EMBEDDED_CONFIG.get_dir("entities").unwrap().files().iter()
            .map(|f| {
                let config: EntityConfig = serde_yaml::from_str(f.contents_utf8().unwrap()).unwrap();
                (config.entity_name.clone(), config)
            }).collect()
        )
    };
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
    assert_ne!(BG_TILES.len(), 0);
    assert!(ENTITY_CONFIG.lock().unwrap().get("default").is_some());
}
