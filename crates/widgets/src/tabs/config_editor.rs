use arborio_utils::vizia::fonts::icons_names::DOWN;
use arborio_utils::vizia::prelude::*;
use dialog::DialogBox;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use arborio_maploader::map_struct::{Attribute, CelesteMap, CelesteMapEntity};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::config::{AttributeInfo, AttributeType, AttributeValue, EntityConfig};
use arborio_modloader::mapstruct_plus_config::{make_entity_env, make_node_env};
use arborio_modloader::module::{CelesteModule, MapPath, ModuleID};
use arborio_state::data::action::StylegroundSelection;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::config_editor::{
    AnyConfig, ConfigSearchFilter, ConfigSearchResult, ConfigSearchType, EntityConfigPreviewModel,
    EntityConfigPreviewModelSetter, EntityConfigSearchResult, SearchScope,
    StylegroundConfigSearchResult, TriggerConfigSearchResult,
};
use arborio_state::data::tabs::{AppTab, ConfigEditorTab};
use arborio_state::data::AppConfigSetter;
use arborio_state::lenses::{CurrentTabImplLens, IsFailedLens};
use arborio_state::rendering::draw_entity;
use arborio_utils::interned::intern_str;
use arborio_utils::units::TileGrid;
use arborio_utils::vizia::state::UnwrapLens;
use arborio_utils::vizia::vg::{Paint, Path as PPath};
use arborio_widgets_common::basic_tweaker::basic_attrs_editor;

pub fn set_default_draw(this: &mut AnyConfig, app: &AppState) {
    if let AnyConfig::Entity(e) = this {
        e.standard_draw = app
            .omni_palette
            .entity_config
            .get("default")
            .unwrap()
            .standard_draw
            .clone();
        e.selected_draw = app
            .omni_palette
            .entity_config
            .get("default")
            .unwrap()
            .selected_draw
            .clone();
        e.hitboxes = app
            .omni_palette
            .entity_config
            .get("default")
            .unwrap()
            .hitboxes
            .clone();
    }
}

pub fn analyze_uses(
    this: &mut AnyConfig,
    result: &ConfigSearchResult,
    attribute_filter: &HashSet<&str>,
) {
    let info = attr_info(this);
    for (name, attr) in result.example_attrs() {
        let suggestion = most_interesting_type(&attr);
        match info.entry(name.clone()) {
            Entry::Occupied(mut o) => {
                let o = o.get_mut();
                o.ty = type_meet(&suggestion, &o.ty);
                if o.default.ty() != o.ty {
                    o.default = default_value(&suggestion);
                }
            }
            Entry::Vacant(v) => {
                v.insert(AttributeInfo {
                    ignore: attribute_filter.contains(name.as_str()),
                    display_name: Some(name),
                    default: default_value(&suggestion),
                    ty: suggestion,
                    options: vec![],
                });
            }
        }
    }
}

pub fn attr_info(this: &mut AnyConfig) -> &mut HashMap<String, AttributeInfo> {
    match this {
        AnyConfig::Entity(e) => &mut e.attribute_info,
        AnyConfig::Trigger(e) => &mut e.attribute_info,
        AnyConfig::Styleground(e) => &mut e.attribute_info,
    }
}

fn default_value(ty: &AttributeType) -> AttributeValue {
    match ty {
        AttributeType::String => AttributeValue::String("".to_owned()),
        AttributeType::Float => AttributeValue::Float(0.0),
        AttributeType::Int => AttributeValue::Int(0),
        AttributeType::Bool => AttributeValue::Bool(false),
    }
}

fn most_interesting_type(attr: &Attribute) -> AttributeType {
    match attr {
        Attribute::Bool(_) => AttributeType::Bool,
        Attribute::Int(_) => AttributeType::Int,
        Attribute::Float(f) => {
            if f.round() == *f {
                AttributeType::Int
            } else {
                AttributeType::Float
            }
        }
        Attribute::Text(s) => {
            if s.parse::<i32>().is_ok() {
                AttributeType::Int
            } else if s.parse::<f32>().is_ok() {
                AttributeType::Float
            } else if s.parse::<bool>().is_ok() {
                AttributeType::Bool
            } else {
                AttributeType::String
            }
        }
    }
}

