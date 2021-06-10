use fltk::{prelude::*,*,enums::Key};

use crate::map_struct;
use crate::atlas_img::{Atlas, SpriteReference};
use crate::autotiler::Tileset;
use crate::assets;

use std::cell::RefCell;
use std::rc::Rc;
use std::cmp::{min, max};
use std::collections::HashMap;
use std::env;
use std::time;
use lazy_static::lazy_static;

lazy_static! {
    static ref PERF_MONITOR: bool = {
        env::var("ARBORIO_PERF_MONITOR").is_ok()
    };
}

fn backdrop_color() -> enums::Color    { enums::Color::from_u32(0x103010) }
fn room_empty_color() -> enums::Color  { enums::Color::from_u32(0x204020) }
fn room_fg_color() -> enums::Color     { enums::Color::from_u32(0x3060f0) }
fn room_bg_color() -> enums::Color     { enums::Color::from_u32(0x101040) }
fn room_entity_color() -> enums::Color { enums::Color::from_u32(0xff0000) }

pub struct EditorWidget {
    state: Rc<RefCell<EditorState>>,
    pub widget: widget::Widget,
}

struct EditorState {
    map: Option<map_struct::CelesteMap>,
    current_room: usize,
    map_corner_x: i32,
    map_corner_y: i32,
    map_scale: u32, // screen pixels per game tile
    last_draw: time::Instant,
}

impl EditorWidget {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> EditorWidget {
        let state = EditorState {
            map: None,
            current_room: 0,
            map_corner_x: 0,
            map_corner_y: 0,
            map_scale: 8,
            last_draw: time::Instant::now(),
        };

        let mut result = EditorWidget {
            state: Rc::new(RefCell::new(state)),
            widget: widget::Widget::new(x, y, w, h, ""),
        };

        result.draw();
        result.handle();

        return result;
    }

    pub fn set_map(&mut self, map: map_struct::CelesteMap) {
        self.state.borrow_mut().map = Some(map);
        self.widget.redraw();
    }

    pub fn reset_view(&mut self) {
        let mut mutstate = self.state.borrow_mut();
        mutstate.map_scale = 8;
        mutstate.map_corner_x = 0;
        mutstate.map_corner_y = -30;
        self.widget.redraw();
    }

    fn draw(&mut self) {
        let state = self.state.clone();
        self.widget.draw(move |b| {
            let mut state = state.borrow_mut();
            if *PERF_MONITOR {
                let now = time::Instant::now();
                println!("Drew {}ms ago", (now - state.last_draw).as_millis());
                state.last_draw = now;
            }
            let screen = map_struct::Rect::from_widget(b);
            draw::push_clip(b.x(), b.y(), b.w(), b.h());
            draw::draw_rect_fill(b.x(), b.y(), b.w(), b.h(), backdrop_color());

            if state.map.is_some() {
                for filler in state.map.as_ref().unwrap().filler.iter() {
                    let filler_screen = state.rect_level_to_screen(filler);
                    if filler_screen.intersects(&screen) {
                        draw::draw_rect_fill(
                            filler_screen.x,
                            filler_screen.y,
                            filler_screen.width as i32,
                            filler_screen.height as i32,
                            enums::Color::from_u32(0x804000))
                    }
                }
                let mut resized_sprite_cache = assets::SPRITE_CACHE.lock().unwrap();
                if resized_sprite_cache.len() <= state.map_scale as usize {
                    resized_sprite_cache.resize_with(state.map_scale as usize + 1, HashMap::new);
                }
                let resized_sprite_cache = &mut resized_sprite_cache[state.map_scale as usize];
                for room_idx in 0..state.map.as_ref().unwrap().levels.len() {
                    let rect_screen = state.rect_level_to_screen(&state.map.as_ref().unwrap().levels[room_idx].bounds);
                    if rect_screen.intersects(&screen) {
                        let should_draw_complex = state.map_scale >= 8;
                        if should_draw_complex {
                            state.draw_room_backdrop(room_idx);
                            state.draw_room_bg_complex(room_idx, resized_sprite_cache);
                            state.draw_room_fg_complex(room_idx, resized_sprite_cache);
                        } else {
                            state.draw_room_backdrop(room_idx);
                            state.draw_room_bg_simple(room_idx);
                            state.draw_room_fg_simple(room_idx);
                        }
                    }
                }
            }
            draw::pop_clip();

            if *PERF_MONITOR {
                let now = time::Instant::now();
                println!("Draw took {}ms", (now - state.last_draw).as_millis());
            }
        });
    }

    fn handle(&mut self) {
        let state = self.state.clone();
        self.widget.handle(move |b, ev| {
            let mut state = state.borrow_mut();
            match ev {
                enums::Event::Enter => {
                    true
                },
                enums::Event::MouseWheel => {
                    let mouse_y = match app::event_dy() {
                        app::MouseWheel::Down => -1,
                        app::MouseWheel::Up => 1,
                        _ => 0,
                    };
                    let mouse_x: i32 = match app::event_dx() {
                        app::MouseWheel::Right => -1,
                        app::MouseWheel::Left => 1,
                        _ => 0,
                    };
                    let (screen_y, screen_x) = if app::event_key_down(Key::ShiftL) || app::event_key_down(Key::ShiftR) {
                        (mouse_x, mouse_y)
                    } else {
                        (mouse_y, mouse_x)
                    };
                    if app::event_key_down(Key::ControlL) || app::event_key_down(Key::ControlR) {
                        let (old_x, old_y) = state.point_screen_to_level(app::event_x(), app::event_y());
                        state.map_scale = max(min(30, state.map_scale as i32 - screen_y), 1) as u32;
                        let (new_x, new_y) = state.point_screen_to_level(app::event_x(), app::event_y());
                        state.map_corner_x += old_x - new_x;
                        state.map_corner_y += old_y - new_y;
                    } else {
                        let amt = state.size_screen_to_level(30);
                        state.map_corner_x += amt as i32 * screen_x;
                        state.map_corner_y += amt as i32 * screen_y;
                    }
                    b.redraw();
                    true
                },
                _ => false
            }
        });
    }
}

