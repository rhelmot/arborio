use arborio_maploader::map_struct::{Attribute, CelesteMapEntity};
use arborio_modloader::config::AttributeType;
use arborio_state::data::action::RoomAction;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::selection::AppSelection;
use arborio_state::data::tabs::AppTab;
use arborio_state::data::{AppConfig, EventPhase};
use arborio_state::lenses::{
    current_selected_entity_lens, AutoSaverLens, CurrentSelectedEntityConfigAttributesLens,
    CurrentSelectedEntityResizableLens, IsFailedLens,
};
use arborio_utils::vizia::prelude::*;
use arborio_widgets_common::advanced_tweaker::advanced_attrs_editor;
use arborio_widgets_common::basic_tweaker::basic_attrs_editor;

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build(cx, move |cx| {
                let entity_lens = current_selected_entity_lens();
                Binding::new(
                    cx,
                    entity_lens.then(CelesteMapEntity::name),
                    move |cx, name| {
                        Binding::new(cx, entity_lens.then(CelesteMapEntity::id), move |cx, id| {
                            let name = name.get_fallible(cx);
                            let id = id.get_fallible(cx);
                            if let (Some(name), Some(id)) = (name, id) {
                                let msg = format!("{} - {}", name, id);
                                Label::new(cx, &msg);
                            } else {
                                Label::new(cx, "No entity selected");
                            }
                        });
                    },
                );
                Binding::new(cx, IsFailedLens::new(entity_lens), move |cx, failed| {
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
    let entity_lens = current_selected_entity_lens();
    let advanced_lens = AppState::config
        .then(AutoSaverLens::new())
        .then(AppConfig::advanced);
    let attributes_lens = entity_lens.then(CelesteMapEntity::attributes);
    HStack::new(cx, move |cx| {
        Label::new(cx, "x");
        Textbox::new(cx, entity_lens.then(CelesteMapEntity::x)).on_edit(edit_x);
    });
    HStack::new(cx, move |cx| {
        Label::new(cx, "y");
        Textbox::new(cx, entity_lens.then(CelesteMapEntity::y)).on_edit(edit_y);
    });

    Binding::new(cx, advanced_lens, move |cx, advanced| {
        let advanced = advanced.get(cx);
        Binding::new(
            cx,
            CurrentSelectedEntityResizableLens {},
            move |cx, resizable| {
                let (rx, ry) = resizable.get(cx);
                if advanced || rx {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "width");
                        Textbox::new(cx, entity_lens.then(CelesteMapEntity::width)).on_edit(edit_w);
                    });
                }
                if advanced || ry {
                    HStack::new(cx, move |cx| {
                        Label::new(cx, "height");
                        Textbox::new(cx, entity_lens.then(CelesteMapEntity::height))
                            .on_edit(edit_h);
                    });
                }
            },
        );

        if advanced {
            advanced_attrs_editor(
                cx,
                attributes_lens,
                edit_attribute,
                add_default_attribute,
                remove_attribute,
            );
        } else {
            let config_lens = CurrentSelectedEntityConfigAttributesLens {};
            basic_attrs_editor(cx, attributes_lens, config_lens, edit_attribute);
        }
    });

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

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<&'static str> {
        Some("entity-tweaker")
    }
}

fn edit_entity<F: FnOnce(&mut CelesteMapEntity)>(cx: &mut EventContext, f: F) {
    let app_state = cx.data::<AppState>().unwrap();
    let (current_map, current_room, current_selected) = match app_state
        .tabs
        .get(app_state.current_tab)
    {
        Some(AppTab::Map(map_tab)) => (map_tab.id, map_tab.current_room, map_tab.current_selected),
        _ => panic!("How'd you do that"),
    };
    let trigger = matches!(
        current_selected,
        Some(AppSelection::EntityBody(_, true) | AppSelection::EntityNode(_, _, true))
    );

    let mut entity = (current_selected_entity_lens()).get(cx);

    f(&mut entity);

    cx.emit(current_map.room_action(
        current_room,
        EventPhase::new(),
        RoomAction::EntityUpdate {
            entity: Box::new(entity),
            trigger,
        },
    )); // TODO batch correctly
}

fn edit_attribute(cx: &mut EventContext, key: String, value: Attribute) {
    edit_entity(cx, move |entity| {
        entity.attributes.insert(key, value);
    });
}

fn remove_attribute(cx: &mut EventContext, key: String) {
    edit_entity(cx, move |entity| {
        entity.attributes.remove(&key);
    });
}

fn add_default_attribute(cx: &mut EventContext, key: String, ty: AttributeType) {
    edit_entity(cx, move |entity| {
        entity.attributes.insert(
            key,
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
    let mut select = None;
    let mut id = None;
    edit_entity(cx, |entity| {
        select = Some(entity.nodes.len());
        id = Some(entity.id);
        entity.nodes.push((entity.x, entity.y).into());
    });

    if let (Some(id), Some(select)) = (id, select) {
        let app_state = cx.data::<AppState>().unwrap();
        let Some(AppTab::Map(map_tab)) = app_state.tabs.get(app_state.current_tab) else { panic!("How'd you do that") };
        let current_selected = map_tab.current_selected;
        let trigger = matches!(
            current_selected,
            Some(AppSelection::EntityBody(_, true) | AppSelection::EntityNode(_, _, true))
        );
        cx.emit(AppEvent::SelectObject {
            tab: app_state.current_tab,
            selection: Some(AppSelection::EntityNode(id, select, trigger)),
        });
    }
}
