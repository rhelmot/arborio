use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::atlas_img::Atlas;
use crate::autotiler;
use crate::autotiler::Tileset;
use crate::config::entity_config::{EntityConfig, TriggerConfig};
use crate::config::walker::ConfigSource;

pub struct CelesteModule {
    pub gameplay_graphics: Atlas,
    pub tilers: HashMap<String, autotiler::Autotiler>,
    pub entity_config: HashMap<String, EntityConfig>,
    pub trigger_config: HashMap<String, TriggerConfig>,
}

impl CelesteModule {
    pub fn new<T: ConfigSource>(source: &mut T) -> Self {
        let gameplay_graphics = Atlas::load(source, "Gameplay");

        let mut tilers = HashMap::new();
        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/ForegroundTiles.xml")) {
            tilers.insert("fg".to_owned(), Tileset::load(fp, &gameplay_graphics, "tilesets/").expect("Could not parse ForegroundTiles.xml"));
        }
        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/BackgroundTiles.xml")) {
            tilers.insert("bg".to_owned(), Tileset::load(fp, &gameplay_graphics, "tilesets/").expect("Could not parse BackgroundTiles.xml"));
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/tilers")) {
            if let Some(fp) = source.get_file(&path) {
                tilers.insert(
                    path.file_stem().unwrap().to_str().expect("Fatal error: non-utf8 config filepath").to_owned(),
                    Tileset::load(fp, &gameplay_graphics, "").expect("Could not parse custom tileset")
                );
            }
        }

        let entity_config = source.list_all_files(&PathBuf::from("Arborio/entities"))
            .map(|f| {
                let mut config: EntityConfig = serde_yaml::from_reader(source.get_file(&f).unwrap()).expect("Failed to parse entity config");
                if config.templates.len() == 0 {
                    config.templates.push(config.default_template());
                }
                (config.entity_name.clone(), config)
            })
            .collect();
        let trigger_config = source.list_all_files(&PathBuf::from("Arborio/triggers"))
            .map(|f| {
                let mut config: TriggerConfig = serde_yaml::from_reader(source.get_file(&f).unwrap()).expect("Failed to parse trigger config");
                if config.templates.len() == 0 {
                    config.templates.push(config.default_template());
                }
                (config.trigger_name.clone(), config)
            })
            .collect();

        Self {
            gameplay_graphics,
            tilers,
            entity_config,
            trigger_config,
        }
    }
}
