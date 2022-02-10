use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::celeste_mod::everest_yaml::{arborio_module_yaml, celeste_module_yaml, EverestYaml};
use crate::celeste_mod::module::CelesteModule;
use crate::celeste_mod::walker::{
    ConfigSource, ConfigSourceTrait, EmbeddedSource, FolderSource, open_module
};

pub fn load_all<F>(root: &Path, modules: &mut HashMap<String, CelesteModule>, mut progress: F)
where
    F: FnMut(f32, String),
{
    let to_load = WalkDir::new(root.join("Mods"))
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    let total = (to_load.len() + 2) as f32;
    for (i, entry) in to_load.iter().enumerate() {
        let i = i as f32;
        progress(i / total, format!("Loading {}", entry.path().file_name().unwrap().to_str().unwrap_or("<bad unicode>")));
        if let Some(config) = open_module(entry.path()) {
            load_into(config, modules);
        }
    }

    progress((total - 2.0) / total, "Loading Celeste".to_owned());
    modules.insert(
        "Celeste".to_owned(), {
            let path = root.join("Content");
            let mut source = FolderSource::new(&path).unwrap();
            let mut r = CelesteModule::new(Some(path.clone()), celeste_module_yaml());
            r.load(&mut source.into());
            r
        }
    );
    progress((total - 1.0) / total, "Loading built-in config".to_owned());
    modules.insert(
        "Arborio".to_owned(), {
            let mut source = EmbeddedSource();
            let mut r = CelesteModule::new(None, arborio_module_yaml());
            r.load(&mut source.into());
            r
        }
    );
}

pub fn load_into(mut source: ConfigSource, modules: &mut HashMap<String, CelesteModule>) {
    if let Some(mut reader) = source.get_file(Path::new("everest.yaml")) {
        let mut data = String::new();
        reader.read_to_string(&mut data);
        let everest_yaml: Vec<EverestYaml> = match serde_yaml::from_str(data.trim_start_matches('\u{FEFF}')) {
            Ok(e) => e,
            Err(e) => {
                println!("Error parsing {}/everest.yaml: {:?}", source.filesystem_root().unwrap().to_str().unwrap_or("<invalid unicode>"), e);
                return;
            }
        };
        if everest_yaml.len() != 1 {
            println!("Error parsing {}/everest.yaml: {} entries", source.filesystem_root().unwrap().to_str().unwrap_or("<invalid unicode>"), everest_yaml.len());
        }
        let mut module = CelesteModule::new(source.filesystem_root(), everest_yaml.into_iter().next().unwrap());
        module.load(&mut source);
        modules.insert(module.everest_metadata.name.clone(), module);
    }
}
