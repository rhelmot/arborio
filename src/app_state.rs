use std::time;
use std::cell::RefCell;
use vizia::*;

use crate::widgets::editor_widget;
use crate::map_struct;
use crate::map_struct::{CelesteMap, CelesteMapDecal, CelesteMapEntity};
use crate::widgets::palette_widget::{DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable};
use crate::tools::Tool;
use crate::tools;
use crate::units::*;
use crate::assets;

#[derive(Lens)]
pub struct AppState {
    pub current_tool: usize,
    pub current_room: usize,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_bg_tile: TileSelectable,
    pub current_entity: EntitySelectable,
    pub current_trigger: TriggerSelectable,
    pub current_decal: DecalSelectable,
    pub current_selected: Option<AppSelection>,

    pub map: Option<map_struct::CelesteMap>,
    pub dirty: bool,
    pub transform: MapToScreen,

    pub draw_interval: f32,
    pub snap: bool,

    pub last_draw: RefCell<time::Instant>, // mutable to draw
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, enum_iterator::IntoEnumIterator)]
pub enum Layer {
    FgTiles,
    BgTiles,
    FgDecals,
    BgDecals,
    Entities,
    Triggers,
    ObjectTiles,
    All,
}

impl Data for Layer {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Layer {
    pub fn name(&self) -> &'static str {
        match self {
            Layer::FgTiles => "Foreground Tiles",
            Layer::BgTiles => "Background Tiles",
            Layer::Entities => "Entities",
            Layer::Triggers => "Triggers",
            Layer::FgDecals => "Foreground Decals",
            Layer::BgDecals => "Background Decals",
            Layer::ObjectTiles => "Object Tiles",
            Layer::All => "All Layers",
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    EntityBody(i32, bool),
    EntityNode(i32, usize, bool),
    Decal(u32, bool),
}

#[derive(Debug)]
pub enum AppEvent {
    Load { map: RefCell<Option<CelesteMap>> },
    Pan { delta: MapVectorPrecise },
    Zoom { delta: f32, focus: MapPointPrecise },
    SelectTool { idx: usize },
    SelectRoom { idx: usize },
    SelectLayer { layer: Layer },
    SelectPaletteTile { fg: bool, tile: TileSelectable },
    SelectPaletteEntity { entity: EntitySelectable },
    SelectPaletteTrigger { trigger: TriggerSelectable },
    SelectPaletteDecal { decal: DecalSelectable },
    SelectObject { selection: Option<AppSelection> },
    TileUpdate { fg: bool, offset: TilePoint, data: TileGrid<char> },
    EntityAdd { entity: CelesteMapEntity, trigger: bool },
    EntityUpdate { entity: CelesteMapEntity, trigger: bool },
    EntityRemove { id: i32, trigger: bool },
    DecalAdd { fg: bool, decal: CelesteMapDecal },
    DecalUpdate { fg: bool, decal: CelesteMapDecal },
    DecalRemove { fg: bool, id: u32 },
}


impl Model for AppState {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(app_event) = event.message.downcast() {
            self.apply(app_event);
        }
    }
}

impl AppState {
    pub fn new() -> AppState {
        let res = AppState {
            current_tool: 2,
            map: None,
            current_room: 0,
            current_fg_tile: TileSelectable::default(),
            current_bg_tile: TileSelectable::default(),
            current_entity: assets::ENTITIES_PALETTE[0],
            current_trigger: assets::TRIGGERS_PALETTE[0],
            current_decal: assets::DECALS_PALETTE[0],
            current_selected: None,
            dirty: false,
            transform: MapToScreen::identity(),
            draw_interval: 4.0,
            snap: true,
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::FgTiles,
        };
        res
    }

