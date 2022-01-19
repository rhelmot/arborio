use std::fmt::{Debug, Formatter};
use vizia::*;

use crate::units::*;
use crate::app_state::{AppState, AppSelection, AppEvent};
use crate::map_struct::{CelesteMap, CelesteMapEntity};

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityLens {

}

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

pub struct EntityTweakerWidget {

}

impl EntityTweakerWidget {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {

        }.build2(cx, move |cx| {
            Binding::new_fallible(cx, CurrentSelectedEntityLens::new(), |cx, selection| {
                let entity = selection.get(cx);
                let msg = format!("Selected entity {}: {}", entity.id, entity.name);
                Label::new(cx, &msg);
            }, move |cx| {
                Label::new(cx, "No entity selected");
            });
        })
    }
}

impl View for EntityTweakerWidget {
}
