use std::time;
use std::cell::RefCell;
use vizia::*;

use crate::editor_widget;
use crate::map_struct;
use crate::map_struct::CelesteMap;
use crate::palette_widget::TileSelectable;
use crate::tools::Tool;
use crate::tools;
use crate::units::*;

#[derive(Lens)]
pub struct AppState {
    pub current_tool: usize,
    pub current_room: usize,
    pub current_layer: Layer,
    pub current_fg_tile: TileSelectable,
    pub current_bg_tile: TileSelectable,
    pub map: Option<map_struct::CelesteMap>,
    pub dirty: bool,
    pub transform: MapToScreen,
    pub last_draw: RefCell<time::Instant>, // mutable to draw
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Layer {
    Tiles(bool),
    Decals(bool),
    Entities,
    Triggers,
    ObjectTiles,
}

impl Data for Layer {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Layer {
    pub fn to_idx(&self) -> usize {
        match self {
            Layer::Tiles(fg) => if *fg { 0 }  else { 1 },
            Layer::Entities => 2,
            Layer::Triggers => 3,
            Layer::Decals(fg) => if *fg { 4 } else { 5 },
            Layer::ObjectTiles => 6,
        }
    }

    pub fn from_idx(idx: usize) -> Layer {
        match idx {
            0 => Layer::Tiles(true),
            1 => Layer::Tiles(false),
            2 => Layer::Entities,
            3 => Layer::Triggers,
            4 => Layer::Decals(true),
            5 => Layer::Decals(false),
            6 => Layer::ObjectTiles,
            _ => panic!("Bad layer index")
        }
    }

    pub fn all_layers() -> impl Iterator<Item = Layer> {
        (0..=6).map(Layer::from_idx)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Layer::Tiles(fg) => if *fg { "Foreground Tiles" } else { "Background Tiles" },
            Layer::Entities => "Entities",
            Layer::Triggers => "Triggers",
            Layer::Decals(fg) => if *fg { "Foreground Decals" } else { "Background Decals" },
            Layer::ObjectTiles => "Object Tiles",
        }
    }
}

#[derive(Debug)]
pub struct TileFloat {
    pub tiles: Vec<char>,
    pub stride: usize,
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
    TileUpdate { fg: bool, offset: TilePoint, data: TileFloat },
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
            current_tool: 1,
            map: None,
            current_room: 0,
            current_fg_tile: TileSelectable::default(),
            current_bg_tile: TileSelectable::default(),
            dirty: false,
            transform: MapToScreen::identity(),
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::Tiles(true),
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
        }
    }

    pub fn apply_tiles(&mut self, offset: &TilePoint, data: &TileFloat, fg: bool) {
        let mut dirty = false;
        if let Some(map) = &mut self.map {
            if let Some(mut room) = map.levels.get_mut(self.current_room) {
                let mut line_start = *offset;
                let mut cur = line_start;
                for (idx, tile) in data.tiles.iter().enumerate() {
                    if *tile != '\0' {
                        if let Some(tile_ref) = room.tile_mut(cur, fg) {
                            if *tile_ref != *tile {
                                dirty = true;
                            }
                            *tile_ref = *tile;
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
}
