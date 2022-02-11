use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::atlas_img::Atlas;
use crate::autotiler;
use crate::autotiler::Tileset;
use crate::celeste_mod::entity_config::{EntityConfig, TriggerConfig};
use crate::celeste_mod::everest_yaml::EverestYaml;
use crate::celeste_mod::walker::ConfigSource;
use crate::celeste_mod::walker::ConfigSourceTrait;

#[derive(Debug)]
pub struct CelesteModule {
    pub filesystem_root: Option<PathBuf>,
    pub everest_metadata: EverestYaml,
    pub gameplay_atlas: Atlas,
    pub tilers: HashMap<String, Arc<autotiler::Autotiler>>,
    pub entity_config: HashMap<String, Arc<EntityConfig>>,
    pub trigger_config: HashMap<String, Arc<TriggerConfig>>,
    pub maps: Vec<String>,
}

impl CelesteModule {
    pub fn new(root: Option<PathBuf>, metadata: EverestYaml) -> Self {
        Self {
            filesystem_root: root,
            everest_metadata: metadata,
            gameplay_atlas: Atlas::new(),
            tilers: HashMap::new(),
            entity_config: HashMap::new(),
            trigger_config: HashMap::new(),
            maps: vec![],
        }
    }

    pub fn load(&mut self, source: &mut ConfigSource) {
        // TODO: return a list of errors
        self.gameplay_atlas.load(source, "Gameplay");

        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/ForegroundTiles.xml")) {
            self.tilers.insert(
                "fg".to_owned(),
                Arc::new(
                    Tileset::new(fp, "tilesets/").expect("Could not parse ForegroundTiles.xml"),
                ),
            );
        }
        if let Some(fp) = source.get_file(&PathBuf::from("Graphics/BackgroundTiles.xml")) {
            self.tilers.insert(
                "bg".to_owned(),
                Arc::new(
                    Tileset::new(fp, "tilesets/").expect("Could not parse BackgroundTiles.xml"),
                ),
            );
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/tilers")) {
            if let Some(fp) = source.get_file(&path) {
                self.tilers.insert(
                    path.file_stem()
                        .unwrap()
                        .to_str()
                        .expect("Fatal error: non-utf8 celeste_mod filepath")
                        .to_owned(),
                    Arc::new(Tileset::new(fp, "").expect("Could not parse custom tileset")),
                );
            }
        }

        for path in source.list_all_files(&PathBuf::from("Arborio/entities")) {
            if let Some(f) = source.get_file(&path) {
                let mut config: EntityConfig =
                    serde_yaml::from_reader(f).expect("Failed to parse entity celeste_mod");
                if config.templates.is_empty() {
                    config.templates.push(config.default_template());
                }
                self.entity_config
                    .insert(config.entity_name.clone(), Arc::new(config));
            } else {
            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/triggers")) {
            if let Some(f) = source.get_file(&path) {
                let mut config: TriggerConfig =
                    serde_yaml::from_reader(f).expect("Failed to parse trigger celeste_mod");
                if config.templates.is_empty() {
                    config.templates.push(config.default_template());
                }
                self.trigger_config
                    .insert(config.trigger_name.clone(), Arc::new(config));
            } else {
            }
        }

        for path in source.list_all_files(&PathBuf::from("Maps")) {
            if path.extension() == Some(OsStr::new("bin")) {
                if let Some(sid) = path
                    .strip_prefix("Maps")
                    .unwrap()
                    .with_extension("")
                    .to_str()
                {
                    self.maps.push(sid.to_owned());
                }
            }
        }
    }

    pub fn module_kind(&self) -> CelesteModuleKind {
        if self.everest_metadata.name == "Celeste" {
            return CelesteModuleKind::Builtin;
        }

        if let Some(path) = &self.filesystem_root {
            if path.extension() == Some(&OsString::from("zip")) {
                CelesteModuleKind::Zip
            } else {
                CelesteModuleKind::Directory
            }
        } else {
            CelesteModuleKind::Builtin
        }
    }
}

pub enum CelesteModuleKind {
    Builtin,
    Zip,
    Directory,
}
