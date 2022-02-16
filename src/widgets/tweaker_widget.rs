use vizia::*;

use crate::lenses::{CurrentSelectedEntityLens, HashMapLenLens, HashMapNthKeyLens};
use crate::map_struct::CelesteMapEntity;

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build2(cx, move |cx| {
            VStack::new(cx, move |cx| {
                let entity_lens = CurrentSelectedEntityLens {};
                Binding::new(cx, entity_lens, move |cx, selection| {
                    if let Some(entity) = selection.get_fallible(cx) {
                        let msg = format!("{} - {}", entity.name, entity.id);
                        Label::new(cx, &msg);
                    } else {
                        Label::new(cx, "No entity selected");
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
                                Label::new(cx, "Attribute").bind(
                                    key_lens,
                                    move |handle, key_lens| {
                                        let key_lens = &*key_lens.get(handle.cx);
                                        handle.text(key_lens);
                                    },
                                );
                            });
                        }
                    },
                );
            });
        })
    }
}

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("tweaker".to_owned())
    }
}

pub struct TweakerWidgetAttribute {}

//impl TweakerWidgetAttribute {
//    pub fn new<L>(cx: &mut Context, attr: L) -> Handle<'_, Self> {
//        Self {}.build2(cx, move |cx| {
//            let app_state = cx.data::<AppState>().unwrap();
//            let trigger = matches!(
//                                    app_state.current_selected,
//                                    Some(
//                                        AppSelection::EntityBody(_, true)
//                                            | AppSelection::EntityNode(_, _, true)
//                                    )
//                                );
//            let (current_map, current_room) =
//                match app_state.tabs.get(app_state.current_tab) {
//                    Some(AppTab::Map(map_tab)) => {
//                        (map_tab.id.clone(), map_tab.current_room)
//                    }
//                    _ => panic!("How'd you do that"),
//                };
//
//            let entity = selection.get(cx);
//            let val = &entity.attributes[&attr];
//            match val {
//                BinElAttr::Bool(b) => {
//                    let b = *b;
//                    Checkbox::new(cx, b).on_toggle(move |cx| {
//                        let mut entity = selection.get(cx).clone();
//                        entity
//                            .attributes
//                            .insert(attr.to_string(), BinElAttr::Bool(!b));
//
//                        cx.emit(AppEvent::EntityUpdate {
//                            map: current_map.clone(),
//                            room: current_room,
//                            entity,
//                            trigger,
//                        });
//                    });
//                }
//                BinElAttr::Int(i) => {
//                    let i = *i;
//                    IntVal { item: i }.build(cx);
//                    Textbox::new(cx, IntVal::item).on_edit(move |cx, text| {
//                        if let Ok(i) = text.parse() {
//                            let mut entity = selection.get(cx).clone();
//                            entity
//                                .attributes
//                                .insert(attr.to_string(), BinElAttr::Int(i));
//
//                            cx.emit(AppEvent::EntityUpdate {
//                                map: current_map.clone(),
//                                room: current_room,
//                                entity,
//                                trigger,
//                            });
//                        }
//                    });
//                }
//                BinElAttr::Float(f) => {
//                    let f = *f;
//                    FloatVal { item: f }.build(cx);
//                    Textbox::new(cx, StringVal::item).on_edit(
//                        move |cx, text| {
//                            if let Ok(f) = text.parse() {
//                                let mut entity = selection.get(cx).clone();
//                                entity.attributes.insert(
//                                    attr.to_string(),
//                                    BinElAttr::Float(f),
//                                );
//
//                                cx.emit(AppEvent::EntityUpdate {
//                                    map: current_map.clone(),
//                                    room: current_room,
//                                    entity,
//                                    trigger,
//                                });
//                            }
//                        },
//                    );
//                }
//                BinElAttr::Text(s) => {
//                    StringVal { item: s.clone() }.build(cx);
//                    Textbox::new(cx, StringVal::item).on_edit(
//                        move |cx, text| {
//                            if let Ok(s) = text.parse() {
//                                let mut entity = selection.get(cx).clone();
//                                entity.attributes.insert(
//                                    attr.to_string(),
//                                    BinElAttr::Text(s),
//                                );
//
//                                cx.emit(AppEvent::EntityUpdate {
//                                    map: current_map.clone(),
//                                    room: current_room,
//                                    entity,
//                                    trigger,
//                                });
//                            }
//                        },
//                    );
//                }
//            }
//        })
//    }
//}

impl View for TweakerWidgetAttribute {}
