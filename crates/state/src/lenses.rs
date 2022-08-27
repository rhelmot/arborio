#![allow(clippy::new_without_default)]

use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;

use crate::auto_saver::AutoSaver;
use crate::data::action::StylegroundSelection;
use crate::data::app::AppState;
use crate::data::project_map::MapStateData;
use crate::data::selection::AppSelection;
use crate::data::tabs::{AppTab, MapTab};
use crate::data::MapID;
use arborio_maploader::map_struct::{
    Attribute, CelesteMapEntity, CelesteMapLevel, CelesteMapStyleground,
};
use arborio_modloader::aggregate::ModuleAggregate;

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

#[derive(Debug, Copy, Clone)]
pub struct CurrentMapImplLens {}

impl Lens for CurrentMapImplLens {
    type Source = AppState;
    type Target = MapStateData;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let data = match source.tabs.get(source.current_tab) {
            Some(AppTab::Map(maptab)) => Some(&maptab.id),
            Some(AppTab::MapMeta(id)) => Some(id),
            _ => None,
        };

        map(data.map(|id| &source.loaded_maps.get(id).unwrap().data))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CurrentRoomLens {}

impl Lens for CurrentRoomLens {
    type Source = AppState;
    type Target = CelesteMapLevel;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let maptab = if let Some(AppTab::Map(maptab)) = source.tabs.get(source.current_tab) {
            maptab
        } else {
            return map(None);
        };

        let the_map = if let Some(the_map) = source.loaded_maps.get(&maptab.id) {
            the_map
        } else {
            return map(None);
        };

        map(the_map
            .data
            .levels
            .get(maptab.current_room)
            .map(|x| &x.data))
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
            current_selected,
            ..
        } = if let AppTab::Map(t) = tab {
            t
        } else {
            return map(None);
        };
        let (entity_id, trigger) = match current_selected {
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
            .data
            .levels
            .get(*current_room)
            .and_then(|room| room.entity(*entity_id, *trigger));
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

        let data = source
            .loaded_maps
            .get(map_id)
            .map(|state| &state.cache.palette);
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

// #[derive(Debug)]
// pub struct HashMapStringKeyLens<T> {
//     key: Interned,
//     t: PhantomData<T>,
// }

// impl<T> HashMapStringKeyLens<T> {
//     pub fn new(key: Interned) -> Self {
//         Self {
//             key,
//             t: PhantomData::default(),
//         }
//     }
// }

// impl<T> Clone for HashMapStringKeyLens<T> {
//     fn clone(&self) -> Self {
//         Self {
//             key: self.key,
//             t: PhantomData::default(),
//         }
//     }
// }

// impl<T> Copy for HashMapStringKeyLens<T> {}

// impl<T: Debug + 'static> Lens for HashMapStringKeyLens<T> {
//     type Source = HashMap<&'static str, T>;
//     type Target = T;

//     fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
//         map(source.get(*self.key))
//     }
// }

#[derive(Debug)]
pub struct VecLenLens<T> {
    p: PhantomData<T>,
}

impl<T> Clone for VecLenLens<T> {
    fn clone(&self) -> Self {
        Self {
            p: PhantomData::default(),
        }
    }
}

impl<T> Copy for VecLenLens<T> {}

impl<T: 'static> Lens for VecLenLens<T> {
    type Source = Vec<T>;
    type Target = usize;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.len()))
    }
}

#[derive(Debug)]
pub struct HashMapLenLens<K, V> {
    p: PhantomData<(K, V)>,
}

impl<K, V> HashMapLenLens<K, V> {
    pub fn new() -> Self {
        Self {
            p: PhantomData::default(),
        }
    }
}

impl<K, V> Clone for HashMapLenLens<K, V> {
    fn clone(&self) -> Self {
        Self {
            p: PhantomData::default(),
        }
    }
}

impl<K, V> Copy for HashMapLenLens<K, V> {}

impl<K: 'static, V: 'static> Lens for HashMapLenLens<K, V> {
    type Source = HashMap<K, V>;
    type Target = usize;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.len()))
    }
}

#[derive(Debug)]
pub struct HashMapNthKeyLens<K, V> {
    idx: usize,
    p: PhantomData<(K, V)>,
}

impl<K, V> HashMapNthKeyLens<K, V> {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,
            p: PhantomData::default(),
        }
    }
}

impl<K, V> Clone for HashMapNthKeyLens<K, V> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            p: PhantomData::default(),
        }
    }
}

impl<K, V> Copy for HashMapNthKeyLens<K, V> {}

impl<K: 'static + Ord, V: 'static> Lens for HashMapNthKeyLens<K, V> {
    type Source = HashMap<K, V>;
    type Target = K;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let mut keys = source.keys().collect::<Vec<_>>();
        keys.sort();
        map(keys.get(self.idx).copied())
    }
}

#[derive(Debug)]
pub struct VecIndexWithLens<L1, L2, T> {
    l1: L1,
    l2: L2,
    t: PhantomData<T>,
}

