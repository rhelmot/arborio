use celeste::binel::BinElAttr;
use vizia::*;

use crate::app_state::{AppEvent, AppSelection, AppState, AppTab};
use crate::lenses::CurrentSelectedEntityLens;

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build2(cx, move |cx| {
            let lens = CurrentSelectedEntityLens {};
            Binding::new(cx, lens, move |cx, selection| {
                if let Some(entity) = selection.get_fallible(cx) {
                    VStack::new(cx, move |cx| {
                        let msg = format!("{} - {}", entity.name, entity.id);
                        let mut attrs = entity
                            .attributes
                            .keys()
                            .map(|k| k.into())
                            .collect::<Vec<String>>();
                        attrs.sort();
                        Label::new(cx, &msg);

                        for attr in attrs {
                            HStack::new(cx, move |cx| {
                                Label::new(cx, &attr);

                                let app_state = cx.data::<AppState>().unwrap();
                                let trigger = matches!(
                                    app_state.current_selected,
                                    Some(
                                        AppSelection::EntityBody(_, true)
                                            | AppSelection::EntityNode(_, _, true)
                                    )
                                );
                                let (current_map, current_room) =
                                    match app_state.tabs.get(app_state.current_tab) {
                                        Some(AppTab::Map(map_tab)) => {
                                            (map_tab.id.clone(), map_tab.current_room)
                                        }
                                        _ => panic!("How'd you do that"),
                                    };

                                let entity = selection.get(cx);
                                let val = &entity.attributes[&attr];
                                match val {
                                    BinElAttr::Bool(b) => {
                                        let b = *b;
                                        Checkbox::new(cx, b).on_toggle(move |cx| {
                                            let mut entity = selection.get(cx).clone();
                                            entity
                                                .attributes
                                                .insert(attr.to_string(), BinElAttr::Bool(!b));

                                            cx.emit(AppEvent::EntityUpdate {
                                                map: current_map.clone(),
                                                room: current_room,
                                                entity,
                                                trigger,
                                            });
                                        });
                                    }
                                    BinElAttr::Int(i) => {
                                        let i = *i;
                                        IntVal { item: i }.build(cx);
                                        Textbox::new(cx, IntVal::item).on_edit(move |cx, text| {
                                            if let Ok(i) = text.parse() {
                                                let mut entity = selection.get(cx).clone();
                                                entity
                                                    .attributes
                                                    .insert(attr.to_string(), BinElAttr::Int(i));

                                                cx.emit(AppEvent::EntityUpdate {
                                                    map: current_map.clone(),
                                                    room: current_room,
                                                    entity,
                                                    trigger,
                                                });
                                            }
                                        });
                                    }
                                    BinElAttr::Float(f) => {
                                        let f = *f;
                                        FloatVal { item: f }.build(cx);
                                        Textbox::new(cx, StringVal::item).on_edit(
                                            move |cx, text| {
                                                if let Ok(f) = text.parse() {
                                                    let mut entity = selection.get(cx).clone();
                                                    entity.attributes.insert(
                                                        attr.to_string(),
                                                        BinElAttr::Float(f),
                                                    );

                                                    cx.emit(AppEvent::EntityUpdate {
                                                        map: current_map.clone(),
                                                        room: current_room,
                                                        entity,
                                                        trigger,
                                                    });
                                                }
                                            },
                                        );
                                    }
                                    BinElAttr::Text(s) => {
                                        StringVal { item: s.clone() }.build(cx);
                                        Textbox::new(cx, StringVal::item).on_edit(
                                            move |cx, text| {
                                                if let Ok(s) = text.parse() {
                                                    let mut entity = selection.get(cx).clone();
                                                    entity.attributes.insert(
                                                        attr.to_string(),
                                                        BinElAttr::Text(s),
                                                    );

                                                    cx.emit(AppEvent::EntityUpdate {
                                                        map: current_map.clone(),
                                                        room: current_room,
                                                        entity,
                                                        trigger,
                                                    });
                                                }
                                            },
                                        );
                                    }
                                }
                            });
                        }
                    });
                }
            });
        })
    }
}

impl View for EntityTweakerWidget {
    fn element(&self) -> Option<String> {
        Some("tweaker".to_owned())
    }
}

#[derive(Lens)]
pub struct IntVal {
    pub item: i32,
}
impl Model for IntVal {}
#[derive(Lens)]
pub struct FloatVal {
    pub item: f32,
}
impl Model for FloatVal {}
#[derive(Lens)]
pub struct StringVal {
    pub item: String,
}
impl Model for StringVal {}
