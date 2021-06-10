use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Mutex;
use std::error::Error;
use serde::{Serialize, Deserialize};

use crate::atlas_img;
use crate::autotiler;


#[derive(Serialize, Deserialize, Default)]
struct Config {
    celeste_root: PathBuf,
}

lazy_static! {
    pub static ref CONFIG: Mutex<Config> = {
        let mut cfg: Config = confy::load("arborio")?;
        if cfg.celeste_path.is_empty() {
            let celeste_path = match dialog::file_chooser("Please choose Celeste.exe", "Celeste.exe", ".", false) {
                Some(v) => v,
                None => panic!("Can't run arborio without a Celeste.exe!"),
            };
            cfg = Config {
                celeste_root: celeste_path.into().parent().unwrap(),
            };
            confy::store("arborio", &cfg)?;
        };
        Mutex::new(cfg)
    };

    pub static ref GAMPLAY_ATLAS: atlas_img::Atlas = {
        let atlas_img::Atlas::load(CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/Atlases/Gameplay.meta").as_path());
        if atlas_img.is_err() {
            panic!(format!("Failed to load gameplay atlas: {}", atlas_img.unwrap_err()));
        }

        atlas_img.unwrap()
    };

    pub static ref FG_TILES: HashMap<char, autotiler::Tileset> = {
        let mut fg_tiles: HashMap<char, autotiler::Tileset> = HashMap::new();
        let result = autotiler::Tileset::load(celeste_root.join("Content/Graphics/ForegroundTiles.xml").as_path(), &gameplay_atlas, &mut fg_tiles);
        if result.is_err() {
            panic!(format!("Failed to load ForegroundTiles.xml: {}", result.unwrap_err()))
        }

        fg_tiles
    }
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
}
