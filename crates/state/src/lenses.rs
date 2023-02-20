use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::state::StatelessLens;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

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
use arborio_modloader::config::{AttributeInfo, EntityConfigV2};

pub fn current_map_lens() -> impl Lens<Source = AppState, Target = MapID> {
    ClosureLens::new(|state: &AppState| {
        let AppTab::Map(map) = state.tabs.get(state.current_tab)? else { return None };
        Some(&map.id)
    })
}

pub struct ClosureLens<T: 'static, U: 'static, F: (Fn(&T) -> Option<&U>) + Clone + 'static>(
    F,
    PhantomData<fn(&T) -> &U>,
);

impl<T, U, F> Clone for ClosureLens<T, U, F>
where
    F: Fn(&T) -> Option<&U> + Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<T, U, F> Copy for ClosureLens<T, U, F> where F: Fn(&T) -> Option<&U> + Copy {}
impl<T, U, F> ClosureLens<T, U, F>
where
    F: Fn(&T) -> Option<&U> + Clone,
{
    pub const fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<T, U, F> Lens for ClosureLens<T, U, F>
where
    F: (Fn(&T) -> Option<&U>) + Clone + 'static,
{
    type Source = T;
    type Target = U;
    fn view<'a>(&self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        self.view_stateless(source)
    }
}

impl<T, U, F> StatelessLens for ClosureLens<T, U, F>
where
    F: (Fn(&T) -> Option<&U>) + Clone + 'static,
{
    fn view_stateless<'a>(&self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(LensValue::Borrowed((self.0)(source)?))
    }
}

pub fn current_map_impl_lens() -> impl Lens<Source = AppState, Target = MapStateData> + Copy {
    ClosureLens::new(|source: &AppState| {
        let (AppTab::Map(MapTab{id, ..}) | AppTab::MapMeta(id)) = source.tabs.get(source.current_tab)? else { return None };
        Some(&source.loaded_maps.get(id).unwrap().data)
    })
}

pub fn current_room_lens() -> impl Lens<Source = AppState, Target = CelesteMapLevel> + Copy {
    ClosureLens::new(|source: &AppState| {
        let AppTab::Map(maptab) = source.tabs.get(source.current_tab)? else { return None };

        Some(
            &source
                .loaded_maps
                .get(&maptab.id)?
                .data
                .levels
                .get(maptab.current_room)?
                .data,
        )
    })
}

