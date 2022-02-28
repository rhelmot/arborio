use std::str::FromStr;
use vizia::*;

use crate::app_state::{AppEvent, AppSelection, AppState, AppTab};
use crate::lenses::{
    CurrentSelectedEntityLens, HashMapIndexWithLens, HashMapLenLens, HashMapNthKeyLens,
    IsFailedLens,
};
use crate::map_struct::{Attribute, CelesteMapEntity};

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}
            .build2(cx, move |cx| {
                let entity_lens = CurrentSelectedEntityLens {};
                Binding::new(cx, entity_lens, move |cx, selection| {
                    if let Some(entity) = selection.get_fallible(cx) {
                        let msg = format!("{} - {}", entity.name, entity.id);
                        Label::new(cx, &msg);
                    } else {
                        Label::new(cx, "No entity selected");
                    }
                });

                ScrollView::new(cx, 0.0, 0.0, false, true, Self::members);
            })
            .class("tweaker")
    }

    fn members(cx: &mut Context) {
        let entity_lens = CurrentSelectedEntityLens {};
        Binding::new(cx, IsFailedLens::new(entity_lens), move |cx, failed| {
            if !*failed.get(cx) {
                HStack::new(cx, move |cx| {
                    Label::new(cx, "x");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::x)).on_edit(edit_x);
                });
                HStack::new(cx, move |cx| {
                    Label::new(cx, "y");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::y)).on_edit(edit_y);
                });
                HStack::new(cx, move |cx| {
                    Label::new(cx, "width");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::width)).on_edit(edit_w);
                });
                HStack::new(cx, move |cx| {
                    Label::new(cx, "height");
                    Textbox::new(cx, entity_lens.then(CelesteMapEntity::height)).on_edit(edit_h);
                });
            }
        });

        let attributes_lens = entity_lens.then(CelesteMapEntity::attributes);
        Binding::new(
            cx,
            attributes_lens.then(HashMapLenLens::new()),
            move |cx, len| {
                let len = len.get_fallible(cx).map(|x| *x).unwrap_or(0);
                for i in 0..len {
                    let key_lens = attributes_lens.then(HashMapNthKeyLens::new(i));
                    HStack::new(cx, move |cx| {
                        Label::new(cx, key_lens);

                        let attr_value_lens = HashMapIndexWithLens::new(attributes_lens, key_lens);
                        let s_value_lens = attr_value_lens.then(Attribute::text);
                        let i_value_lens = attr_value_lens.then(Attribute::int);
                        let f_value_lens = attr_value_lens.then(Attribute::float);
                        let b_value_lens = attr_value_lens.then(Attribute::bool);

                        attr_editor(cx, s_value_lens, key_lens, Attribute::Text);
                        attr_editor(cx, i_value_lens, key_lens, Attribute::Int);
                        attr_editor(cx, f_value_lens, key_lens, Attribute::Float);
                        Binding::new(cx, IsFailedLens::new(b_value_lens), move |cx, failed| {
                            if !*failed.get(cx) {
                                Checkbox::new(cx, b_value_lens).on_toggle(move |cx| {
                                    let b = *b_value_lens.get(cx);
                                    edit_attribute(
                                        cx,
                                        key_lens.get(cx).to_string(),
                                        Attribute::Bool(!b),
                                    );
                                });
                            }
                        });
                    });
                }
            },
        );

        Binding::new(cx, IsFailedLens::new(entity_lens), move |cx, failed| {
            if !*failed.get(cx) {
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
                            Label::new(cx, "-").class("remove_btn").on_press(move |cx| {
                                remove_node(cx, idx);
                            });
                        });
                    },
                );
                Button::new(cx, add_node, |cx| Label::new(cx, "+ Node"));
            }
        });
    }
}

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("entity-tweaker".to_owned())
    }
}

fn attr_editor<T: ToString + FromStr + Data>(
    cx: &mut Context,
    lens: impl Lens<Target = T>,
    key: impl Send + Sync + Lens<Target = String>,
    translator: impl 'static + Clone + Send + Sync + Fn(T) -> Attribute,
) {
    Binding::new(cx, IsFailedLens::new(lens.clone()), move |cx, failed| {
        if !*failed.get(cx) {
            let key = key.clone();
            let translator = translator.clone();
            Textbox::new(cx, lens.clone()).on_edit(move |cx, text| {
                if let Ok(value) = text.parse() {
                    edit_attribute(cx, key.get(cx).to_string(), translator(value));
                    cx.current.toggle_class(cx, "validation_error", false);
                } else {
                    cx.current.toggle_class(cx, "validation_error", true);
                }
            });
        }
    });
}

fn edit_entity<F: FnOnce(&mut CelesteMapEntity)>(cx: &mut Context, f: F) {
    let app_state = cx.data::<AppState>().unwrap();
    let (current_map, current_room, current_selected) =
        match app_state.tabs.get(app_state.current_tab) {
            Some(AppTab::Map(map_tab)) => (
                map_tab.id.clone(),
                map_tab.current_room,
                map_tab.current_selected,
            ),
            _ => panic!("How'd you do that"),
        };
    let trigger = matches!(
        current_selected,
        Some(AppSelection::EntityBody(_, true) | AppSelection::EntityNode(_, _, true))
    );

    let mut entity = (CurrentSelectedEntityLens {}).get(cx).take();

    f(&mut entity);

    cx.emit(AppEvent::EntityUpdate {
        map: current_map,
        room: current_room,
        entity,
        trigger,
    });
}

fn edit_attribute(cx: &mut Context, key: String, value: Attribute) {
    edit_entity(cx, move |entity| {
        entity.attributes.insert(key, value);
    });
}

fn edit_x(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.x = value;
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}
fn edit_y(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.y = value;
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}
fn edit_w(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.width = value;
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}
fn edit_h(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.height = value;
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}

fn edit_node_x(cx: &mut Context, idx: usize, value: String) {
    if let Ok(x) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.nodes[idx] = (x, entity.nodes[idx].y).into();
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}

fn edit_node_y(cx: &mut Context, idx: usize, value: String) {
    if let Ok(y) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.nodes[idx] = (entity.nodes[idx].x, y).into();
        });
        cx.current.toggle_class(cx, "validation_error", false);
    } else {
        cx.current.toggle_class(cx, "validation_error", true);
    }
}

fn remove_node(cx: &mut Context, idx: usize) {
    edit_entity(cx, move |entity| {
        entity.nodes.remove(idx);
    })
}

fn add_node(cx: &mut Context) {
    let mut select = None;
    let mut id = None;
    edit_entity(cx, |entity| {
        select = Some(entity.nodes.len());
        id = Some(entity.id);
        entity.nodes.push((entity.x, entity.y).into());
    });

    if let (Some(id), Some(select)) = (id, select) {
        let app_state = cx.data::<AppState>().unwrap();
        let current_tab = app_state.current_tab;
        let current_selected = match app_state.tabs.get(app_state.current_tab) {
            Some(AppTab::Map(map_tab)) => map_tab.current_selected,
            _ => panic!("How'd you do that"),
        };
        let trigger = matches!(
            current_selected,
            Some(AppSelection::EntityBody(_, true) | AppSelection::EntityNode(_, _, true))
        );
        cx.emit(AppEvent::SelectObject {
            tab: current_tab,
            selection: Some(AppSelection::EntityNode(id, select, trigger)),
        });
    }
}