impl<L1, L2, T> VecIndexWithLens<L1, L2, T> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self {
            l1,
            l2,
            t: PhantomData::default(),
        }
    }
}

impl<L1: Clone, L2: Clone, T> Clone for VecIndexWithLens<L1, L2, T> {
    fn clone(&self) -> Self {
        Self::new(self.l1.clone(), self.l2.clone())
    }
}

impl<L1: Copy, L2: Copy, T> Copy for VecIndexWithLens<L1, L2, T> {}

impl<L1, L2, T: 'static + Debug> Lens for VecIndexWithLens<L1, L2, T>
where
    L1: Lens<Target = Vec<T>>,
    L2: Lens<Source = <L1 as Lens>::Source, Target = usize>,
{
    type Source = <L1 as Lens>::Source;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        if let Some(index) = self.l2.view(source, |s| s.copied()) {
            self.l1.view(source, |s| map(s.and_then(|s| s.get(index))))
        } else {
            map(None)
        }
    }
}

#[derive(Debug)]
pub struct HashMapIndexWithLens<L1, L2, T> {
    l1: L1,
    l2: L2,
    t: PhantomData<T>,
}

impl<L1, L2, T> HashMapIndexWithLens<L1, L2, T> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self {
            l1,
            l2,
            t: PhantomData::default(),
        }
    }
}

impl<L1: Clone, L2: Clone, T> Clone for HashMapIndexWithLens<L1, L2, T> {
    fn clone(&self) -> Self {
        Self::new(self.l1.clone(), self.l2.clone())
    }
}

impl<L1: Copy, L2: Copy, T> Copy for HashMapIndexWithLens<L1, L2, T> {}

impl<L1, L2, T: 'static + Debug> Lens for HashMapIndexWithLens<L1, L2, T>
where
    L1: Lens<Target = HashMap<<L2 as Lens>::Target, T>>,
    L2: Lens<Source = <L1 as Lens>::Source>,
    <L2 as Lens>::Target: Hash + Eq,
{
    type Source = <L1 as Lens>::Source;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        self.l2.view(source, |idx| {
            self.l1.view(source, |hashmap| {
                map(match (hashmap, idx) {
                    (Some(x), Some(y)) => x.get(y),
                    _ => None,
                })
            })
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IsFailedLens<L> {
    lens: L,
}

impl<L> IsFailedLens<L> {
    pub fn new(lens: L) -> Self {
        Self { lens }
    }
}

impl<L: 'static + Lens> Lens for IsFailedLens<L> {
    type Source = L::Source;
    type Target = bool;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&self.lens.view(source, |opt| opt.is_none())))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RoomTweakerScopeLens {}

impl Lens for RoomTweakerScopeLens {
    type Source = AppState;
    type Target = (MapID, usize);

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let maptab = if let Some(AppTab::Map(maptab)) = source.tabs.get(source.current_tab) {
            maptab
        } else {
            return map(None);
        };

        map(Some(&(maptab.id, maptab.current_room)))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct AnotherLens<L1, L2> {
    one: L1,
    two: L2,
}

impl<L1, L2> AnotherLens<L1, L2> {
    pub fn new(one: L1, two: L2) -> Self {
        Self { one, two }
    }
}

impl<L1, L2> Lens for AnotherLens<L1, L2>
where
    L1: 'static + Lens,
    L2: 'static + Lens<Source = <L1 as Lens>::Source>,
    <L1 as Lens>::Target: Clone,
    <L2 as Lens>::Target: Clone,
{
    type Source = L1::Source;
    type Target = (L1::Target, L2::Target);

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        self.one.view(source, |one| {
            if let Some(one) = one {
                self.two.view(source, |two| {
                    if let Some(two) = two {
                        map(Some(&(one.clone(), two.clone())))
                    } else {
                        map(None)
                    }
                })
            } else {
                map(None)
            }
        })
    }
}

#[derive(Debug)]
pub struct RectXLens<T, U> {
    p: PhantomData<T>,
    u: PhantomData<U>,
}

impl<T, U> RectXLens<T, U> {
    pub fn new() -> Self {
        Self {
            p: PhantomData::default(),
            u: PhantomData::default(),
        }
    }
}

impl<T, U> Clone for RectXLens<T, U> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T, U> Copy for RectXLens<T, U> {}

impl<T: 'static, U: 'static> Lens for RectXLens<T, U> {
    type Source = Rect<T, U>;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.origin.x))
    }
}

#[derive(Debug)]
pub struct RectYLens<T, U> {
    p: PhantomData<T>,
    u: PhantomData<U>,
}

impl<T, U> RectYLens<T, U> {
    pub fn new() -> Self {
        Self {
            p: PhantomData::default(),
            u: PhantomData::default(),
        }
    }
}

impl<T, U> Clone for RectYLens<T, U> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T, U> Copy for RectYLens<T, U> {}

