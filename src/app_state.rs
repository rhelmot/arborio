use std::time;
use std::cell::RefCell;
use vizia::*;

use crate::editor_widget;
use crate::map_struct;
use crate::map_struct::CelesteMap;
use crate::tools::Tool;
use crate::tools;
use crate::units::*;

#[derive(Lens)]
pub struct AppState {
    pub tools: RefCell<Vec<Box<dyn Tool>>>, // mutable to event
    pub current_tool: usize,
    pub current_room: usize,
    pub current_layer: Layer,
    pub map: Option<map_struct::CelesteMap>,
    pub dirty: bool,
    pub transform: MapToScreen,
    pub last_draw: RefCell<time::Instant>, // mutable to draw
}

#[derive(Debug, Copy, Clone)]
pub enum Layer {
    Tiles(bool),
    Decals(bool),
    Entities,
    Triggers,
    ObjectTiles,
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
    FgTileUpdate { offset: TilePoint, data: TileFloat },
    BgTileUpdate { offset: TilePoint, data: TileFloat },
}


impl Model for AppState {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(app_event) = event.message.downcast() {
            self.apply(app_event);
        }
    }
}

pub const NUM_TOOLS: usize = 2;

impl AppState {
    pub fn new() -> AppState {
        let res = AppState {
            tools: RefCell::new(vec![
                Box::new(tools::hand::HandTool::default()),
                Box::new(tools::pencil::PencilTool::default()),
            ]),
            current_tool: 1,
            map: None,
            current_room: 0,
            dirty: false,
            transform: MapToScreen::identity(),
            last_draw: RefCell::new(time::Instant::now()),
            current_layer: Layer::Tiles(true)
        };
        assert_eq!(res.tools.borrow().len(), NUM_TOOLS);
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
            AppEvent::FgTileUpdate { offset, data } => {
                self.apply_tiles(offset, data, true);
            }
            AppEvent::BgTileUpdate { offset, data } => {
                self.apply_tiles(offset, data, false);
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