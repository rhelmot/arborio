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
