use arborio_utils::vizia::prelude::*;
use notify::Watcher;
use priority_queue::PriorityQueue;
use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::env::var;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use walkdir::WalkDir;

use crate::everest_yaml::{
    arborio_module_yaml, celeste_module_yaml, EverestYaml, EverestYamlLoadError,
};
use crate::module::{CelesteModule, ModuleID, ARBORIO_MODULE_ID, CELESTE_MODULE_ID};
use arborio_walker::{open_module, ConfigSource, ConfigSourceTrait, EmbeddedSource, FolderSource};

pub fn for_each_mod<F: FnMut(usize, usize, &str, ConfigSource)>(root: &Path, mut callback: F) {
    let blacklist_str = var("ARBORIO_BLACKLIST");
    let blacklist: Vec<_> = blacklist_str
        .as_ref()
        .map(|s| s.split(':').collect())
        .unwrap_or_default();
    let to_load = WalkDir::new(root.join("Mods"))
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !blacklist.contains(&e.file_name().to_string_lossy().as_ref()))
        .collect::<Vec<_>>();
    let total = to_load.len();

    for (i, entry) in to_load.iter().enumerate() {
        if entry.file_name() == "Cache" {
            continue;
        }
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
    mut progress: F,
) -> (HashMap<ModuleID, CelesteModule>, HashMap<PathBuf, ModuleID>)
where
    F: FnMut(f32, String),
{
    let mut total = 0.0;
    let mut modules = HashMap::new();
    let mut id_lookup = HashMap::new();
    for_each_mod(root, |i, n, name, mut config| {
        let i = i as f32;
        let n = n as f32 + 2.0;
        total = n;
        progress(i / n, format!("Loading {name}"));
        if let Err((_, e)) = load_into(&mut config, &mut modules, &mut id_lookup) {
            id_lookup.remove(&config.filesystem_root().unwrap());
            log::error!("Failed parsing everest.yaml for {}: {}", config, e);
        }
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
    (modules, id_lookup)
}

pub fn load_into(
    source: &mut ConfigSource,
    modules: &mut HashMap<ModuleID, CelesteModule>,
    id_lookup: &mut HashMap<PathBuf, ModuleID>,
) -> Result<ModuleID, (ModuleID, EverestYamlLoadError)> {
    let path = source.filesystem_root().unwrap();
    let id = match id_lookup.entry(path) {
        Entry::Occupied(o) => *o.get(),
        Entry::Vacant(v) => *v.insert(ModuleID::new()),
    };
    let yaml = EverestYaml::from_config(source).map_err(|e| (id, e))?;
    let mut module = CelesteModule::new(source.filesystem_root(), yaml);
    module.load(source);
    modules.insert(id, module);
    Ok(id)
}

pub enum LoaderThreadMessage {
    SetRoot(PathBuf),
    Muffle(PathBuf),
    Change(PathBuf),
    Move(PathBuf, PathBuf),
}

enum LoaderThreadInternalMessage {
    SetRoot(PathBuf),
    Reload(HashSet<PathBuf>),
    Move(PathBuf, PathBuf),
}

const DEBOUNCE_TIME: Duration = Duration::from_millis(500);

pub fn setup_loader_thread<A0: Any + Send, A1: Any + Send, A2: Any + Send>(
    cx: &mut Context,
    mut make_progress: impl 'static + Send + FnMut(f32, String) -> A0,
    mut make_reset: impl 'static + Send + FnMut(HashMap<ModuleID, CelesteModule>) -> A1,
    mut make_update: impl 'static + Send + FnMut(HashMap<ModuleID, Option<CelesteModule>>) -> A2,
) -> Sender<LoaderThreadMessage> {
    let (tx, rx) = channel::<LoaderThreadMessage>();
    let (loader_tx, loader_rx) = channel::<LoaderThreadInternalMessage>();

    // notify listener thread
    let tx2 = tx.clone();
    let mut watcher = notify::recommended_watcher(
        move |event: Result<notify::Event, notify::Error>| match event {
            Ok(ev) => {
                if matches!(
                    ev.kind,
                    notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_)
                ) {
                    for path in ev.paths.into_iter() {
                        tx2.send(LoaderThreadMessage::Change(path)).unwrap();
                    }
                }
            }
            Err(er) => {
                log::error!("Hot-reload failure: {}", er);
            }
        },
    )
    .unwrap();

    thread::spawn(move || {
        // debouncer thread
        let mut queue = PriorityQueue::<PathBuf, Instant>::new();
        let mut muffled = HashSet::new();
        loop {
            let msg_or_err = rx.recv_timeout(queue.peek().map_or_else(
                || Duration::from_secs(60),
                |(_, deadline)| *deadline - Instant::now(),
            ));
            match msg_or_err {
                Ok(LoaderThreadMessage::Change(path)) => {
                    queue.push(path, Instant::now() + DEBOUNCE_TIME);
                }
                Ok(LoaderThreadMessage::Muffle(path)) => {
                    muffled.insert(path);
                }
                Ok(LoaderThreadMessage::SetRoot(new_path)) => {
                    loader_tx
                        .send(LoaderThreadInternalMessage::SetRoot(new_path))
                        .unwrap();
                    muffled.clear();
                    queue.clear();
                }
                Ok(LoaderThreadMessage::Move(old_path, new_path)) => {
                    loader_tx
                        .send(LoaderThreadInternalMessage::Move(
                            old_path.clone(),
                            new_path.clone(),
                        ))
                        .unwrap();
                    queue.push(old_path.clone(), Instant::now() + DEBOUNCE_TIME);
                    queue.push(new_path.clone(), Instant::now() + DEBOUNCE_TIME);
                    muffled.insert(old_path);
                    muffled.insert(new_path);
                }
                Err(RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    let mut to_send = HashSet::new();
                    while let Some((_, deadline)) = queue.peek() {
                        if deadline > &now {
                            break;
                        }
                        let (path, _) = queue.pop().unwrap();
                        if muffled.iter().any(|m| path.starts_with(m)) {
                            if muffled.remove(&path) {
                                continue;
                            }
                            continue;
                        }
                        to_send.insert(path);
                    }
                    if !to_send.is_empty() {
                        loader_tx
                            .send(LoaderThreadInternalMessage::Reload(to_send))
                            .unwrap();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });

    cx.spawn(move |cx| {
        // loader thread
        let mut link_points = HashMap::<PathBuf, PathBuf>::new(); // map watch points to mod folders
        let mut root = Option::<PathBuf>::default();
        let mut modules = HashMap::new();
        let mut id_lookup = HashMap::new();
        while let Ok(msg) = loader_rx.recv() {
            match msg {
                LoaderThreadInternalMessage::SetRoot(new_path) => {
                    if let Some(old_path) = root.take() {
                        watcher.unwatch(&old_path).unwrap();
                    }
                    for (old_path, _) in link_points.drain() {
                        watcher.unwatch(&old_path).unwrap();
                    }
                    (modules, id_lookup) = load_all(&new_path, |a, b| {
                        cx.emit(make_progress(a, b)).unwrap();
                    });
                    cx.emit(make_reset(modules.clone())).unwrap();
                    cx.emit(make_progress(1., "".to_owned())).unwrap();
                    watcher.watch(&new_path, notify::RecursiveMode::Recursive).unwrap();
                    root = Some(new_path);
                }
                LoaderThreadInternalMessage::Move(old_path, new_path) => {
                    let Some(id) = id_lookup.get(&old_path).copied() else {
                        log::warn!("Internal error: got move event for unloaded path {:?}", old_path);
                        continue;
                    };
                    id_lookup.insert(new_path, id);
                }
                LoaderThreadInternalMessage::Reload(paths) => {
                    let Some(root) = &root else {
                        log::warn!("Got hot-reload event before initialization");
                        continue;
                    };
                    let mods_path = root.join("Mods");
                    let mut worklist = HashSet::new();
                    let mut result = HashMap::new();
                    for path in paths {
                        if let Ok(suffix) = path.strip_prefix(&mods_path) {
                            if let Some(modname) = suffix.iter().next() {
                                worklist.insert(mods_path.join(modname));
                            }
                        }
                        for (watchpoint, mod_folder) in link_points.iter() {
                            if path.starts_with(watchpoint) {
                                worklist.insert(mod_folder.clone());
                            }
                        }
                    }
                    let n = worklist.len() as f32;
                    for (i, path) in worklist.into_iter().enumerate() {
                        if let Some(mut config) = open_module(&path) {
                            let name = path
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap_or("<bad unicode>");
                            cx.emit(make_progress((i + 1) as f32 / n, format!("Hot-reloading {}", name))).unwrap();
                            match load_into(&mut config, &mut modules, &mut id_lookup) {
                                Ok(id) => {
                                    result.insert(id, modules.get(&id).cloned());
                                },
                                Err((id, EverestYamlLoadError::Missing)) => {
                                    result.insert(id, None);
                                },
                                Err((_, e)) => log::error!("Failed parsing everest.yaml for {}: {}", config, e),
                            }
                        }
                    }
                    if !result.is_empty() {
                        cx.emit(make_update(result)).unwrap();
                    }
                    cx.emit(make_progress(1., "".to_owned())).unwrap();
                }
            }
        }
    });
    tx
}
