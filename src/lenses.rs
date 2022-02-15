use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use vizia::*;

use crate::app_state::{AppSelection, AppState, AppTab, MapTab};
use crate::assets::Interned;
use crate::auto_saver::AutoSaver;
use crate::map_struct::{CelesteMapEntity, MapID};
use crate::ModuleAggregate;

#[derive(Debug, Copy, Clone)]
pub struct CurrentMapLens {}

impl Lens for CurrentMapLens {
    type Source = AppState;
    type Target = MapID;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let data = match source.tabs.get(source.current_tab) {
            Some(AppTab::Map(maptab)) => Some(&maptab.id),
            _ => None,
        };

        map(data)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityLens {}

impl Lens for CurrentSelectedEntityLens {
    type Source = AppState;
    type Target = CelesteMapEntity;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let tab = if let Some(tab) = source.tabs.get(source.current_tab) {
            tab
        } else {
            return map(None);
        };
        let MapTab {
            id: map_id,
            current_room,
            ..
        } = if let AppTab::Map(t) = tab {
            t
        } else {
            return map(None);
        };
        let (entity_id, trigger) = match source.current_selected {
            Some(AppSelection::EntityBody(id, trigger)) => (id, trigger),
            Some(AppSelection::EntityNode(id, _, trigger)) => (id, trigger),
            _ => return map(None),
        };
        let cmap = if let Some(cmap) = source.loaded_maps.get(map_id) {
            cmap
        } else {
            return map(None);
        };
        let data = cmap
            .levels
            .get(*current_room)
            .and_then(|room| room.entity(entity_id, trigger));
        map(data)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentPaletteLens {}

impl Lens for CurrentPaletteLens {
    type Source = AppState;
    type Target = ModuleAggregate;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let tab = if let Some(tab) = source.tabs.get(source.current_tab) {
            tab
        } else {
            return map(None);
        };
        let MapTab { id: map_id, .. } = if let AppTab::Map(t) = tab {
            t
        } else {
            return map(None);
        };

        let data = source.palettes.get(&map_id.module);
        map(data)
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

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(source.deref()))
    }
}

#[derive(Debug)]
pub struct HashMapStringKeyLens<T> {
    key: Interned,
    t: PhantomData<T>,
}

impl<T> Clone for HashMapStringKeyLens<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            t: PhantomData::default(),
        }
    }
}

impl<T> Copy for HashMapStringKeyLens<T> {}

impl<T: Debug + 'static> Lens for HashMapStringKeyLens<T> {
    type Source = HashMap<String, T>;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(source.get(*self.key))
    }
}