impl EditorState {
    fn draw_room_backdrop(&mut self, room_idx: usize) {
        if self.map.as_ref().is_none() {
            return;
        }

        let room = &self.map.as_ref().unwrap().levels[room_idx];
        let rect = self.rect_level_to_screen(&room.bounds);
        draw::draw_rect_fill(rect.x, rect.y, rect.width as i32, rect.height as i32, room_empty_color());
    }

    fn draw_room_fg_simple(&mut self, room_idx: usize) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let tstride = room.bounds.width / 8;
        let unit = self.size_level_to_screen(8);
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let (sx, sy) = self.point_level_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
                let fgtile = room.fg_tiles[(tx + ty * tstride) as usize];
                if fgtile != '0' {
                    draw::draw_rect_fill(sx, sy, unit as i32, unit as i32, room_fg_color());
                }
            }
        }
    }

    fn draw_room_bg_simple(&mut self, room_idx: usize) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let tstride = room.bounds.width / 8;
        let unit = self.size_level_to_screen(8);
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let (sx, sy) = self.point_level_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
                let bgtile = room.bg_tiles[(tx + ty * tstride) as usize];
                if bgtile != '0' {
                    draw::draw_rect_fill(sx, sy, unit as i32, unit as i32, room_bg_color());
                }
            }
        }
    }

    fn draw_room_fg_complex(&mut self, room_idx: usize, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let scale = self.map_scale as f32 / 8_f32;

        let tstride = room.bounds.width / 8;
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let fgtile = room.fg_tiles[(tx + ty * tstride) as usize];
                let (sx, sy) = self.point_level_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
                if fgtile != '0' {
                    if let Some(tileset) = assets::FG_TILES.get(&fgtile) {
                        if let Some(tile) = tileset.tile_fg(room, tx as i32, ty as i32) {
                            assets::GAMEPLAY_ATLAS.draw_tile(tile, sx, sy, scale, resized_sprite_cache);
                        }
                    }
                }
            }
        }
    }

    fn draw_room_bg_complex(&mut self, room_idx: usize, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let scale = self.map_scale as f32 / 8_f32;

        let tstride = room.bounds.width / 8;
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let bgtile = room.bg_tiles[(tx + ty * tstride) as usize];
                let (sx, sy) = self.point_level_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
                if bgtile != '0' {
                    if let Some(tileset) = assets::BG_TILES.get(&bgtile) {
                        if let Some(tile) = tileset.tile_bg(room, tx as i32, ty as i32) {
                            assets::GAMEPLAY_ATLAS.draw_tile(tile, sx, sy, scale, resized_sprite_cache);
                        }
                    }
                }
            }
        }
    }

    fn rect_level_to_screen(&self, rect: &map_struct::Rect) -> map_struct::Rect {
        let (x, y) = self.point_level_to_screen(rect.x, rect.y);
        map_struct::Rect {
            x, y,
            width: self.size_level_to_screen(rect.width),
            height: self.size_level_to_screen(rect.height),
        }
    }

    fn rect_screen_to_level(&self, rect: map_struct::Rect) -> map_struct::Rect {
        let (x, y) = self.point_screen_to_level(rect.x, rect.y);
        map_struct::Rect {
            x, y,
            width: self.size_screen_to_level(rect.width),
            height: self.size_screen_to_level(rect.height),
        }
    }

    fn size_level_to_screen(&self, size: u32) -> u32 {
        size * self.map_scale / 8
    }

    fn size_screen_to_level(&self, size: u32) -> u32 {
        size * 8 / self.map_scale
    }

    fn point_screen_to_level(&self, x: i32, y: i32) -> (i32, i32) {
        (x * 8 / self.map_scale as i32 + self.map_corner_x,
         y * 8 / self.map_scale as i32 + self.map_corner_y)
    }

    fn point_level_to_screen(&self, x: i32, y: i32) -> (i32, i32) {
        ((x - self.map_corner_x) * self.map_scale as i32 / 8,
         (y - self.map_corner_y) * self.map_scale as i32 / 8)
    }
}

impl map_struct::Rect {
    pub fn intersects(&self, other: &map_struct::Rect) -> bool {
        (   (self.x >= other.x && self.x < other.x + other.width as i32) ||
            (other.x >= self.x && other.x < self.x + self.width as i32)) && (
            (self.y >= other.y && self.y < other.y + other.height as i32) ||
            (other.y >= self.y && other.y < self.y + self.height as i32))
    }

    pub fn from_widget(wid: &widget::Widget) -> map_struct::Rect {
        map_struct::Rect {
            x: wid.x(),
            y: wid.y(),
            width: wid.w() as u32,
            height: wid.h() as u32,
        }
    }
}