pub fn current_selected_entity_lens(
) -> impl Lens<Source = AppState, Target = CelesteMapEntity> + Copy {
    ClosureLens::new(|source: &AppState| {
        let AppTab::Map(MapTab {
            id: map_id,
            current_room,
            current_selected,
            ..
        }) = source.tabs.get(source.current_tab)? else { return None };
        let which_guys = current_selected
            .iter()
            .filter_map(|sel| match sel {
                AppSelection::EntityBody(id, trigger) => Some((*id, *trigger)),
                AppSelection::EntityNode(id, _, trigger) => Some((*id, *trigger)),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let mut ii = which_guys.iter();
        let first = ii.next();
        let second = ii.next();
        let (Some((entity_id, trigger)), None) = (first, second) else { return None };

        source
            .loaded_maps
            .get(map_id)?
            .data
            .levels
            .get(*current_room)?
            .entity(*entity_id, *trigger)
    })
}

#[derive(Clone)]
pub enum CurrentSelectedEntitiesAllLens<T> {
    F1(Arc<dyn 'static + Send + Sync + Fn(&AppState, &CelesteMapEntity) -> Option<T>>),
    F2(
        Arc<
            dyn 'static
                + Send
                + Sync
                + for<'a> Fn(&'a AppState, &'a CelesteMapEntity) -> Option<&'a T>,
        >,
    ),
    //L(Lens<Source=CelesteMapEntity, Target=T>>),
}

impl<T: 'static + PartialEq + Clone> CurrentSelectedEntitiesAllLens<T> {
    pub fn new_computed<
        F: 'static + Send + Sync + Clone + Fn(&AppState, &CelesteMapEntity) -> Option<T>,
    >(
        f: F,
    ) -> Self {
        Self::F1(Arc::new(f))
    }

    pub fn new_referenced<
        F: 'static
            + Send
            + Sync
            + Clone
            + for<'a> Fn(&'a AppState, &'a CelesteMapEntity) -> Option<&'a T>,
    >(
        f: F,
    ) -> Self {
        Self::F2(Arc::new(f))
    }

    // pub fn new_lensed<L: Lens<Source=CelesteMapEntity, Target=T>>(l: L) -> Self {
    //     Self::L(Rc::new(l))
    // }
}

impl<T: 'static + PartialEq + Clone> Lens for CurrentSelectedEntitiesAllLens<T> {
    type Source = AppState;
    type Target = T;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let AppTab::Map(MapTab {
                                 id: map_id,
                                 current_room,
                                 current_selected,
                                 ..
                             }) = source.tabs.get(source.current_tab)? else { return None };
        let room = source
            .loaded_maps
            .get(map_id)
            .and_then(|map| map.data.levels.get(*current_room))?;

        let mut ent_iter = current_selected
            .iter()
            .filter_map(AppSelection::entity_info)
            .filter_map(|(eid, trigger)| room.entity(eid, trigger));
        let ent_first = ent_iter.next()?;
        match self {
            CurrentSelectedEntitiesAllLens::F1(f) => {
                let res_first = f(source, ent_first);
                if ent_iter.all(|ent| res_first == f(source, ent)) {
                    Some(res_first?.into())
                } else {
                    None
                }
            }
            CurrentSelectedEntitiesAllLens::F2(f) => {
                let res_first = f(source, ent_first)?;
                if ent_iter.all(|ent| Some(res_first) == f(source, ent)) {
                    Some(res_first.into())
                } else {
                    None
                }
            }
        }
    }
}

pub enum CurrentSelectedEntitiesAttributesLens<T> {
    F1(Arc<dyn 'static + Send + Sync + Fn(&AppState, &HashSet<&String>) -> Option<T>>),
    F2(
        Arc<
            dyn 'static
                + Send
                + Sync
                + for<'a> Fn(&'a AppState, &HashSet<&'a String>) -> Option<&'a T>,
        >,
    ),
}

impl<T> Clone for CurrentSelectedEntitiesAttributesLens<T> {
    fn clone(&self) -> Self {
        match self {
            Self::F1(f) => Self::F1(f.clone()),
            Self::F2(f) => Self::F2(f.clone()),
        }
    }
}

impl<T: 'static> CurrentSelectedEntitiesAttributesLens<T> {
    pub fn new_computed<
        F: 'static + Send + Sync + Fn(&AppState, &HashSet<&String>) -> Option<T>,
    >(
        f: F,
    ) -> Self {
        Self::F1(Arc::new(f))
    }

    pub fn new_referenced<
        F: 'static + Send + Sync + for<'a> Fn(&'a AppState, &HashSet<&'a String>) -> Option<&'a T>,
    >(
        f: F,
    ) -> Self {
        Self::F2(Arc::new(f))
    }
}