fn type_meet(a: &AttributeType, b: &AttributeType) -> AttributeType {
    use AttributeType::*;
    match (a, b) {
        (String, String) => String,
        (String, Int) => String,
        (String, Float) => String,
        (String, Bool) => String,
        (Float, Float) => Float,
        (Float, Bool) => String,
        (Float, Int) => Float,
        (Float, String) => String,
        (Bool, Bool) => Bool,
        (Bool, Int) => String,
        (Bool, String) => String,
        (Bool, Float) => String,
        (Int, Int) => Int,
        (Int, Float) => Float,
        (Int, String) => String,
        (Int, Bool) => String,
    }
}

pub fn collect_search_targets<C: DataContext>(cx: &mut C) -> Vec<SearchScope> {
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
            result.push(SearchScope::Map(
                app.loaded_maps.get(&m.id).unwrap().cache.path.clone(),
            ));
        }
    }

    result
}

pub fn build_config_editor(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            build_search_settings(cx);
            build_search_results(cx);
        });
        VStack::new(cx, |cx| {
            build_item_editor(cx);
            build_item_preview(cx);
        });
    });
}

pub fn build_search_settings(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    VStack::new(cx, move |cx| {
        HStack::new(cx, move |cx| {
            Label::new(cx, "Search Scope");
            Dropdown::new(
                cx,
                move |cx| {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "").bind(
                            ctab.then(ConfigEditorTab::search_scope),
                            |handle, scope| {
                                if let Some(thing) = scope.get_fallible(handle.cx) {
                                    let text = thing.text(handle.cx);
                                    handle.text(&text);
                                }
                            },
                        );
                        Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                    })
                },
                move |cx| {
                    for target in collect_search_targets(cx) {
                        let text = target.text(cx);
                        Label::new(cx, &text)
                            .class("dropdown_element")
                            .class("btn_highlight")
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
                move |cx| {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, ctab.then(ConfigEditorTab::search_type));
                        Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                    })
                },
                move |cx| {
                    for target in [
                        ConfigSearchType::Entities,
                        ConfigSearchType::Triggers,
                        ConfigSearchType::Stylegrounds,
                    ] {
                        Label::new(cx, &format!("{}", target))
                            .class("dropdown_element")
                            .class("btn_highlight")
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
                move |cx| {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, ctab.then(ConfigEditorTab::search_filter));
                        Label::new(cx, DOWN).class("icon").class("dropdown_icon");
                    })
                },
                move |cx| {
                    for target in [
                        ConfigSearchFilter::All,
                        ConfigSearchFilter::NoConfig,
                        ConfigSearchFilter::NoAttrConfig,
                        ConfigSearchFilter::NoDrawConfig,
                    ] {
                        Label::new(cx, &format!("{}", target))
                            .class("dropdown_element")
                            .class("btn_highlight")
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
                    .class("btn_highlight")
                    .on_press(move |cx| {
                        if let Some(filter) =
                            ctab.then(ConfigEditorTab::search_filter).get_fallible(cx)
                        {
                            if !matches!(filter, ConfigSearchFilter::Matches(_)) {
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
        HStack::new(cx, move |cx| {
            Label::new(cx, "Exclude attributes");
            Textbox::new(cx, ctab.then(ConfigEditorTab::attribute_filter)).on_edit(|cx, data| {
                let tab = cx.data::<AppState>().unwrap().current_tab;
                cx.emit(AppEvent::SelectSearchFilterAttributes { tab, filter: data });
            });
        });
        Binding::new(
            cx,
            IsFailedLens::new(
                ctab.then(ConfigEditorTab::search_filter)
                    .then(ConfigSearchFilter::matches),
            ),
            move |cx, is_failed| {
                if !is_failed.get(cx) {
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
                let modules = cx.data::<AppState>().unwrap().modules.clone();
                let filter = ctab.then(ConfigEditorTab::search_filter).get(cx);
                let attrs = ctab.then(ConfigEditorTab::attribute_filter).get(cx);
                let ty = ctab.then(ConfigEditorTab::search_type).get(cx);
                let scope = ctab.then(ConfigEditorTab::search_scope).get(cx);
                let targets = collect_search_targets(cx);

                cx.spawn(move |cx| {
                    let results = match ty {
                        ConfigSearchType::Entities => {
                            walk_maps(&modules, &scope, &filter, &targets, &attrs, scan_entities)
                                .into_iter()
                                .map(ConfigSearchResult::Entity)
                                .collect()
                        }
                        ConfigSearchType::Triggers => {
                            walk_maps(&modules, &scope, &filter, &targets, &attrs, scan_triggers)
                                .into_iter()
                                .map(ConfigSearchResult::Trigger)
                                .collect()
                        }
                        ConfigSearchType::Stylegrounds => walk_maps(
                            &modules,
                            &scope,
                            &filter,
                            &targets,
                            &attrs,
                            scan_stylegrounds,
                        )
                        .into_iter()
                        .map(ConfigSearchResult::Styleground)
                        .collect(),
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
    modules: &HashMap<ModuleID, CelesteModule>,
    scope: &SearchScope,
    filter: &ConfigSearchFilter,
    targets: &[SearchScope],
    attrs: &str,
    f: impl Fn(
        &mut HashSet<T>,
        &ConfigSearchFilter,
        &HashSet<&str>,
        &CelesteMap,
        &MapPath,
        &ModuleAggregate,
    ),
) -> HashSet<T> {
    let modules_lookup = modules
        .iter()
        .map(|(id, m)| (m.everest_metadata.name.clone(), *id))
        .collect::<HashMap<String, ModuleID>>();
    let mut results = HashSet::new();
    let attrs = attrs.split(',').collect::<HashSet<_>>();
    for (name, module) in modules.iter() {
        for map in &module.maps {
            let map_path = MapPath {
                module: *name,
                sid: map.clone(),
            };
            if scope.filter_map(&map_path, targets) {
                if let Ok(map) =
                    CelesteModule::load_map_static(module.filesystem_root.as_ref().unwrap(), map)
                {
                    let palette =
                        ModuleAggregate::new(modules, &modules_lookup, &map.meta, *name, false);
                    f(&mut results, filter, &attrs, &map, &map_path, &palette);
                }
            }
        }
    }

    results
}

fn scan_entities(
    results: &mut HashSet<EntityConfigSearchResult>,
    filter: &ConfigSearchFilter,
    attrs: &HashSet<&str>,
    map: &CelesteMap,
    map_path: &MapPath,
    palette: &ModuleAggregate,
) {
    for (room_idx, room) in map.levels.iter().enumerate() {
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
                        entity.attributes.iter().any(|(key, _)| {
                            !attrs.contains(key.as_str())
                                && !config.attribute_info.contains_key(key.as_str())
                        })
                    }),
                ConfigSearchFilter::Matches(s) => entity
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let ecsr = EntityConfigSearchResult::new(intern_str(&entity.name));
                let ecsr = if let Some(entity_result) = results.get(&ecsr) {
                    entity_result
                } else {
                    results.insert(ecsr);
                    results
                        .get(&EntityConfigSearchResult::new(intern_str(&entity.name)))
                        .unwrap()
                };
                let mut vec = ecsr.examples.lock();
                let example = (entity.clone(), map_path.clone(), room_idx);
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = example;
                } else {
                    vec.push(example);
                }
            }
        }
    }
}

fn scan_triggers(
    results: &mut HashSet<TriggerConfigSearchResult>,
    filter: &ConfigSearchFilter,
    attrs: &HashSet<&str>,
    map: &CelesteMap,
    map_path: &MapPath,
    palette: &ModuleAggregate,
) {
    for (room_idx, room) in map.levels.iter().enumerate() {
        for entity in &room.triggers {
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
                        entity.attributes.iter().any(|(key, _)| {
                            !attrs.contains(key.as_str())
                                && !config.attribute_info.contains_key(key.as_str())
                        })
                    }),
                ConfigSearchFilter::Matches(s) => entity
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let tcsr = TriggerConfigSearchResult::new(intern_str(&entity.name));
                let tcsr = if let Some(trigger_result) = results.get(&tcsr) {
                    trigger_result
                } else {
                    results.insert(tcsr);
                    results
                        .get(&TriggerConfigSearchResult::new(intern_str(&entity.name)))
                        .unwrap()
                };
                let mut vec = tcsr.examples.lock();
                let example = (entity.clone(), map_path.clone(), room_idx);
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = example;
                } else {
                    vec.push(example);
                }
            }
        }
    }
}

fn scan_stylegrounds(
    results: &mut HashSet<StylegroundConfigSearchResult>,
    filter: &ConfigSearchFilter,
    attrs: &HashSet<&str>,
    map: &CelesteMap,
    map_path: &MapPath,
    palette: &ModuleAggregate,
) {
    for (b, g) in [&map.foregrounds, &map.backgrounds].into_iter().enumerate() {
        for (idx, style) in g.iter().enumerate() {
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
                        style.attributes.iter().any(|(key, _)| {
                            !attrs.contains(key.as_str())
                                && !config.attribute_info.contains_key(key.as_str())
                        })
                    }),
                ConfigSearchFilter::Matches(s) => style
                    .name
                    .to_ascii_lowercase()
                    .contains(&s.to_ascii_lowercase()),
            };
            if included {
                let scsr = StylegroundConfigSearchResult::new(intern_str(&style.name));
                let scsr = if let Some(style_result) = results.get(&scsr) {
                    style_result
                } else {
                    results.insert(scsr);
                    results
                        .get(&StylegroundConfigSearchResult::new(intern_str(&style.name)))
                        .unwrap()
                };
                let mut vec = scsr.examples.lock();
                let example = (
                    style.clone(),
                    map_path.clone(),
                    StylegroundSelection { fg: b == 0, idx },
                );
                if vec.len() == 100 {
                    vec[rand::random::<usize>() % 100] = example;
                } else {
                    vec.push(example);
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
                    .class("list_highlight")
                    .bind(
                        ctab.then(ConfigEditorTab::selected_result),
                        move |handle, selected| {
                            let selected = selected.get(handle.cx);
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

pub fn build_item_editor(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    let tab = cx.data::<AppState>().unwrap().current_tab;
    let config_lens = ctab.then(ConfigEditorTab::editing_config).unwrap();
    config_editor_textbox(cx, config_lens.then(AnyConfig::entity), move |cx, c| {
        cx.emit(AppEvent::EditConfig {
            tab,
            config: Box::new(AnyConfig::Entity(c)),
        })
    });
    config_editor_textbox(cx, config_lens.then(AnyConfig::trigger), move |cx, c| {
        cx.emit(AppEvent::EditConfig {
            tab,
            config: Box::new(AnyConfig::Trigger(c)),
        })
    });
    config_editor_textbox(
        cx,
        config_lens.then(AnyConfig::styleground),
        move |cx, c| {
            cx.emit(AppEvent::EditConfig {
                tab,
                config: Box::new(AnyConfig::Styleground(c)),
            })
        },
    );
    Label::new(cx, ctab.then(ConfigEditorTab::error_message));
    HStack::new(cx, move |cx| {
        Button::new(
            cx,
            move |cx| {
                let app = cx.data::<AppState>().unwrap();
                let config = ctab.view(app, |ctab| {
                    ctab.unwrap().editing_config.as_ref().unwrap().clone()
                });
                let text = match config {
                    AnyConfig::Entity(e) => e.to_string(),
                    AnyConfig::Trigger(e) => e.to_string(),
                    AnyConfig::Styleground(e) => e.to_string(),
                };
                let default = PathBuf::from_str(".").unwrap();
                let path = if !app.config.last_filepath.is_dir() {
                    &default
                } else {
                    &app.config.last_filepath
                };
                let result = dialog::FileSelection::new("Save Config")
                    .mode(dialog::FileSelectionMode::Save)
                    .path(path)
                    .show()
                    .unwrap();
                if let Some(result) = result {
                    if let Err(e) = std::fs::write(&result, text) {
                        dialog::Message::new(format!("Could not save file: {}", e))
                            .title("Error")
                            .show()
                            .unwrap();
                    }
                    let result_path: PathBuf = result.into();
                    cx.emit(AppEvent::EditSettings {
                        setter: AppConfigSetter::LastFilepath(
                            result_path
                                .parent()
                                .unwrap_or_else(|| Path::new("/"))
                                .to_owned(),
                        ),
                    });
                }
            },
            move |cx| Label::new(cx, "Save"),
        );
        Button::new(
            cx,
            move |cx| {
                let app = cx.data::<AppState>().unwrap();
                let tab = app.current_tab;
                let mut config: AnyConfig = config_lens.get(cx);
                set_default_draw(&mut config, app);
                cx.emit(AppEvent::EditConfig {
                    tab,
                    config: Box::new(config),
                });
            },
            move |cx| Label::new(cx, "Default Draw"),
        );
        Button::new(
            cx,
            move |cx| {
                let app = cx.data::<AppState>().unwrap();
                let tab = app.current_tab;
                let mut config: AnyConfig = config_lens.get(cx);
                let attrs = ctab.then(ConfigEditorTab::attribute_filter).get(cx);
                let attrs = attrs.split(',').collect::<HashSet<_>>();
                let result = ctab.view(app, |ctab| {
                    ctab.unwrap()
                        .search_results
                        .get(ctab.unwrap().selected_result)
                        .unwrap()
                        .clone()
                });
                analyze_uses(&mut config, &result, &attrs);
                cx.emit(AppEvent::EditConfig {
                    tab,
                    config: Box::new(config),
                });
            },
            move |cx| Label::new(cx, "Analyze Attrs"),
        );
    })
    .id("config_editor_toolbar");
}

fn config_editor_textbox<T>(
    cx: &mut Context,
    lens: impl Copy + Lens<Target = T>,
    on_edit: impl 'static + Send + Sync + Clone + Fn(&mut EventContext, T),
) where
    T: FromStr + std::fmt::Display + PartialEq + Clone,
    <T as FromStr>::Err: ToString,
{
    let tab = cx.data::<AppState>().unwrap().current_tab;
    Binding::new(cx, IsFailedLens::new(lens), move |cx, failed| {
        if !failed.get(cx) {
            let on_edit = on_edit.clone();
            Textbox::new_multiline(cx, lens, true)
                .on_edit(move |cx, text| match text.parse() {
                    Ok(t) => {
                        on_edit(cx, t);
                        cx.emit(AppEvent::SetConfigErrorMessage {
                            tab,
                            message: "".to_owned(),
                        });
                    }
                    Err(e) => {
                        cx.emit(AppEvent::SetConfigErrorMessage {
                            tab,
                            message: e.to_string(),
                        });
                    }
                })
                .id("config_editor_box");
        }
    });
}

pub fn build_item_preview(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    let entity_config_lens = ctab
        .then(ConfigEditorTab::editing_config)
        .then(UnwrapLens::new())
        .then(AnyConfig::entity);
    let entity_data_lens = ctab.then(ConfigEditorTab::preview_entity);
    Binding::new(
        cx,
        IsFailedLens::new(entity_config_lens),
        move |cx, is_failed| {
            if !is_failed.get(cx) {
                EntityConfigPreviewModel::default().build(cx);
                HStack::new(cx, move |cx| {
                    EntityConfigPreview::new(cx, entity_config_lens, entity_data_lens);
                    ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                        VStack::new(cx, move |cx| {
                            HStack::new(cx, move |cx| {
                                Label::new(cx, "Show Hitboxes");
                                Checkbox::new(cx, EntityConfigPreviewModel::show_boxes).on_toggle(
                                    move |cx| {
                                        let new_val = !EntityConfigPreviewModel::show_boxes.get(cx);
                                        cx.emit(EntityConfigPreviewModelSetter::ShowBoxes(new_val));
                                    },
                                );
                            });
                            HStack::new(cx, move |cx| {
                                Label::new(cx, "Show As Selected");
                                Checkbox::new(cx, EntityConfigPreviewModel::show_selected)
                                    .on_toggle(move |cx| {
                                        let new_val =
                                            !EntityConfigPreviewModel::show_selected.get(cx);
                                        cx.emit(EntityConfigPreviewModelSetter::ShowSelected(
                                            new_val,
                                        ));
                                    });
                            });

                            build_entity_tweaker(cx);
                        });
                    });
                });
            }
        },
    )
}

fn build_entity_tweaker(cx: &mut Context) {
    let ctab = CurrentTabImplLens {}.then(AppTab::config_editor);
    let config_lens = ctab
        .then(ConfigEditorTab::editing_config)
        .then(UnwrapLens::new())
        .then(AnyConfig::entity);
    let entity_lens = ctab.then(ConfigEditorTab::preview_entity);

    let attributes_lens = entity_lens.then(CelesteMapEntity::attributes);
    HStack::new(cx, move |cx| {
        Label::new(cx, "x");
        Textbox::new(cx, entity_lens.then(CelesteMapEntity::x)).on_edit(edit_x);
    });
    HStack::new(cx, move |cx| {
        Label::new(cx, "y");
        Textbox::new(cx, entity_lens.then(CelesteMapEntity::y)).on_edit(edit_y);
    });

    Binding::new(
        cx,
        config_lens.then(EntityConfig::resizable_x),
        move |cx, rx| {
            let rx = rx.get(cx);
            if rx {
                HStack::new(cx, move |cx| {
                    Label::new(cx, "width");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::width)).on_edit(edit_w);
                });
            }
        },
    );
    Binding::new(
        cx,
        config_lens.then(EntityConfig::resizable_y),
        move |cx, ry| {
            let ry = ry.get(cx);
            if ry {
                HStack::new(cx, move |cx| {
                    Label::new(cx, "height");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::height)).on_edit(edit_h);
                });
            }
        },
    );

    basic_attrs_editor(
        cx,
        attributes_lens,
        config_lens.then(EntityConfig::attribute_info),
        edit_attribute,
    );

    Label::new(cx, "Nodes");
    List::new(
        cx,
        entity_lens.then(CelesteMapEntity::nodes),
        move |cx, idx, item| {
            HStack::new(cx, move |cx| {
                Label::new(cx, "x");
                Textbox::new(cx, item.map(|pair| pair.x)).on_edit(move |cx, text| {
                    edit_node_x(cx, idx, text);
                });
                Label::new(cx, "y");
                Textbox::new(cx, item.map(|pair| pair.y)).on_edit(move |cx, text| {
                    edit_node_y(cx, idx, text);
                });
                Label::new(cx, "\u{e15b}")
                    .class("icon")
                    .class("remove_btn")
                    .on_press(move |cx| {
                        remove_node(cx.as_mut(), idx);
                    });
            });
        },
    );
    Button::new(cx, add_node, |cx| Label::new(cx, "+ Node"));
}

#[derive(Debug)]
pub struct EntityConfigPreview<L1, L2> {
    config: L1,
    entity: L2,
}

impl<L1, L2> EntityConfigPreview<L1, L2>
where
    L1: Lens<Target = EntityConfig>,
    L2: Lens<Target = CelesteMapEntity, Source = <L1 as Lens>::Source>,
{
    fn new(cx: &mut Context, config: L1, entity: L2) -> Handle<'_, Self> {
        Self { config, entity }.build(cx, |_| {})
    }
}

