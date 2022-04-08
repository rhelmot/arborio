use crate::assets::{intern_str, Interned, InternedMap};
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Arc;

use crate::atlas_img::Atlas;
use crate::autotiler;
use crate::autotiler::Tileset;
use crate::celeste_mod::config::{EntityConfig, StylegroundConfig, TriggerConfig};
use crate::celeste_mod::everest_yaml::EverestYaml;
use crate::celeste_mod::walker::ConfigSource;
use crate::celeste_mod::walker::ConfigSourceTrait;
use crate::logging::*;

#[derive(Debug, Clone)] // Clone should just increase the refcount on each arc, right?
pub struct CelesteModule {
    pub filesystem_root: Option<PathBuf>,
    pub everest_metadata: EverestYaml,
    pub gameplay_atlas: Atlas,
    pub tilers: InternedMap<Arc<autotiler::Autotiler>>,
    pub entity_config: InternedMap<Arc<EntityConfig>>,
    pub trigger_config: InternedMap<Arc<TriggerConfig>>,
    pub styleground_config: InternedMap<Arc<StylegroundConfig>>,
    pub maps: Vec<Interned>,
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

    pub fn load(&mut self, source: &mut ConfigSource) -> LogResult<()> {
        let mut log = LogBuf::new();
        self.gameplay_atlas
            .load(source, "Gameplay")
            .offload(&mut log);

        for path in source.list_all_files(&PathBuf::from("Arborio/tilers")) {
            if path.to_str().is_some() {
                if let Some(fp) = source.get_file(&path) {
                    if let Some(tiler) = Tileset::new(fp, "").offload(LogLevel::Error, &mut log) {
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
                } else {
                    log.push(log!(
                        Critical,
                        "Path disappeared from {}: {:?}",
                        source,
                        path
                    ))
                }
            } else {
                log.push(log!(Error, "Invalid unicode in {}: {:?}", source, path));
            }
        }

        for path in source.list_all_files(&PathBuf::from("Arborio/entities")) {
            if let Some(f) = source.get_file(&path) {
                if let Some(mut config) =
                    serde_yaml::from_reader::<_, EntityConfig>(f).offload(LogLevel::Error, &mut log)
                {
                    if config.templates.is_empty() {
                        config.templates.push(config.default_template());
                    }
                    self.entity_config
                        .insert(intern_str(&config.entity_name), Arc::new(config));
                }
            } else {
                log.push(log!(
                    Critical,
                    "Path disappeared from {}: {:?}",
                    source,
                    path
                ))
            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/triggers")) {
            if let Some(f) = source.get_file(&path) {
                if let Some(mut config) = serde_yaml::from_reader::<_, TriggerConfig>(f)
                    .offload(LogLevel::Error, &mut log)
                {
                    if config.templates.is_empty() {
                        config.templates.push(config.default_template());
                    }
                    self.trigger_config
                        .insert(intern_str(&config.trigger_name), Arc::new(config));
                }
            } else {
                log.push(log!(
                    Critical,
                    "Path disappeared from {}: {:?}",
                    source,
                    path
                ))
            }
        }
        for path in source.list_all_files(&PathBuf::from("Arborio/stylegrounds")) {
            if let Some(f) = source.get_file(&path) {
                if let Some(config) = serde_yaml::from_reader::<_, StylegroundConfig>(f)
                    .offload(LogLevel::Error, &mut log)
                {
                    self.styleground_config
                        .insert(intern_str(&config.styleground_name), Arc::new(config));
                }
            } else {
                log.push(log!(
                    Critical,
                    "Path disappeared from {}: {:?}",
                    source,
                    path
                ))
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
                    self.maps.push(intern_str(sid));
                }
            }
        }

        log.done(())
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
}

pub enum CelesteModuleKind {
    Builtin,
    Zip,
    Directory,
}