impl<T: 'static> Lens for CurrentSelectedEntitiesAttributesLens<T> {
    type Source = AppState;
    type Target = T;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let tab = source.tabs.get(source.current_tab)?;
        let AppTab::Map(MapTab {
            id: map_id,
            current_room,
            current_selected,
            ..
        }) = tab else { return None };

        let cmap = source.loaded_maps.get(map_id)?;
        let room = cmap.data.levels.get(*current_room)?;
        let mut counter = HashSet::new();

        for sel in current_selected {
            if let AppSelection::EntityBody(entity_id, trigger)
            | AppSelection::EntityNode(entity_id, _, trigger) = sel
            {
                if let Some(entity) = room.entity(*entity_id, *trigger) {
                    for key in entity.attributes.keys() {
                        counter.insert(key);
                    }
                }
            }
        }

        Some(match self {
            CurrentSelectedEntitiesAttributesLens::F1(f) => LensValue::Owned(f(source, &counter)?),
            CurrentSelectedEntitiesAttributesLens::F2(f) => {
                LensValue::Borrowed(f(source, &counter)?)
            }
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityResizableLens {}

impl Lens for CurrentSelectedEntityResizableLens {
    type Source = AppState;
    type Target = (bool, bool);

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let tab = source.tabs.get(source.current_tab)?;
        let AppTab::Map(MapTab {
            id: map_id,
            current_room,
            current_selected,
            ..
        }) = tab else { return None };
        let cmap = source.loaded_maps.get(map_id)?;
        let room = cmap.data.levels.get(*current_room)?;
        let mut result = (true, true);
        for sel in current_selected {
            if let AppSelection::EntityBody(entity_id, trigger)
            | AppSelection::EntityNode(entity_id, _, trigger) = sel
            {
                let name = room.entity(*entity_id, *trigger)?.name.as_str();
                if !*trigger {
                    let cfg = cmap.cache.palette.entity_config.get(name)?;
                    let EntityConfigV2 {
                        resizable_x: dx,
                        resizable_y: dy,
                        ..
                    } = cfg.as_ref();
                    result.0 &= dx;
                    result.1 &= dy;
                }
            }
        }
        Some(result.into())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CurrentSelectedEntityHasNodesLens {}

impl Lens for CurrentSelectedEntityHasNodesLens {
    type Source = AppState;
    type Target = bool;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let tab = source.tabs.get(source.current_tab)?;
        let AppTab::Map(MapTab {
                id: map_id,
                current_room,
                current_selected,
                ..
            }) = tab else { return None };
        let cmap = source.loaded_maps.get(map_id)?;
        let room = cmap.data.levels.get(*current_room)?;
        let mut result = true;
        for sel in current_selected {
            if let AppSelection::EntityBody(entity_id, trigger)
            | AppSelection::EntityNode(entity_id, _, trigger) = sel
            {
                let name = room.entity(*entity_id, *trigger)?.name.as_str();
                let d = if !*trigger {
                    cmap.cache.palette.entity_config.get(name)?.nodes
                } else {
                    cmap.cache.palette.trigger_config.get(name)?.nodes
                };
                result &= d;
            }
        }
        Some(LensValue::Owned(result))
    }
}

pub fn current_palette_lens(
) -> impl StatelessLens<Source = AppState, Target = ModuleAggregate> + 'static {
    ClosureLens::new(|source: &AppState| {
        let tab = source.tabs.get(source.current_tab)?;
        let AppTab::Map(MapTab { id: map_id, .. }) = tab else {
            return None;
        };

        Some(&source.loaded_maps.get(map_id)?.cache.palette)
    })
}

#[derive(Debug, Copy, Clone)]
pub struct CurrentSelectedEntityConfigAttributesLens {}

impl Lens for CurrentSelectedEntityConfigAttributesLens {
    type Source = AppState;
    type Target = HashMap<String, AttributeInfo>;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let lens = CurrentSelectedEntitiesAllLens::new_referenced(|_, ent| Some(&ent.name));
        let entity = lens.view(source)?;
        let palette = current_palette_lens().view_stateless(source)?;
        match palette {
            LensValue::Borrowed(palette) => {
                // TODO UGHHHHHHHHHHH have to do something complex to get each trigger status
                // maybe easy now??
                Some(
                    if let Some(cfg) = palette.entity_config.get(entity.as_str()) {
                        &cfg.attribute_info
                    } else {
                        let cfg = palette.trigger_config.get(entity.as_str())?;
                        &cfg.attribute_info
                    }
                    .into(),
                )
            }
            LensValue::Owned(_) => todo!(),
        }
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
        Self { t: PhantomData }
    }
}

impl<T> Default for AutoSaverLens<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Copy for AutoSaverLens<T> {}