impl<L1, L2> View for EntityConfigPreview<L1, L2>
where
    L1: Lens<Target = EntityConfig>,
    L2: Lens<Target = CelesteMapEntity, Source = <L1 as Lens>::Source>,
{
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();

        canvas.save();
        canvas.translate(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
        canvas.scissor(-bounds.w / 2.0, -bounds.h / 2.0, bounds.w, bounds.h);

        let mut path = PPath::new();
        path.rect(-bounds.w / 2.0, -bounds.h / 2.0, bounds.w, bounds.h);
        canvas.fill_path(
            &mut path,
            &Paint::linear_gradient(
                0.0,
                -bounds.h / 2.0,
                0.0,
                bounds.h / 2.0,
                Color::black().into(),
                Color::blue().into(),
            ),
        );

        let mut path = PPath::new();
        path.move_to(0.0, -bounds.h / 2.0);
        path.line_to(0.0, bounds.h / 2.0);
        path.move_to(-bounds.w / 2.0, 0.0);
        path.line_to(bounds.w / 2.0, 0.0);
        canvas.stroke_path(
            &mut path,
            &Paint::color(Color::cyan().into()).with_line_width(cx.style.dpi_factor as f32),
        );

        canvas.scale(cx.style.dpi_factor as f32, cx.style.dpi_factor as f32);

        self.config.view(cx.data().unwrap(), |config| {
            if let Some(config) = config {
                self.entity.view(cx.data().unwrap(), |entity| {
                    if let Some(entity) = entity {
                        draw_entity(
                            config,
                            &cx.data::<AppState>().unwrap().omni_palette,
                            canvas,
                            entity,
                            &TileGrid::empty(),
                            EntityConfigPreviewModel::show_selected.get(cx),
                            &TileGrid::empty(),
                        );

                        if EntityConfigPreviewModel::show_boxes.get(cx) {
                            let env = make_entity_env(entity);
                            let mut path = PPath::new();
                            for rect in config.hitboxes.initial_rects.iter() {
                                if let Ok(rect) = rect.evaluate_float(&env) {
                                    path.rect(
                                        rect.min_x(),
                                        rect.min_y(),
                                        rect.width(),
                                        rect.height(),
                                    );
                                }
                            }
                            for idx in 0..entity.nodes.len() {
                                let env = make_node_env(entity, env.clone(), idx);
                                for rect in config.hitboxes.node_rects.iter() {
                                    if let Ok(rect) = rect.evaluate_float(&env) {
                                        path.rect(
                                            rect.min_x(),
                                            rect.min_y(),
                                            rect.width(),
                                            rect.height(),
                                        );
                                    }
                                }
                            }

                            canvas.fill_path(
                                &mut path,
                                &Paint::color(arborio_utils::vizia::vg::Color::rgba(
                                    255, 255, 0, 128,
                                )),
                            );
                        }
                    }
                })
            }
        });
        canvas.restore();
    }
}

