use crate::assets::{intern_str, InternedMap};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::atlas_img::Atlas;
use crate::autotiler::Tileset;
use crate::celeste_mod::config::{EntityConfig, StylegroundConfig, TriggerConfig};
use crate::celeste_mod::everest_yaml::EverestYaml;
use crate::celeste_mod::walker::ConfigSourceTrait;
use crate::celeste_mod::walker::{open_module, ConfigSource};
use crate::map_struct::from_reader;
use crate::{autotiler, save_as, CelesteMap};

#[derive(Debug, Clone)] // Clone should just increase the refcount on each arc, right?
pub struct CelesteModule {
    pub filesystem_root: Option<PathBuf>,
    pub everest_metadata: EverestYaml,
    pub gameplay_atlas: Atlas,
    pub tilers: InternedMap<Arc<autotiler::Autotiler>>,
    pub entity_config: InternedMap<Arc<EntityConfig>>,
    pub trigger_config: InternedMap<Arc<TriggerConfig>>,
    pub styleground_config: InternedMap<Arc<StylegroundConfig>>,
    pub maps: Vec<String>,
}

impl CelesteModule {
    pub fn new(root: Option<PathBuf>, metadata: EverestYaml) -> Self {
        Self {
            filesystem_root: root,
            everest_metadata: metadata,
            gameplay_atlas: Atlas::new(),
            tilers: InternedMap::new(),
            entity_config: InternedMap::new(),
            trigger_config: InternedMap::new(),
            styleground_config: InternedMap::new(),
            maps: vec![],
        }
    }

    pub fn load(&mut self, source: &mut ConfigSource) {
        self.gameplay_atlas.load(source, "Gameplay");

        for path in source.list_all_files(&PathBuf::from("Arborio/tilers")) {
            if path.to_str().is_some() {
                if let Some(fp) = source.get_file(&path) {
                    match Tileset::new(fp, "") {
                        Ok(tiler) => {
                            self.tilers.insert(
                                intern_str(
                                    path.file_stem()
                                        .unwrap_or(path.as_os_str())
                                        .to_str()
                                        .unwrap(),
                                ),
                                Arc::new(tiler),
                            );
                        }
                        Err(e) => log::error!("Failed constructing tileset: {}", e),
                    }
                } else {
                    log::error!("Path disappeared from {}: {:?}", source, path);
                }
            } else {
                log::error!("Invalid unicode in {}: {:?}", source, path);
            }
        }

        for path in source.list_all_files(&PathBuf::from("Arborio/entities")) {
            if let Some(f) = source.get_file(&path) {
                match serde_yaml::from_reader::<_, EntityConfig>(f) {
                    Ok(mut config) => {
                        if config.templates.is_empty() {
                            config.templates.push(config.default_template());
                        }
                        self.entity_config
                            .insert(intern_str(&config.entity_name), Arc::new(config));
                    }
                    Err(e) => log::error!("Failed loading entity config {}: {}", path.display(), e),
                }
            } else {
                log::error!("Path disappeared from {}: {:?}", source, path);
            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/triggers")) {
            if let Some(f) = source.get_file(&path) {
                match serde_yaml::from_reader::<_, TriggerConfig>(f) {
                    Ok(mut config) => {
                        if config.templates.is_empty() {
                            config.templates.push(config.default_template());
                        }
                        self.trigger_config
                            .insert(intern_str(&config.trigger_name), Arc::new(config));
                    }
                    Err(e) => {
                        log::error!("Failed loading trigger config {}: {}", path.display(), e)
                    }
                }
            } else {
                log::error!("Path disappeared from {}: {:?}", source, path);
            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/stylegrounds")) {
            if let Some(f) = source.get_file(&path) {
                match serde_yaml::from_reader::<_, StylegroundConfig>(f) {
                    Ok(config) => {
                        self.styleground_config
                            .insert(intern_str(&config.styleground_name), Arc::new(config));
                    }
                    Err(e) => log::error!(
                        "Failed loading styleground config {}: {}",
                        path.display(),
                        e
                    ),
                }
            } else {
                log::error!("Path disappeared from {}: {:?}", source, path);
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
                    self.maps.push(sid.to_string());
                }
            }
        }
    }

    pub fn module_kind(&self) -> CelesteModuleKind {
        if *self.everest_metadata.name == "Celeste" {
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

    pub fn load_map_static(root: &Path, sid: &str) -> Result<CelesteMap, io::Error> {
        let mut config = match open_module(root) {
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Module has disappeared. Did you delete something?",
                ))
            }
            Some(c) => c,
        };
        let reader = match config.get_file(&PathBuf::from("Maps").join(sid.to_string() + ".bin")) {
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Map file has disappeared. Did you delete something?",
                ))
            }
            Some(r) => r,
        };

        from_reader(reader)
    }

    pub fn create_map(&mut self, sid: String) {
        let p = self
            .filesystem_root
            .as_ref()
            .unwrap()
            .join("Maps")
            .join(sid.clone() + ".bin");
        std::fs::create_dir_all(p.parent().unwrap()).expect("Failed to create directory for map");
        save_as(&CelesteMap::new(), &p).expect("Could not save blank map");
        self.maps.push(sid);
    }
}

pub enum CelesteModuleKind {
    Builtin,
    Zip,
    Directory,
}
