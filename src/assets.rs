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
        let mut cfg: Config = confy::load("arborio").unwrap_or_default();
        if cfg.celeste_root.as_os_str().is_empty() {
            let celeste_path = PathBuf::from(
                dialog::file_chooser("Please choose Celeste.exe", "Celeste.exe", ".", false)
                .unwrap_or_else(|| panic!("Can't run arborio without a Celeste.exe!")));
            cfg = Config {
                celeste_root: celeste_path.parent().unwrap().to_path_buf(),
            };
            if let Err(e) = confy::store("arborio", &cfg) {
                panic!("Failed to save config file: {}", e);
            };
        };
        Mutex::new(cfg)
    };

    pub static ref GAMEPLAY_ATLAS: atlas_img::Atlas = {
        let atlas = atlas_img::Atlas::load(CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/Atlases/Gameplay.meta").as_path());
        if let Err(e) = atlas {
            panic!("Failed to load gameplay atlas: {}", e);
        }

        atlas.unwrap()
    };

    pub static ref FG_TILES: HashMap<char, autotiler::Tileset> = {
        let mut fg_tiles: HashMap<char, autotiler::Tileset> = HashMap::new();
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/ForegroundTiles.xml");
        let result = autotiler::Tileset::load(path.as_path(), &GAMEPLAY_ATLAS, &mut fg_tiles);
        if let Err(e) = result {
            panic!("Failed to load ForegroundTiles.xml: {}", e);
        }

        fg_tiles
    };
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
}