fn edit_attribute(cx: &mut EventContext, key: String, value: Attribute) {
    edit_entity(cx, move |entity| {
        entity.attributes.insert(key, value);
    });
}

fn edit_x(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.x = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_y(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.y = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_w(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.width = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_h(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.height = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_node_x(cx: &mut EventContext, idx: usize, value: String) {
    if let Ok(x) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.nodes[idx] = (x, entity.nodes[idx].y).into();
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_node_y(cx: &mut EventContext, idx: usize, value: String) {
    if let Ok(y) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.nodes[idx] = (entity.nodes[idx].x, y).into();
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn remove_node(cx: &mut EventContext, idx: usize) {
    edit_entity(cx, move |entity| {
        entity.nodes.remove(idx);
    })
}

fn add_node(cx: &mut EventContext) {
    edit_entity(cx, |entity| {
        entity.nodes.push((entity.x, entity.y).into());
    });
}

fn edit_entity<F: FnOnce(&mut CelesteMapEntity)>(cx: &mut EventContext, f: F) {
    let app_state = cx.data::<AppState>().unwrap();
    let mut entity = match app_state.tabs.get(app_state.current_tab) {
        Some(AppTab::ConfigEditor(ctab)) => ctab.preview_entity.clone(),
        _ => panic!("How'd you do that"),
    };

    f(&mut entity);

    cx.emit(AppEvent::EditPreviewEntity {
        tab: app_state.current_tab,
        entity,
    });
}
