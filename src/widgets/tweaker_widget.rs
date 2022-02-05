use celeste::binel::BinElAttr;
use std::fmt::{Debug, Formatter};
use vizia::*;

use crate::app_state::{AppEvent, AppSelection, AppState, AppTab, CurrentSelectedEntityLens};
use crate::assets;
use crate::map_struct::{CelesteMap, CelesteMapEntity};
use crate::units::*;

pub struct EntityTweakerWidget {}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build2(cx, move |cx| {
            let lens = CurrentSelectedEntityLens {};
            Binding::new_fallible(
                cx,
                lens,
                move |cx, selection| {
                    VStack::new(cx, move |cx| {
                        let entity = selection.get(cx);
                        let msg = format!("{} - {}", entity.name, entity.id);
                        let mut attrs = entity
                            .attributes
                            .iter()
                            .map(|(a, b)| Some(a.clone()))
                            .collect::<Vec<_>>();
                        Label::new(cx, &msg);

                        ForEach::new(cx, 0..attrs.len(), move |cx, idx| {
                            let attr = attrs[idx].take().unwrap();
                            HStack::new(cx, move |cx| {
                                Label::new(cx, &attr);
                                let entity = selection.get(cx);
                                let val = &entity.attributes[&attr];
                                match val {
                                    BinElAttr::Bool(b) => {
                                        let b = *b;
                                        Checkbox::new(cx, b).on_toggle(move |cx| {
                                            let mut entity = selection.get(cx).clone();
                                            entity
                                                .attributes
                                                .insert(attr.clone(), BinElAttr::Bool(!b));
                                            let app_state = cx.data::<AppState>().unwrap();
                                            let trigger = matches!(
                                                app_state.current_selected,
                                                Some(
                                                    AppSelection::EntityBody(_, true)
                                                        | AppSelection::EntityNode(_, _, true)
                                                )
                                            );
                                            let (current_map, current_room) = match app_state.tabs.get(app_state.current_tab) {
                                                Some(AppTab::Map(map_tab)) => (map_tab.id.clone(), map_tab.current_room),
                                                _ => panic!("How'd you do that"),
                                            };

                                            cx.emit(AppEvent::EntityUpdate { map: current_map, room: current_room, entity, trigger });
                                        });
                                    }
                                    BinElAttr::Int(_) => {}
                                    BinElAttr::Float(_) => {}
                                    BinElAttr::Text(_) => {}
                                }
                            });
                        });
                    });
                },
                move |cx| {
                    Element::new(cx);
                },
            );
        })
    }
}

impl View for EntityTweakerWidget {}