    pub fn apply(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Pan { delta } => {
                self.transform = self.transform.pre_translate(*delta);
            }
            AppEvent::Zoom { delta, focus } => {
                // TODO scale stepping, high and low limits
                self.transform = self.transform
                    .pre_translate(focus.to_vector())
                    .pre_scale(*delta, *delta)
                    .pre_translate(-focus.to_vector());
            }
            AppEvent::Load { map } => {
                let mut swapped: Option<CelesteMap> = None;
                std::mem::swap(&mut *map.borrow_mut(), &mut swapped);

                if swapped.is_some() {
                    self.map = swapped;
                    self.transform = MapToScreen::identity();
                }
            }
            AppEvent::TileUpdate { fg, offset, data } => {
                self.apply_tiles(offset, data, *fg);
            }
            AppEvent::SelectTool { idx } => {
                self.current_tool = *idx;
            }
            AppEvent::SelectRoom { idx } => {
                self.current_room = *idx;
            }
            AppEvent::SelectLayer { layer } => {
                self.current_layer = *layer;
            }
            AppEvent::SelectPaletteTile { fg, tile } => {
                if *fg {
                    self.current_fg_tile = *tile;
                } else {
                    self.current_bg_tile = *tile;
                }
            }
            AppEvent::SelectPaletteEntity { entity } => {
                self.current_entity = *entity;
            }
            AppEvent::SelectPaletteTrigger { trigger } => {
                self.current_trigger = *trigger;
            }
            AppEvent::SelectPaletteDecal { decal } => {
                self.current_decal = *decal;
            }
            AppEvent::EntityAdd { entity, trigger } => {
                dbg!(&event);
                if let Some(room) = self.current_room_mut() {
                    let mut entity = entity.clone();
                    entity.id = room.next_id();
                    if *trigger {
                        room.triggers.push(entity);
                    } else {
                        room.entities.push(entity)
                    }
                    room.cache.borrow_mut().render_cache_valid = false;
                    self.dirty = true;
                }
            }
            AppEvent::EntityUpdate { entity, trigger } => {
                if let Some(room) = self.current_room_mut() {
                    if let Some(mut e) = room.entity_mut(entity.id, *trigger) {
                        *e = entity.clone();
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.dirty = true;
                    }
                }
            }
            AppEvent::EntityRemove { id, trigger } => {
                if let Some(room) = self.current_room_mut() {
                    // tfw drain_filter is unstable
                    let mut i = 0;
                    let mut any = false;
                    let mut entities = if *trigger { &mut room.triggers } else { &mut room.entities };
                    while i < entities.len() {
                        if entities[i].id == *id {
                            entities.remove(i);
                            any = true;
                        } else {
                            i += 1;
                        }
                    }
                    if any {
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.dirty = true;
                    }
                }
            }
            AppEvent::SelectObject { selection } => {
                self.current_selected = *selection;
                if let Some(room) = self.current_room_ref() {
                    room.cache.borrow_mut().render_cache_valid = false;
                }
            }
            AppEvent::DecalAdd { fg, decal } => {
                if let Some(room) = self.current_room_mut() {
                    let mut decal = decal.clone();
                    let decals = if *fg { &mut room.fg_decals } else { &mut room.bg_decals };
                    decal.id = crate::map_struct::next_uuid();
                    decals.push(decal);
                    room.cache.borrow_mut().render_cache_valid = false;
                    self.dirty = true;
                }
            }
            AppEvent::DecalUpdate { fg, decal } => {
                if let Some(room) = self.current_room_mut() {
                    if let Some(decal_dest) = room.decal_mut(decal.id, *fg) {
                        *decal_dest = decal.clone();
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.dirty = true;
                    }
                }
            }
            AppEvent::DecalRemove { fg, id } => {
                if let Some(room) = self.current_room_mut() {
                    // tfw drain_filter is unstable
                    let mut i = 0;
                    let mut any = false;
                    let decals = if *fg { &mut room.fg_decals } else { &mut room.bg_decals };
                    while i < decals.len() {
                        if decals[i].id == *id {
                            decals.remove(i);
                            any = true;
                        } else {
                            i += 1;
                        }
                    }
                    if any {
                        room.cache.borrow_mut().render_cache_valid = false;
                        self.dirty = true;
                    }
                }
            }
        }
    }

    pub fn apply_tiles(&mut self, offset: &TilePoint, data: &TileGrid<char>, fg: bool) {
        let mut dirty = false;
        if let Some(map) = &mut self.map {
            if let Some(mut room) = map.levels.get_mut(self.current_room) {
                let mut line_start = *offset;
                let mut cur = line_start;
                for (idx, tile) in data.tiles.iter().enumerate() {
                    if *tile != '\0' {
                        if let Some(tile_ref) = room.tile_mut(cur, fg) {
                            if *tile_ref != *tile {
                                *tile_ref = *tile;
                                dirty = true;
                            }
                        }
                    }
                    if (idx + 1) % data.stride == 0 {
                        line_start += TileVector::new(0, 1);
                        cur = line_start;
                    } else {
                        cur += TileVector::new(1, 0);
                    }
                }
                if dirty {
                    room.cache.borrow_mut().render_cache_valid = false;
                    self.dirty = true;
                }
            }
        }
    }

    pub fn current_room_ref(&self) -> Option<&map_struct::CelesteMapLevel> {
        self.map.as_ref().and_then(|map| map.levels.get(self.current_room))
    }

    pub fn current_room_mut(&mut self) -> Option<&mut map_struct::CelesteMapLevel> {
        if let Some(map) = &mut self.map {
            return map.levels.get_mut(self.current_room);
        }
        None
    }
}
