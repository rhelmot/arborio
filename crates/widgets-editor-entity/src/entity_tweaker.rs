use arborio_maploader::map_struct::{Attribute, CelesteMapEntity};
use arborio_modloader::config::AttributeType;
use arborio_state::data::action::RoomAction;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::selection::AppSelection;
use arborio_state::data::tabs::AppTab;
use arborio_state::data::{AppConfig, EventPhase};
use arborio_state::lenses::{
    current_selected_entity_lens, hash_map_nth_key_lens, AutoSaverLens,
    CurrentSelectedEntitiesAllLens, CurrentSelectedEntitiesAttributesLens,
    CurrentSelectedEntityConfigAttributesLens, CurrentSelectedEntityHasNodesLens,
    CurrentSelectedEntityResizableLens, HashMapIndexWithLens, HashMapLenLens, IsFailedLens,
};
use arborio_utils::vizia::fonts::icons_names::MINUS;
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::advanced_tweaker::advanced_attrs_editor;
use arborio_widgets_common::basic_tweaker::basic_attrs_editor;
use std::collections::HashSet;

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build(cx, move |cx| {
                let any_entity_lens = CurrentSelectedEntitiesAllLens::new_computed(|_, _| Some(()));

                Binding::new(cx, any_entity_lens.clone(), move |cx, any_entity_lens| {
                    let same_entity_lens =
                        CurrentSelectedEntitiesAllLens::new_referenced(|_, e| Some(&e.name));
                    Binding::new(cx, same_entity_lens, move |cx, same_entity_lens| {
                        let one_entity_lens =
                            CurrentSelectedEntitiesAllLens::new_computed(|_, e| Some(e.id));
                        let any_entity_lens = any_entity_lens.clone();
                        Binding::new(cx, one_entity_lens, move |cx, one_entity_lens| {
                            if any_entity_lens.get_fallible(cx).is_some() {
                                let name = same_entity_lens
                                    .get_fallible(cx)
                                    .unwrap_or_else(|| "<multiple>".to_owned());
                                let id = one_entity_lens.get_fallible(cx);
                                let msg = format!(
                                    "{} - {}",
                                    name,
                                    id.map_or_else(|| "*".to_owned(), |i| i.to_string())
                                );
                                Label::new(cx, &msg);
                            } else {
                                Label::new(cx, "No entity selected");
                            }
                        });
                    });
                });
                Binding::new(cx, IsFailedLens::new(any_entity_lens), move |cx, failed| {
                    if !failed.get(cx) {
                        ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                            VStack::new(cx, build_tweaker).class("tweaker_container");
                        });
                    }
                });
            })
            .class("tweaker")
    }
}

pub fn build_tweaker(cx: &mut Context) {
    let advanced_lens = AppState::config
        .then(AutoSaverLens::new())
        .then(AppConfig::advanced);
    HStack::new(cx, move |cx| {
        Label::new(cx, "x");
        Textbox::new(
            cx,
            CurrentSelectedEntitiesAllLens::new_referenced(|_, e| Some(&e.x)),
        )
        .on_edit(edit_x);
    });
    HStack::new(cx, move |cx| {
        Label::new(cx, "y");
        Textbox::new(
            cx,
            CurrentSelectedEntitiesAllLens::new_referenced(|_, e| Some(&e.y)),
        )
        .on_edit(edit_y);
    });

    Binding::new(cx, advanced_lens, move |cx, advanced| {
        let advanced = advanced.get(cx);
        Binding::new(
            cx,
            CurrentSelectedEntityResizableLens {},
            move |cx, resizable| {
                let (rx, ry) = resizable.get_fallible(cx).unwrap_or((true, true));
                if advanced || rx {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "width");
                        Textbox::new(
                            cx,
                            CurrentSelectedEntitiesAllLens::new_referenced(|_, e| Some(&e.width)),
                        )
                        .on_edit(edit_w);
                    });
                }
                if advanced || ry {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "height");
                        Textbox::new(
                            cx,
                            CurrentSelectedEntitiesAllLens::new_referenced(|_, e| Some(&e.height)),
                        )
                        .on_edit(edit_h);
                    });
                }
            },
        );

        if advanced {
            advanced_attrs_editor(
                cx,
                CurrentSelectedEntitiesAttributesLens::new_computed(|_, set| Some(set.len())),
                |index| {
                    CurrentSelectedEntitiesAttributesLens::new_referenced(move |_, set| {
                        let mut v = set.iter().collect::<Vec<_>>();
                        v.sort();
                        v.get(index).map(|x| **x)
                    })
                },
                |key_lens| {
                    CurrentSelectedEntitiesAllLens::new_referenced(move |app, ent| {
                        key_lens.view(app, |key| {
                            key.and_then(|key| ent.attributes.get(key.as_str()))
                        })
                    })
                },
                edit_attribute,
                add_default_attribute,
                remove_attribute,
            );
        } else {
            let config_lens = CurrentSelectedEntityConfigAttributesLens {};
            basic_attrs_editor(
                cx,
                config_lens.then(HashMapLenLens::new()),
                move |idx| {
                    (
                        config_lens.then(hash_map_nth_key_lens(idx)),
                        CurrentSelectedEntitiesAllLens::new_referenced(move |source, ent| {
                            config_lens
                                .then(hash_map_nth_key_lens(idx))
                                .view(source, |key| key.and_then(|key| ent.attributes.get(key)))
                        }),
                        HashMapIndexWithLens::new(
                            config_lens,
                            config_lens.then(hash_map_nth_key_lens(idx)),
                        ),
                    )
                },
                edit_attribute,
            );
        }

        let entity_lens = current_selected_entity_lens();
        Binding::new(cx, IsFailedLens::new(entity_lens), move |cx, failed| {
            Binding::new(
                cx,
                CurrentSelectedEntityHasNodesLens {},
                move |cx, has_nodes| {
                    if !failed.get(cx)
                        && (advanced || has_nodes.get_fallible(cx).unwrap_or_default())
                    {
                        Label::new(cx, "Nodes");
                        List::new(
                            cx,
                            entity_lens.then(CelesteMapEntity::nodes),
                            move |cx, idx, item| {
                                HStack::new(cx, move |cx| {
                                    Label::new(cx, "x");
                                    Textbox::new(cx, item.map(|pair| pair.x)).on_edit(
                                        move |cx, text| {
                                            edit_node_x(cx, idx, text);
                                        },
                                    );
                                    Label::new(cx, "y");
                                    Textbox::new(cx, item.map(|pair| pair.y)).on_edit(
                                        move |cx, text| {
                                            edit_node_y(cx, idx, text);
                                        },
                                    );
                                    Label::new(cx, MINUS)
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
                },
            );
        });
    });
}

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<&'static str> {
        Some("entity-tweaker")
    }
}

