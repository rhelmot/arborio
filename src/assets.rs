use fltk::dialog;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::atlas_img;
use crate::autotiler;

use crate::auto_saver::AutoSaver;
use std::borrow::{Borrow, BorrowMut};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub celeste_root: PathBuf,
}

lazy_static! {
    pub static ref CONFIG: Mutex<AutoSaver<Config>> = {
        let mut cfg: Config = confy::load("arborio").unwrap_or_default();
        let mut cfg = AutoSaver::new(cfg, |cfg: &mut Config| {
            confy::store("arborio", &cfg).unwrap_or_else(|e| panic!("Failed to save config file: {}", e));
        });
        if cfg.celeste_root.as_os_str().is_empty() {
            let celeste_path = PathBuf::from(
                dialog::file_chooser("Please choose Celeste.exe", "Celeste.exe", ".", false)
                .unwrap_or_else(|| panic!("Can't run arborio without a Celeste.exe!")));
            cfg.borrow_mut().celeste_root = celeste_path.parent().unwrap().to_path_buf();
        };
        Mutex::new(cfg)
    };
    pub static ref GAMEPLAY_ATLAS: atlas_img::Atlas = {
        let atlas = atlas_img::Atlas::load(CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/Atlases/Gameplay.meta").as_path());

        atlas.unwrap_or_else(|e| panic!("Failed to load gameplay atlas: {}", e))
    };
    pub static ref FG_TILES: HashMap<char, autotiler::Tileset> = {
        let path = CONFIG.lock().unwrap().celeste_root.join("Content/Graphics/ForegroundTiles.xml");
        let fg_tiles = autotiler::Tileset::load(&path, &GAMEPLAY_ATLAS)
            .unwrap_or_else(|e| panic!("Failed to load ForegroundTiles.xml: {}", e));

        fg_tiles
    };
}

pub fn load() {
    assert_ne!(FG_TILES.len(), 0);
}
