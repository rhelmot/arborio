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
use crate::celeste_mod::entity_config::{EntityConfig, TriggerConfig};
use crate::celeste_mod::module::CelesteModule;
use crate::celeste_mod::walker::{ConfigSource, EmbeddedSource, FolderSource};
use crate::widgets::palette_widget::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};

lazy_static! {
    static ref INTERNSHIP: elsa::sync::FrozenMap<&'static str, &'static str> =
        { elsa::sync::FrozenMap::new() };
    static ref UUID: Mutex<u32> = Mutex::new(0);
}

pub fn next_uuid() -> u32 {
    let mut locked = UUID.lock().unwrap();
    let result = *locked;
    *locked += 1;
    result
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
