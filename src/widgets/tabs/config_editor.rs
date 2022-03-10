use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use vizia::*;

use crate::app_state::{
    AppConfig, AppEvent, AppState, AppTab, ConfigEditorTab, ConfigSearchFilter, ConfigSearchType,
    SearchScope,
};
use crate::celeste_mod::discovery;
use crate::lenses::{AutoSaverLens, CurrentTabImplLens, IsFailedLens};
use crate::map_struct::{CelesteMapEntity, CelesteMapStyleground};
use crate::widgets::tabs::project::load_map_inner;
use crate::{CelesteMap, MapID, ModuleAggregate};

#[derive(Debug, Clone)]
pub enum ConfigSearchResult {
    Entity(EntityConfigSearchResult),
    Trigger(TriggerConfigSearchResult),
    Styleground(StylegroundConfigSearchResult),
}

#[derive(Clone, Debug)]
pub struct EntityConfigSearchResult {
    pub name: String,
    pub examples: Arc<Mutex<Vec<CelesteMapEntity>>>,
}

#[derive(Clone, Debug)]
pub struct TriggerConfigSearchResult {
    pub name: String,
    pub examples: Arc<Mutex<Vec<CelesteMapEntity>>>,
}

#[derive(Clone, Debug)]
pub struct StylegroundConfigSearchResult {
    pub name: String,
    pub examples: Arc<Mutex<Vec<CelesteMapStyleground>>>,
}

impl EntityConfigSearchResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            examples: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl PartialEq for EntityConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for EntityConfigSearchResult {}

impl Hash for EntityConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl TriggerConfigSearchResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            examples: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl PartialEq for TriggerConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for TriggerConfigSearchResult {}

impl Hash for TriggerConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl StylegroundConfigSearchResult {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            examples: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl PartialEq for StylegroundConfigSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for StylegroundConfigSearchResult {}

impl Hash for StylegroundConfigSearchResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl ConfigSearchResult {
    fn name(&self) -> &str {
        match self {
            ConfigSearchResult::Entity(e) => &e.name,
            ConfigSearchResult::Trigger(t) => &t.name,
            ConfigSearchResult::Styleground(s) => &s.name,
        }
    }

    fn num_examples(&self) -> usize {
        match self {
            ConfigSearchResult::Entity(e) => e.examples.lock().len(),
            ConfigSearchResult::Trigger(t) => t.examples.lock().len(),
            ConfigSearchResult::Styleground(s) => s.examples.lock().len(),
        }
    }

