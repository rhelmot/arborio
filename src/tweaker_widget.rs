use std::fmt::{Debug, Formatter};
use celeste::binel::BinElAttr;
use vizia::*;

use crate::units::*;
use crate::app_state::{AppState, AppSelection, AppEvent};
use crate::map_struct::{CelesteMap, CelesteMapEntity};
use crate::assets;

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityLens { }

impl Lens for CurrentSelectedEntityLens {
    type Source = AppState;
    type Target = CelesteMapEntity;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        match source.current_selected {
            Some(AppSelection::EntityBody(id)) => Some(id),
            Some(AppSelection::EntityNode(id, _)) => Some(id),
            _ => None,
        }.and_then(|id| source.map.as_ref().and_then(
            |map| map.levels.get(source.current_room).and_then(
                |room| room.entity(id)
            )))
    }
}

impl CurrentSelectedEntityLens {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct EntityTweakerWidget { }

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {

        }.build2(cx, move |cx| {
            let lens = CurrentSelectedEntityLens::new();
            Binding::new_fallible(cx, lens.clone(), move |cx, selection| {
                VStack::new(cx, move |cx| {
                    let entity = selection.get(cx);
                    let msg = format!("{} - {}", entity.name, entity.id);
                    let mut attrs = entity.attributes.iter().map(|(a, b)| Some(a.clone())).collect::<Vec<_>>();
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
                                    Checkbox::new(cx, b)
                                        .on_toggle(move |cx| {
                                            let mut entity = selection.get(cx).clone();
                                            entity.attributes.insert(attr.clone(), BinElAttr::Bool(!b));
                                            cx.emit(AppEvent::EntityUpdate { entity });
                                        });
                                }
                                BinElAttr::Int(_) => {}
                                BinElAttr::Float(_) => {}
                                BinElAttr::Text(_) => {}
                            }
                        });
                    });
                });
            }, move |cx| {
                Element::new(cx);
            });
        })
    }
}

impl View for EntityTweakerWidget {
}
