use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::ffi::OsStr;
use std::path::Path;
use walkdir::WalkDir;

use crate::assets::InternedMap;
use crate::celeste_mod::everest_yaml::{arborio_module_yaml, celeste_module_yaml, EverestYaml};
use crate::celeste_mod::module::CelesteModule;
use crate::celeste_mod::walker::{
    open_module, ConfigSource, ConfigSourceTrait, EmbeddedSource, FolderSource,
};
use crate::logging::*;

pub fn for_each_mod<F: FnMut(usize, usize, &str, ConfigSource)>(root: &Path, mut callback: F) {
    let to_load = WalkDir::new(root.join("Mods"))
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    let total = to_load.len();

    for (i, entry) in to_load.iter().enumerate() {
        if let Some(config) = open_module(entry.path()) {
            let name = entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or("<bad unicode>");
            callback(i, total, name, config);
        }
    }
}

pub fn load_all<F>(
    root: &Path,
    modules: &mut InternedMap<CelesteModule>,
    mut progress: F,
) -> LogResult<()>
where
    F: FnMut(f32, String),
{
    let mut log = LogBuf::new();
    let mut total = 0.0;
    for_each_mod(root, |i, n, name, config| {
        let i = i as f32;
        let n = n as f32 + 2.0;
        total = n;
        progress(i / n, format!("Loading {}", name));
        load_into(config, modules).offload(&mut log);
    });

    progress(total / (total + 2.0), "Loading Celeste".to_owned());
    modules.insert("Celeste".into(), {
        let path = root.join("Content");
        let source = FolderSource::new(&path).unwrap();
        let mut r = CelesteModule::new(Some(path), celeste_module_yaml());
        r.load(&mut source.into()).offload(&mut log);
        r
    });
    progress(
        (total + 1.0) / (total + 2.0),
        "Loading built-in config".to_owned(),
    );
    modules.insert("Arborio".into(), {
        let source = EmbeddedSource();
        let mut r = CelesteModule::new(None, arborio_module_yaml());
        r.load(&mut source.into()).offload(&mut log);
        r
    });

    log.done(())
}

pub fn load_into(
    mut source: ConfigSource,
    modules: &mut InternedMap<CelesteModule>,
) -> LogResult<()> {
    let mut log = LogBuf::new();
    if source.get_file(Path::new("everest.yaml")).is_some() {
        if let Some(yaml) = EverestYaml::from_config(&mut source).offload(LogLevel::Error, &mut log)
        {
            let mut module = CelesteModule::new(source.filesystem_root(), yaml);
            module.load(&mut source).offload(&mut log);
            match modules.entry(module.everest_metadata.name) {
                Entry::Occupied(mut e) => {
                    let path_existing = e.get().filesystem_root.as_ref();
                    let path_new = module.filesystem_root.as_ref();
                    let ext_existing = path_existing
                        .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                        .and_then(|ext| ext.to_str());
                    let ext_new = path_new
                        .map(|root| root.extension().unwrap_or_else(|| OsStr::new("")))
                        .and_then(|ext| ext.to_str());
                    if ext_existing == Some("zip") && ext_new == Some("") {
                        log.push(log!(
                            Info,
                            "Conflict between {} and {}, picked latter",
                            path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                            path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                        ));
                        e.insert(module);
                    } else if ext_existing == Some("") && ext_new == Some("zip") {
                        log.push(log!(
                            Info,
                            "Conflict between {} and {}, picked former",
                            path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                            path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                        ));
                    } else {
                        log.push(log!(
                            Warning,
                            "Conflict between {} and {}, picked latter",
                            path_existing.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                            path_new.map_or(Cow::from("<builtin>"), |r| r.to_string_lossy()),
                        ));
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(module);
                }
            }
        }
    }
    log.done(())
}
