use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::atlas_img::Atlas;
use crate::autotiler;
use crate::autotiler::Tileset;
use crate::config::entity_config::{EntityConfig, TriggerConfig};
use crate::config::walker::ConfigSource;

pub struct CelesteModule {
    pub gameplay_atlas: Atlas,
    pub tilers: HashMap<String, Rc<autotiler::Autotiler>>,
    pub entity_config: HashMap<String, Rc<EntityConfig>>,
    pub trigger_config: HashMap<String, Rc<TriggerConfig>>,
}

impl CelesteModule {
    pub fn new() -> Self {
        Self {
            gameplay_atlas: Atlas::new(),
            tilers: HashMap::new(),
            entity_config: HashMap::new(),
            trigger_config: HashMap::new(),
        }
    }

    pub fn load<T: ConfigSource>(&mut self, source: &mut T) {
        // TODO: return a list of errors
        self.gameplay_atlas.load(source, "Gameplay");

        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/ForegroundTiles.xml")) {
            self.tilers.insert(
                "fg".to_owned(),
                Rc::new(Tileset::new(fp, "tilesets/")
                    .expect("Could not parse ForegroundTiles.xml")),
            );
        }
        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/BackgroundTiles.xml")) {
            self.tilers.insert(
                "bg".to_owned(),
                Rc::new(Tileset::new(fp, "tilesets/")
                    .expect("Could not parse BackgroundTiles.xml")),
            );
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/tilers")) {
            if let Some(fp) = source.get_file(&path) {
                self.tilers.insert(
                    path.file_stem()
                        .unwrap()
                        .to_str()
                        .expect("Fatal error: non-utf8 config filepath")
                        .to_owned(),
                    Rc::new(Tileset::new(fp, "")
                        .expect("Could not parse custom tileset")),
                );
            }
        }

        for path in source.list_all_files(&PathBuf::from("Arborio/entities")) {
            if let Some(f) = source.get_file(&path) {
                let mut config: EntityConfig =
                    serde_yaml::from_reader(f).expect("Failed to parse entity config");
                if config.templates.is_empty() {
                    config.templates.push(config.default_template());
                }
                self.entity_config.insert(config.entity_name.clone(), Rc::new(config));
            } else {

            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/triggers")) {
            if let Some(f) = source.get_file(&path) {
                let mut config: TriggerConfig =
                    serde_yaml::from_reader(f).expect("Failed to parse trigger config");
                if config.templates.is_empty() {
                    config.templates.push(config.default_template());
                }
                self.trigger_config.insert(config.trigger_name.clone(), Rc::new(config));
            } else {

            }
        }
    }
}