impl<T: 'static + Debug> Lens for AutoSaverLens<T> {
    type Source = AutoSaver<T>;
    type Target = T;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(source.deref().into())
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
        Self { p: PhantomData }
    }
}

impl<T> Copy for VecLenLens<T> {}

impl<T: 'static> Lens for VecLenLens<T> {
    type Source = Vec<T>;
    type Target = usize;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(source.len().into())
    }
}

#[derive(Debug)]
pub struct HashSetLenLens<T> {
    p: PhantomData<T>,
}

impl<T> Clone for HashSetLenLens<T> {
    fn clone(&self) -> Self {
        Self { p: PhantomData }
    }
}

impl<T> Copy for HashSetLenLens<T> {}

impl<T> HashSetLenLens<T> {
    pub fn new() -> Self {
        Self { p: PhantomData }
    }
}

impl<T> Default for HashSetLenLens<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Lens for HashSetLenLens<T> {
    type Source = HashSet<T>;
    type Target = usize;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(source.len().into())
    }
}

#[derive(Debug)]
pub struct HashMapLenLens<K, V> {
    p: PhantomData<(K, V)>,
}

impl<K, V> HashMapLenLens<K, V> {
    pub fn new() -> Self {
        Self { p: PhantomData }
    }
}

impl<K, V> Default for HashMapLenLens<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Clone for HashMapLenLens<K, V> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<K, V> Copy for HashMapLenLens<K, V> {}

impl<K: 'static, V: 'static> Lens for HashMapLenLens<K, V> {
    type Source = HashMap<K, V>;
    type Target = usize;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(source.len().into())
    }
}

pub fn hash_map_nth_key_lens<K: Ord + 'static, V: 'static>(
    idx: usize,
) -> impl Lens<Source = HashMap<K, V>, Target = K> + Copy + Send + Sync {
    ClosureLens::new(move |source: &HashMap<K, V>| {
        let mut keys = source.keys().collect::<Vec<_>>();
        keys.sort();
        keys.get(idx).copied()
    })
}

pub fn hash_set_nth_key_lens<K: Ord + 'static>(
    idx: usize,
) -> impl Lens<Source = HashSet<K>, Target = K> + Copy {
    ClosureLens::new(move |source: &HashSet<K>| {
        let mut keys = source.iter().collect::<Vec<_>>();
        keys.sort();
        keys.get(idx).copied()
    })
}

#[derive(Debug, Clone, Copy)]
pub struct VecIndexWithLens<L1, L2> {
    l1: L1,
    l2: L2,
}

impl<L1, L2> VecIndexWithLens<L1, L2> {
    pub fn new(l1: L1, l2: L2) -> Self {
        Self { l1, l2 }
    }
}