impl<T: 'static, U: 'static> Lens for RectYLens<T, U> {
    type Source = Rect<T, U>;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.origin.y))
    }
}

#[derive(Debug)]
pub struct RectWLens<T, U> {
    p: PhantomData<T>,
    u: PhantomData<U>,
}

impl<T, U> RectWLens<T, U> {
    pub fn new() -> Self {
        Self {
            p: PhantomData::default(),
            u: PhantomData::default(),
        }
    }
}

impl<T, U> Clone for RectWLens<T, U> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T, U> Copy for RectWLens<T, U> {}

impl<T: 'static, U: 'static> Lens for RectWLens<T, U> {
    type Source = Rect<T, U>;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.size.width))
    }
}

#[derive(Debug)]
pub struct RectHLens<T, U> {
    p: PhantomData<T>,
    u: PhantomData<U>,
}

impl<T, U> RectHLens<T, U> {
    pub fn new() -> Self {
        Self {
            p: PhantomData::default(),
            u: PhantomData::default(),
        }
    }
}

impl<T, U> Clone for RectHLens<T, U> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T, U> Copy for RectHLens<T, U> {}

impl<T: 'static, U: 'static> Lens for RectHLens<T, U> {
    type Source = Rect<T, U>;
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(Some(&source.size.height))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StylegroundNameLens {}

impl Lens for StylegroundNameLens {
    type Source = CelesteMapStyleground;
    type Target = String;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        if source.name == "parallax" {
            map(Some(&source.attributes.get("texture").map_or(
                "".to_owned(),
                |t| match t {
                    Attribute::Bool(b) => b.to_string(),
                    Attribute::Int(i) => i.to_string(),
                    Attribute::Float(f) => f.to_string(),
                    Attribute::Text(s) => s.to_owned(),
                },
            )))
        } else {
            map(Some(&source.name))
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentStylegroundLens {}

impl Lens for CurrentStylegroundLens {
    type Source = AppState;
    type Target = StylegroundSelection;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let data = match source.tabs.get(source.current_tab) {
            Some(AppTab::Map(maptab)) => maptab.styleground_selected.as_ref(),
            _ => None,
        };

        map(data)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentStylegroundImplLens {}

impl Lens for CurrentStylegroundImplLens {
    type Source = AppState;
    type Target = CelesteMapStyleground;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        let maptab = match source.tabs.get(source.current_tab) {
            Some(AppTab::Map(maptab)) => maptab,
            _ => return map(None),
        };
        let stysel = match &maptab.styleground_selected {
            Some(stysel) => stysel,
            None => return map(None),
        };
        let mapstate = match source.loaded_maps.get(&maptab.id) {
            Some(map) => map,
            None => return map(None),
        };
        map(mapstate.styles(stysel.fg).get(stysel.idx))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentTabImplLens {}

impl Lens for CurrentTabImplLens {
    type Source = AppState;
    type Target = AppTab;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        map(source.tabs.get(source.current_tab))
    }
}

pub struct StaticerLens<T: 'static> {
    data: T,
}

impl<T: Clone> Clone for StaticerLens<T> {
    fn clone(&self) -> Self {
        StaticerLens {
            data: self.data.clone(),
        }
    }
}

impl<T: Copy> Copy for StaticerLens<T> {}

impl<T: Clone> Lens for StaticerLens<T> {
    type Source = ();
    type Target = T;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, _: &Self::Source, map: F) -> O {
        map(Some(&self.data))
    }
}

impl<T> StaticerLens<T> {
    pub fn new(data: T) -> Self {
        StaticerLens { data }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TabTextLens(pub usize);

impl Lens for TabTextLens {
    type Source = AppState;
    type Target = String;

    fn view<O, F: FnOnce(Option<&Self::Target>) -> O>(&self, source: &Self::Source, map: F) -> O {
        if let Some(tab) = source.tabs.get(self.0) {
            map(Some(&match tab {
                AppTab::CelesteOverview => "All Mods".to_owned(),
                AppTab::ProjectOverview(s) => source
                    .modules
                    .get(s)
                    .unwrap()
                    .everest_metadata
                    .name
                    .to_string(),
                AppTab::Map(maptab) => {
                    let mut name = source
                        .loaded_maps
                        .get(&maptab.id)
                        .unwrap()
                        .cache
                        .path
                        .sid
                        .clone();
                    if source.loaded_maps.get(&maptab.id).unwrap().cache.dirty {
                        name.insert(0, '*');
                    }
                    name
                }
                AppTab::ConfigEditor(_) => "Config Editor".to_owned(),
                AppTab::Logs => "Logs".to_owned(),
                AppTab::MapMeta(id) => {
                    let mut name = source.loaded_maps.get(id).unwrap().cache.path.sid.clone();
                    if source.loaded_maps.get(id).unwrap().cache.dirty {
                        name.insert(0, '*');
                    }
                    name.push_str(" - Meta");
                    name
                }
            }))
        } else {
            map(None)
        }
    }
}