    fn display_list(&self) -> String {
        format!("{} ({})", self.name(), self.num_examples())
    }
}

pub fn collect_search_targets(cx: &mut Context) -> Vec<SearchScope> {
    let app = cx.data::<AppState>().unwrap();
    let mut result = vec![
        SearchScope::AllOpenMods,
        SearchScope::AllOpenMaps,
        SearchScope::AllMods,
    ];

    for tab in &app.tabs {
        if let AppTab::ProjectOverview(p) = tab {
            result.push(SearchScope::Mod(*p));
        }
    }
    for tab in &app.tabs {
        if let AppTab::Map(m) = tab {
            result.push(SearchScope::Map(m.id.clone()));
        }
    }

    result
}

pub fn build_config_editor(cx: &mut Context) {
    build_search_settings(cx);
    build_search_results(cx);
}

pub fn build_search_settings(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    VStack::new(cx, move |cx| {
        HStack::new(cx, move |cx| {
            Label::new(cx, "Search Scope");
            Dropdown::new(
                cx,
                move |cx| {
                    Label::new(cx, "").bind(
                        ctab.then(ConfigEditorTab::search_scope),
                        |handle, scope| {
                            if let Some(thing) = scope.get_fallible(handle.cx) {
                                handle.text(format!("{}", thing.take()));
                            }
                        },
                    )
                },
                move |cx| {
                    for target in collect_search_targets(cx) {
                        Label::new(cx, &format!("{}", target))
                            .class("dropdown_element")
                            .on_press(move |cx| {
                                cx.emit(PopupEvent::Close);
                                let tab = cx.data::<AppState>().unwrap().current_tab;
                                cx.emit(AppEvent::SelectSearchScope {
                                    tab,
                                    scope: target.clone(),
                                })
                            });
                    }
                },
            );
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Search for");
            Dropdown::new(
                cx,
                move |cx| Label::new(cx, ctab.then(ConfigEditorTab::search_type)),
                move |cx| {
                    for target in [
                        ConfigSearchType::Entities,
                        ConfigSearchType::Triggers,
                        ConfigSearchType::Stylegrounds,
                    ] {
                        Label::new(cx, &format!("{}", target))
                            .class("dropdown_element")
                            .on_press(move |cx| {
                                cx.emit(PopupEvent::Close);
                                let tab = cx.data::<AppState>().unwrap().current_tab;
                                cx.emit(AppEvent::SelectSearchType { tab, ty: target })
                            });
                    }
                },
            );
        });
        HStack::new(cx, move |cx| {
            Label::new(cx, "Filter");
            Dropdown::new(
                cx,
                move |cx| Label::new(cx, ctab.then(ConfigEditorTab::search_filter)),
                move |cx| {
                    for target in [
                        ConfigSearchFilter::All,
                        ConfigSearchFilter::NoConfig,
                        ConfigSearchFilter::NoAttrConfig,
                        ConfigSearchFilter::NoDrawConfig,
                    ] {
                        Label::new(cx, &format!("{}", target))
                            .class("dropdown_element")
                            .on_press(move |cx| {
                                cx.emit(PopupEvent::Close);
                                let tab = cx.data::<AppState>().unwrap().current_tab;
                                cx.emit(AppEvent::SelectSearchFilter {
                                    tab,
                                    filter: target.clone(),
                                })
                            });
                    }
                    Label::new(
                        cx,
                        &format!("{}", ConfigSearchFilter::Matches("".to_owned())),
                    )
                    .class("dropdown_element")
                    .on_press(move |cx| {
                        if let Some(filter) =
                            ctab.then(ConfigEditorTab::search_filter).get_fallible(cx)
                        {
                            if !matches!(filter.take(), ConfigSearchFilter::Matches(_)) {
                                let tab = cx.data::<AppState>().unwrap().current_tab;
                                cx.emit(AppEvent::SelectSearchFilter {
                                    tab,
                                    filter: ConfigSearchFilter::Matches("".to_owned()),
                                })
                            }
                        }
                        cx.emit(PopupEvent::Close);
                    });
                },
            );
        });
        Binding::new(
            cx,
            IsFailedLens::new(
                ctab.then(ConfigEditorTab::search_filter)
                    .then(ConfigSearchFilter::matches),
            ),
            move |cx, is_failed| {
                if !*is_failed.get(cx) {
                    Textbox::new(
                        cx,
                        ctab.then(ConfigEditorTab::search_filter)
                            .then(ConfigSearchFilter::matches),
                    )
                    .on_edit(move |cx, text| {
                        let tab = cx.data::<AppState>().unwrap().current_tab;
                        cx.emit(AppEvent::SelectSearchFilter {
                            tab,
                            filter: ConfigSearchFilter::Matches(text),
                        });
                    });
                }
            },
        );
        Button::new(
            cx,
            move |cx| {
                let tab = cx.data::<AppState>().unwrap().current_tab;
                let filter = ctab.then(ConfigEditorTab::search_filter).get(cx).take();
                let ty = ctab.then(ConfigEditorTab::search_type).get(cx).take();
                let scope = ctab.then(ConfigEditorTab::search_scope).get(cx).take();
                let root = AppState::config
                    .then(AutoSaverLens::new())
                    .then(AppConfig::celeste_root)
                    .then(UnwrapLens::new())
                    .get(cx)
                    .take();
                let targets = collect_search_targets(cx);

                cx.spawn(move |cx| {
                    let results = match ty {
                        ConfigSearchType::Entities => {
                            walk_maps(&root, &scope, &filter, &targets, scan_entities)
                                .into_iter()
                                .map(ConfigSearchResult::Entity)
                                .collect()
                        }
                        ConfigSearchType::Triggers => {
                            walk_maps(&root, &scope, &filter, &targets, scan_triggers)
                                .into_iter()
                                .map(ConfigSearchResult::Trigger)
                                .collect()
                        }
                        ConfigSearchType::Stylegrounds => {
                            walk_maps(&root, &scope, &filter, &targets, scan_stylegrounds)
                                .into_iter()
                                .map(ConfigSearchResult::Styleground)
                                .collect()
                        }
                    };
                    cx.emit(AppEvent::PopulateConfigSearchResults { tab, results })
                        .unwrap();
                });
            },
            |cx| Label::new(cx, "Search"),
        );
    })
    .class("config_search_settings");
}

fn walk_maps<T>(
    root: &Path,
    scope: &SearchScope,
    filter: &ConfigSearchFilter,
    targets: &[SearchScope],
    f: impl Fn(&mut HashSet<T>, &ConfigSearchFilter, &CelesteMap, &ModuleAggregate),
) -> HashSet<T> {
    let mut results = HashSet::new();
    let mut modules = HashMap::new();
    discovery::load_all(root, &mut modules, |_, _| {});
    for (name, module) in modules.iter() {
        for map in &module.maps {
            if scope.filter_map(
                &MapID {
                    module: *name,
                    sid: *map,
                },
                targets,
            ) {
                if let Ok(map) =
                    load_map_inner(module.filesystem_root.as_ref().unwrap(), *name, *map)
                {
                    let palette = ModuleAggregate::new(&modules, &map);
                    f(&mut results, filter, &map, &palette);
                }
            }
        }
    }
    results
}

fn scan_entities(
    results: &mut HashSet<EntityConfigSearchResult>,
    filter: &ConfigSearchFilter,
    map: &CelesteMap,
    palette: &ModuleAggregate,
) {
    for room in &map.levels {
        for entity in &room.entities {
            let included = match filter {
                ConfigSearchFilter::All => true,
                ConfigSearchFilter::NoConfig => {
                    !palette.entity_config.contains_key(entity.name.as_str())
                }
                ConfigSearchFilter::NoDrawConfig => palette
                    .entity_config
                    .get(entity.name.as_str())
                    .map_or(true, |config| {
                        let default = palette.entity_config.get("default").unwrap();
                        config.selected_draw == default.selected_draw
                            && config.standard_draw == default.standard_draw
                    }),
                ConfigSearchFilter::NoAttrConfig => palette
                    .entity_config
                    .get(entity.name.as_str())
                    .map_or(true, |config| {
                        entity
                            .attributes
                            .iter()
                            .any(|(key, _)| config.attribute_info.contains_key(key.as_str()))
                    }),
                ConfigSearchFilter::Matches(s) => entity
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let ecsr = EntityConfigSearchResult::new(&entity.name);
                let ecsr = if let Some(entity_result) = results.get(&ecsr) {
                    entity_result
                } else {
                    results.insert(ecsr);
                    results
                        .get(&EntityConfigSearchResult::new(&entity.name))
                        .unwrap()
                };
                let mut vec = ecsr.examples.lock();
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = entity.clone();
                } else {
                    vec.push(entity.clone());
                }
            }
        }
    }
}

