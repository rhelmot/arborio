use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Mutex;
use std::error::Error;
use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;
use fltk::dialog;

use crate::atlas_img;
use crate::autotiler;


#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub celeste_root: PathBuf,
}

lazy_static! {
    pub static ref CONFIG: Mutex<Config> = {
        let mut cfg: Result<Config, _> = confy::load("arborio");
        if cfg.is_err() {
            panic!(format!("Failed to load config file: {}", cfg.unwrap_err()));
        }
        let mut cfg = cfg.unwrap();
        if cfg.celeste_root.as_os_str().is_empty() {
            let celeste_path = match dialog::file_chooser("Please choose Celeste.exe", "Celeste.exe", ".", false) {
                Some(v) => v,
                None => panic!("Can't run arborio without a Celeste.exe!"),
            };
            cfg = Config {
                celeste_root: celeste_path.into().parent().unwrap(),
            };
            if let Err(e) = confy::store("arborio", &cfg) {
                panic!(format!("Failed to save config file: {}", e))
            }
        };
        Mutex::new(cfg)
    };

    pub static ref GAMEPLAY_ATLAS: atlas_img::Atlas = {
        let atlas = atlas_img::Atlas::load(CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/Atlases/Gameplay.meta").as_path());
        if atlas.is_err() {
            panic!(format!("Failed to load gameplay atlas: {}", atlas.unwrap_err()));
        }

        atlas.unwrap()
    };

    pub static ref FG_TILES: HashMap<char, autotiler::Tileset> = {
        let mut fg_tiles: HashMap<char, autotiler::Tileset> = HashMap::new();
        let result = autotiler::Tileset::load(CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/ForegroundTiles.xml").as_path(), &GAMEPLAY_ATLAS, &mut fg_tiles);
        if result.is_err() {
            panic!(format!("Failed to load ForegroundTiles.xml: {}", result.unwrap_err()));
        }

        fg_tiles
    };
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
}
