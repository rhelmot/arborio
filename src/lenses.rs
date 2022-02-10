use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use vizia::*;

use crate::app_state::{AppSelection, AppState, AppTab, MapTab};
use crate::auto_saver::AutoSaver;
use crate::map_struct::{CelesteMapEntity, MapID};
use crate::ModuleAggregate;

#[derive(Debug, Copy, Clone)]
pub struct CurrentMapLens {}

impl Lens for CurrentMapLens {
    type Source = AppState;
    type Target = MapID;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        match source.tabs.get(source.current_tab) {
            Some(AppTab::Map(maptab)) => Some(&maptab.id),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityLens {}

impl Lens for CurrentSelectedEntityLens {
    type Source = AppState;
    type Target = CelesteMapEntity;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        let tab = source.tabs.get(source.current_tab)?;
        let MapTab {
            id: map_id,
            current_room,
            ..
        } = if let AppTab::Map(t) = tab {
            t
        } else {
            return None;
        };
        let (entity_id, trigger) = match source.current_selected {
            Some(AppSelection::EntityBody(id, trigger)) => (id, trigger),
            Some(AppSelection::EntityNode(id, _, trigger)) => (id, trigger),
            _ => return None,
        };
        let map = source.loaded_maps.get(map_id)?;
        map.levels
            .get(*current_room)
            .and_then(|room| room.entity(entity_id, trigger))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentPaletteLens {}

impl Lens for CurrentPaletteLens {
    type Source = AppState;
    type Target = ModuleAggregate;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        let tab = source.tabs.get(source.current_tab)?;
        let MapTab { id: map_id, .. } = if let AppTab::Map(t) = tab {
            t
        } else {
            return None;
        };

        source.palettes.get(&map_id.module)
    }
}

#[derive(Debug)]
pub struct AutoSaverLens<T> {
    t: PhantomData<T>,
}

impl<T> Clone for AutoSaverLens<T> {
    fn clone(&self) -> Self {
        AutoSaverLens::new()
    }
}

impl<T> AutoSaverLens<T> {
    pub fn new() -> Self {
        Self {
            t: PhantomData::default(),
        }
    }
}

impl<T> Copy for AutoSaverLens<T> {}

impl<T: 'static + Debug> Lens for AutoSaverLens<T> {
    type Source = AutoSaver<T>;
    type Target = T;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        Some(source.deref())
    }
}

#[derive(Debug)]
pub struct UnwrapLens<T> {
    t: PhantomData<T>,
}

impl<T> Clone for UnwrapLens<T> {
    fn clone(&self) -> Self {
        UnwrapLens::new()
    }
}

impl<T> UnwrapLens<T> {
    pub fn new() -> Self {
        Self {
            t: PhantomData::default(),
        }
    }
}

impl<T> Copy for UnwrapLens<T> {}

impl<T: 'static + Debug> Lens for UnwrapLens<T> {
    type Source = Option<T>;
    type Target = T;

    fn view<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Target> {
        source.as_ref()
    }
}
