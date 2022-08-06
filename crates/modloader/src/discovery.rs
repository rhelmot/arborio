use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

use crate::everest_yaml::{arborio_module_yaml, celeste_module_yaml, EverestYaml};
use crate::module::{CelesteModule, ModuleID, ARBORIO_MODULE_ID, CELESTE_MODULE_ID};
use arborio_walker::{open_module, ConfigSource, ConfigSourceTrait, EmbeddedSource, FolderSource};

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

pub fn load_all<F>(root: &Path, modules: &mut HashMap<ModuleID, CelesteModule>, mut progress: F)
where
    F: FnMut(f32, String),
{
    let mut total = 0.0;
    for_each_mod(root, |i, n, name, config| {
        let i = i as f32;
        let n = n as f32 + 2.0;
        total = n;
        progress(i / n, format!("Loading {}", name));
        load_into(config, modules);
    });

    progress(total / (total + 2.0), "Loading Celeste".to_owned());
    modules.insert(*CELESTE_MODULE_ID, {
        let path = root.join("Content");
        let source = FolderSource::new(&path).unwrap();
        let mut r = CelesteModule::new(Some(path), celeste_module_yaml());
        r.load(&mut source.into());
        r
    });
    progress(
        (total + 1.0) / (total + 2.0),
        "Loading built-in config".to_owned(),
    );
    modules.insert(*ARBORIO_MODULE_ID, {
        let source = EmbeddedSource();
        let mut r = CelesteModule::new(None, arborio_module_yaml());
        r.load(&mut source.into());
        r
    });
}

pub fn load_into(mut source: ConfigSource, modules: &mut HashMap<ModuleID, CelesteModule>) {
    let mut reverse_mapping = modules
        .iter()
        .filter_map(|(id, st)| st.filesystem_root.as_ref().map(|p| (p.clone(), *id)))
        .collect::<HashMap<_, _>>();
    if source.get_file(Path::new("everest.yaml")).is_some() {
        match EverestYaml::from_config(&mut source) {
            Ok(yaml) => {
                let mut module = CelesteModule::new(source.filesystem_root(), yaml);
                module.load(&mut source);
                let id =
                    match reverse_mapping.entry(module.filesystem_root.as_ref().unwrap().clone()) {
                        Entry::Occupied(o) => *o.get(),
                        Entry::Vacant(_) => ModuleID::new(),
                    };
                modules.insert(id, module);
            }
            Err(e) => log::warn!("Failed parsing everest.yaml for {}: {}", source, e),
        }
    }
}