fn scan_triggers(
    results: &mut HashSet<TriggerConfigSearchResult>,
    filter: &ConfigSearchFilter,
    map: &CelesteMap,
    palette: &ModuleAggregate,
) {
    for room in &map.levels {
        for entity in &room.entities {
            let included = match filter {
                ConfigSearchFilter::All => true,
                ConfigSearchFilter::NoConfig => {
                    !palette.trigger_config.contains_key(entity.name.as_str())
                }
                ConfigSearchFilter::NoDrawConfig => false,
                ConfigSearchFilter::NoAttrConfig => palette
                    .trigger_config
                    .get(entity.name.as_str())
                    .map_or(true, |config| {
                        entity
                            .attributes
                            .iter()
                            .any(|(key, _)| config.attribute_info.contains_key(key.as_str()))
                    }),
                ConfigSearchFilter::Matches(s) => entity
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let tcsr = TriggerConfigSearchResult::new(&entity.name);
                let tcsr = if let Some(trigger_result) = results.get(&tcsr) {
                    trigger_result
                } else {
                    results.insert(tcsr);
                    results
                        .get(&TriggerConfigSearchResult::new(&entity.name))
                        .unwrap()
                };
                let mut vec = tcsr.examples.lock();
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = entity.clone();
                } else {
                    vec.push(entity.clone());
                }
            }
        }
    }
}

fn scan_stylegrounds(
    results: &mut HashSet<StylegroundConfigSearchResult>,
    filter: &ConfigSearchFilter,
    map: &CelesteMap,
    palette: &ModuleAggregate,
) {
    for g in [&map.foregrounds, &map.backgrounds] {
        for style in g {
            let included = match filter {
                ConfigSearchFilter::All => true,
                ConfigSearchFilter::NoConfig => {
                    !palette.styleground_config.contains_key(style.name.as_str())
                }
                ConfigSearchFilter::NoDrawConfig => palette
                    .styleground_config
                    .get(style.name.as_str())
                    .map_or(false, |c| c.preview.is_none()),
                ConfigSearchFilter::NoAttrConfig => palette
                    .styleground_config
                    .get(style.name.as_str())
                    .map_or(true, |config| {
                        style
                            .attributes
                            .iter()
                            .any(|(key, _)| config.attribute_info.contains_key(key.as_str()))
                    }),
                ConfigSearchFilter::Matches(s) => style
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let scsr = StylegroundConfigSearchResult::new(&style.name);
                let scsr = if let Some(style_result) = results.get(&scsr) {
                    style_result
                } else {
                    results.insert(scsr);
                    results
                        .get(&StylegroundConfigSearchResult::new(&style.name))
                        .unwrap()
                };
                let mut vec = scsr.examples.lock();
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = style.clone();
                } else {
                    vec.push(style.clone());
                }
            }
        }
    }
}

fn build_search_results(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
        List::new(
            cx,
            ctab.then(ConfigEditorTab::search_results),
            move |cx, idx, item| {
                let display = item.view(cx.data().unwrap(), |item| {
                    item.map(|item| item.display_list()).unwrap_or_default()
                });
                Label::new(cx, &display)
                    .class("palette_item")
                    .bind(
                        ctab.then(ConfigEditorTab::selected_result),
                        move |handle, selected| {
                            let selected = *selected.get(handle.cx);
                            handle.checked(selected == idx);
                        },
                    )
                    .on_press(move |cx| {
                        let tab = cx.data::<AppState>().unwrap().current_tab;
                        cx.emit(AppEvent::SelectConfigSearchResult { tab, idx });
                    });
            },
        );
    });
}
