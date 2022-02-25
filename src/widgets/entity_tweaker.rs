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
                            Textbox::new(cx, entity_lens.then(CelesteMapEntity::width))
                                .on_edit(edit_w);
                        });
                        HStack::new(cx, move |cx| {
                            Label::new(cx, "height");
                            Textbox::new(cx, entity_lens.then(CelesteMapEntity::height))
                                .on_edit(edit_h);
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

                                let attr_value_lens =
                                    HashMapIndexWithLens::new(attributes_lens, key_lens);
                                let s_value_lens = attr_value_lens.then(Attribute::text);
                                let i_value_lens = attr_value_lens.then(Attribute::int);
                                let f_value_lens = attr_value_lens.then(Attribute::float);
                                let b_value_lens = attr_value_lens.then(Attribute::bool);
                                Binding::new(
                                    cx,
                                    IsFailedLens::new(s_value_lens),
                                    move |cx, failed| {
                                        if !*failed.get(cx) {
                                            Textbox::new(cx, s_value_lens).on_edit(
                                                move |cx, text| {
                                                    edit_attribute(
                                                        cx,
                                                        key_lens.get(cx).to_string(),
                                                        Attribute::Text(text),
                                                    );
                                                },
                                            );
                                        }
                                    },
                                );
                                Binding::new(
                                    cx,
                                    IsFailedLens::new(i_value_lens),
                                    move |cx, failed| {
                                        if !*failed.get(cx) {
                                            Textbox::new(cx, i_value_lens).on_edit(
                                                move |cx, text| {
                                                    if let Ok(i) = text.parse() {
                                                        edit_attribute(
                                                            cx,
                                                            key_lens.get(cx).to_string(),
                                                            Attribute::Int(i),
                                                        );
                                                    }
                                                },
                                            );
                                        }
                                    },
                                );
                                Binding::new(
                                    cx,
                                    IsFailedLens::new(f_value_lens),
                                    move |cx, failed| {
                                        if !*failed.get(cx) {
                                            Textbox::new(cx, f_value_lens).on_edit(
                                                move |cx, text| {
                                                    if let Ok(i) = text.parse() {
                                                        edit_attribute(
                                                            cx,
                                                            key_lens.get(cx).to_string(),
                                                            Attribute::Float(i),
                                                        );
                                                    }
                                                },
                                            );
                                        }
                                    },
                                );
                                Binding::new(cx, b_value_lens, move |cx, b| {
                                    if let Some(b) = b.get_fallible(cx) {
                                        Checkbox::new(cx, *b).on_toggle(move |cx| {
                                            edit_attribute(
                                                cx,
                                                key_lens.get(cx).to_string(),
                                                Attribute::Bool(!*b),
                                            );
                                        });
                                    }
                                });
                            });
                        }
                    },
                );
            })
            .class("tweaker")
    }
}

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("entity-tweaker".to_owned())
    }
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
        })
    }
}
fn edit_y(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.y = value;
        })
    }
}
fn edit_w(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.width = value;
        })
    }
}
fn edit_h(cx: &mut Context, value: String) {
    if let Ok(value) = value.parse() {
        edit_entity(cx, move |entity| {
            entity.height = value;
        })
    }
}
