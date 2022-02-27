use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use vizia::*;

use crate::app_state::{AppSelection, AppState, AppTab, MapTab};
use crate::auto_saver::AutoSaver;
use crate::map_struct::{CelesteMapEntity, CelesteMapLevel, MapID};
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

        map(the_map.levels.get(maptab.current_room))
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

        let data = source.palettes.get(map_id);
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

        map(Some(&(maptab.id.clone(), maptab.current_room)))
    }
}