impl<L1, L2, T: 'static + Debug> Lens for VecIndexWithLens<L1, L2>
where
    L1: StatelessLens<Target = Vec<T>>,
    L2: Lens<Source = <L1 as Lens>::Source, Target = usize>,
{
    type Source = <L1 as Lens>::Source;
    type Target = T;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let index = self.l2.view(source)?;
        let vec = self.l1.view_stateless(source)?;
        match vec {
            LensValue::Borrowed(vec) => Some(vec.get(*index)?.into()),
            LensValue::Owned(mut vec) => {
                if vec.len() >= *index {
                    None
                } else {
                    Some(vec.swap_remove(*index).into())
                }
            }
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
            t: PhantomData,
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

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let idx = self.l2.view(source)?;
        let hashmap = self.l1.view(source)?;
        let LensValue::Borrowed(hashmap) = hashmap else { panic!() };
        Some(hashmap.get(&idx)?.into())
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

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(self.lens.view(source).is_none().into())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RoomTweakerScopeLens {}

impl Lens for RoomTweakerScopeLens {
    type Source = AppState;
    type Target = (MapID, usize);

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let AppTab::Map(maptab) = source.tabs.get(source.current_tab)? else { return None };
        Some((maptab.id, maptab.current_room).into())
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

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some(
            (
                self.one.view(source)?.into_owned(),
                self.two.view(source)?.into_owned(),
            )
                .into(),
        )
    }
}

pub fn rect_x_lens<T: 'static, U: 'static>() -> impl Lens<Source = Rect<T, U>, Target = T> {
    ClosureLens::new(|source: &Rect<T, U>| Some(&source.origin.x))
}
pub fn rect_y_lens<T: 'static, U: 'static>() -> impl Lens<Source = Rect<T, U>, Target = T> {
    ClosureLens::new(|source: &Rect<T, U>| Some(&source.origin.y))
}
pub fn rect_w_lens<T: 'static, U: 'static>() -> impl Lens<Source = Rect<T, U>, Target = T> {
    ClosureLens::new(|source: &Rect<T, U>| Some(&source.size.width))
}
pub fn rect_h_lens<T: 'static, U: 'static>() -> impl Lens<Source = Rect<T, U>, Target = T> {
    ClosureLens::new(|source: &Rect<T, U>| Some(&source.size.height))
}
#[derive(Copy, Clone, Debug)]
pub struct StylegroundNameLens {}

impl Lens for StylegroundNameLens {
    type Source = CelesteMapStyleground;
    type Target = String;

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        const EMPTY: &String = &String::new();
        Some(if source.name == "parallax" {
            source
                .attributes
                .get("texture")
                .map_or(LensValue::Borrowed(EMPTY), |t| match t {
                    Attribute::Bool(b) => b.to_string().into(),
                    Attribute::Int(i) => i.to_string().into(),
                    Attribute::Float(f) => f.to_string().into(),
                    Attribute::Text(s) => s.into(),
                })
        } else {
            LensValue::Borrowed(&source.name)
        })
    }
}

pub fn current_styleground_lens() -> impl Lens<Source = AppState, Target = StylegroundSelection> {
    ClosureLens::new(|source: &AppState| {
        let Some(AppTab::Map(maptab)) = source.tabs.get(source.current_tab) else { return None };
        maptab.styleground_selected.as_ref()
    })
}

pub fn current_styleground_impl_lens(
) -> impl Lens<Source = AppState, Target = CelesteMapStyleground> + Copy {
    ClosureLens::new(|source: &AppState| {
        let AppTab::Map(MapTab { id, styleground_selected: Some(stysel),   ..}) = source.tabs.get(source.current_tab)? else { return None };
        source
            .loaded_maps
            .get(id)?
            .styles(stysel.fg)
            .get(stysel.idx)
    })
}

pub fn current_tab_impl_lens() -> impl Lens<Source = AppState, Target = AppTab> + Copy {
    ClosureLens::new(|source: &AppState| source.tabs.get(source.current_tab))
}

#[derive(Copy, Clone)]
pub struct StaticerLens<T: 'static> {
    data: T,
}

impl<T: Clone> Lens for StaticerLens<T> {
    type Source = ();
    type Target = T;

    fn view<'a>(&'a self, (): &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        Some((&self.data).into())
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

    fn view<'a>(&'a self, source: &'a Self::Source) -> Option<LensValue<'a, Self::Target>> {
        let tab = source.tabs.get(self.0)?;
        Some(match tab {
            AppTab::CelesteOverview => "All Mods".to_owned().into(),
            AppTab::ProjectOverview(s) => {
                (&source.modules.get(s).unwrap().everest_metadata.name).into()
            }
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
                name.into()
            }
            AppTab::ConfigEditor(_) => "Config Editor".to_owned().into(),
            AppTab::Logs => "Logs".to_owned().into(),
            AppTab::MapMeta(id) => {
                let mut name = source.loaded_maps.get(id).unwrap().cache.path.sid.clone();
                if source.loaded_maps.get(id).unwrap().cache.dirty {
                    name.insert(0, '*');
                }
                name.push_str(" - Meta");
                name.into()
            }
        })
    }
}