fn edit_entity<F: FnMut(&mut CelesteMapEntity, bool)>(cx: &mut EventContext, mut f: F) {
    let app_state = cx.data::<AppState>().unwrap();
    let (current_map, current_room, current_selected) = match app_state
        .tabs
        .get(app_state.current_tab)
    {
        Some(AppTab::Map(map_tab)) => (map_tab.id, map_tab.current_room, &map_tab.current_selected),
        _ => panic!("How'd you do that"),
    };

    let phase = EventPhase::new(); // TODO batch correctly (based on timeout)

    let mut events = vec![];
    for sel in current_selected {
        if let AppSelection::EntityBody(id, trigger) | AppSelection::EntityNode(id, _, trigger) =
            sel
        {
            if let Some(mut entity) = app_state
                .loaded_maps
                .get(&current_map)
                .and_then(|x| x.data.levels.get(current_room))
                .and_then(|x| x.entity(*id, *trigger))
                .cloned()
            {
                f(&mut entity, *trigger);

                events.push(current_map.room_action(
                    current_room,
                    phase,
                    RoomAction::EntityUpdate {
                        entity: Box::new(entity),
                        trigger: *trigger,
                    },
                ));
            }
        }
    }
    for event in events {
        cx.emit(event);
    }
}

fn edit_attribute(cx: &mut EventContext, key: String, value: Attribute) {
    edit_entity(cx, move |entity, _| {
        entity.attributes.insert(key.clone(), value.clone());
    });
}

fn remove_attribute(cx: &mut EventContext, key: String) {
    edit_entity(cx, move |entity, _| {
        entity.attributes.remove(&key);
    });
}

fn add_default_attribute(cx: &mut EventContext, key: String, ty: AttributeType) {
    edit_entity(cx, move |entity, _| {
        entity.attributes.insert(
            key.clone(),
            match ty {
                AttributeType::String => Attribute::Text("".to_owned()),
                AttributeType::Float => Attribute::Float(0.0),
                AttributeType::Int => Attribute::Int(0),
                AttributeType::Bool => Attribute::Bool(false),
            },
        );
    });
}

fn edit_x(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.x = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_y(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.y = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_w(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.width = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_h(cx: &mut EventContext, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.height = value;
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_node_x(cx: &mut EventContext, idx: usize, value: String) {
    if let Ok(x) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.nodes[idx] = (x, entity.nodes[idx].y).into();
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn edit_node_y(cx: &mut EventContext, idx: usize, value: String) {
    if let Ok(y) = value.parse() {
        edit_entity(cx, move |entity, _| {
            entity.nodes[idx] = (entity.nodes[idx].x, y).into();
        });
        cx.toggle_class("validation_error", false);
    } else {
        cx.toggle_class("validation_error", true);
    }
}

fn remove_node(cx: &mut EventContext, idx: usize) {
    edit_entity(cx, move |entity, _| {
        entity.nodes.remove(idx);
    })
}

fn add_node(cx: &mut EventContext) {
    let mut selection = HashSet::new();
    edit_entity(cx, |entity, trigger| {
        selection.insert(AppSelection::EntityNode(
            entity.id,
            entity.nodes.len(),
            trigger,
        ));
        entity.nodes.push((entity.x, entity.y).into());
    });

    cx.emit(AppEvent::SelectObjects {
        tab: cx.data::<AppState>().unwrap().current_tab,
        selection,
    });
}
