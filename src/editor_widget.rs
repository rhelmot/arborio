use fltk::{prelude::*,*,enums::Key};

use crate::map_struct;
use crate::atlas_img::SpriteReference;
use crate::assets;

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::time;
use std::env;
use lazy_static::lazy_static;
use crate::image_view::ImageBuffer;
use crate::map_struct::{Rect, CelesteMapEntity, CelesteMapLevel};
use crate::entity_config::{DrawElement, AutotilerType};
use crate::entity_expression::Const;

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
    transform: EditorTransform,
    last_draw: time::Instant,
    prev_mouse: (i32, i32),
}

struct EditorTransform {
    map_corner_x: i32,
    map_corner_y: i32,
    map_scale: u32, // screen pixels per game tile
}
impl EditorTransform {
    fn rect_map_to_screen(&self, rect: &map_struct::Rect) -> map_struct::Rect {
        let (x, y) = self.point_map_to_screen(rect.x, rect.y);
        map_struct::Rect {
            x, y,
            width: self.size_map_to_screen(rect.width),
            height: self.size_map_to_screen(rect.height),
        }
    }

    fn rect_screen_to_map(&self, rect: map_struct::Rect) -> map_struct::Rect {
        let (x, y) = self.point_screen_to_map(rect.x, rect.y);
        map_struct::Rect {
            x, y,
            width: self.size_screen_to_map(rect.width),
            height: self.size_screen_to_map(rect.height),
        }
    }

    fn size_map_to_screen(&self, size: u32) -> u32 {
        size * self.map_scale / 8
    }

    fn size_screen_to_map(&self, size: u32) -> u32 {
        size * 8 / self.map_scale
    }

    fn point_screen_to_map(&self, x: i32, y: i32) -> (i32, i32) {
        (x * 8 / self.map_scale as i32 + self.map_corner_x,
         y * 8 / self.map_scale as i32 + self.map_corner_y)
    }

    fn point_map_to_screen(&self, x: i32, y: i32) -> (i32, i32) {
        (((x - self.map_corner_x) * self.map_scale as i32).div_euclid(8),
         ((y - self.map_corner_y) * self.map_scale as i32).div_euclid(8))
    }
}

impl CelesteMapLevel {
    fn point_room_to_map(&self, x: i32, y: i32) -> (i32, i32) {
        (x + self.bounds.x, y + self.bounds.y)
    }

    fn rect_room_to_map(&self, rect: &map_struct::Rect) -> map_struct::Rect {
        let (x, y) = self.point_room_to_map(rect.x, rect.y);
        map_struct::Rect {
            x, y,
            width: rect.width,
            height: rect.height,
        }
    }
}

impl EditorWidget {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> EditorWidget {
        let state = EditorState {
            map: None,
            transform: EditorTransform {
                map_corner_x: 0,
                map_corner_y: 0,
                map_scale: 8,
            },
            last_draw: time::Instant::now(),
            prev_mouse: (0, 0),
        };

        let mut result = EditorWidget {
            state: Rc::new(RefCell::new(state)),
            widget: widget::Widget::new(x, y, w, h, ""),
        };

        result.draw();
        result.handle();

        result
    }

    pub fn set_map(&mut self, map: map_struct::CelesteMap) {
        self.state.borrow_mut().map = Some(map);
        self.widget.redraw();
    }

    pub fn reset_view(&mut self) {
        let mut mutstate = self.state.borrow_mut();
        let position = &mut mutstate.transform;
        position.map_scale = 8;
        position.map_corner_x = 0;
        position.map_corner_y = -30;
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

            if let Some(ref map) = state.map {
                for filler in &map.filler {
                    let filler_screen = state.transform.rect_map_to_screen(filler);
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
                if resized_sprite_cache.len() <= state.transform.map_scale as usize {
                    resized_sprite_cache.resize_with(state.transform.map_scale as usize + 1, HashMap::new);
                }
                let resized_sprite_cache = &mut resized_sprite_cache[state.transform.map_scale as usize];
                for room_idx in 0..state.map.as_ref().unwrap().levels.len() {
                    let rect_screen = state.transform.rect_map_to_screen(&state.map.as_ref().unwrap().levels[room_idx].bounds);
                    if rect_screen.intersects(&screen) {
                        state.draw_room_backdrop(room_idx);

                        let should_draw_complex = state.transform.map_scale >= 1;
                        if should_draw_complex {
                            state.draw_room_complex(room_idx, &screen, false, resized_sprite_cache);
                            state.draw_entities_complex(room_idx, &screen, resized_sprite_cache);
                            state.draw_room_complex(room_idx, &screen, true, resized_sprite_cache);
                        } else {
                            state.draw_room_simple(room_idx, false);
                            state.draw_room_simple(room_idx, true);
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
                        let (old_x, old_y) = state.transform.point_screen_to_map(app::event_x(), app::event_y());
                        state.transform.map_scale = (state.transform.map_scale as i32 - screen_y).clamp(1, 30) as u32;
                        let (new_x, new_y) = state.transform.point_screen_to_map(app::event_x(), app::event_y());
                        state.transform.map_corner_x += old_x - new_x;
                        state.transform.map_corner_y += old_y - new_y;
                    } else {
                        let amt = state.transform.size_screen_to_map(30);
                        state.transform.map_corner_x += amt as i32 * screen_x;
                        state.transform.map_corner_y += amt as i32 * screen_y;
                    }
                    b.redraw();
                    true
                },
                enums::Event::Push => {
                    if app::event_button() == 2 {
                        state.prev_mouse = state.transform.point_screen_to_map(app::event_x(), app::event_y());
                        true
                    } else {
                        false
                    }
                }
                enums::Event::Drag => {
                    if app::event_button() == 2 {
                        let new_mouse = state.transform.point_screen_to_map(app::event_x(), app::event_y());
                        let difference = (new_mouse.0 - state.prev_mouse.0, new_mouse.1 - state.prev_mouse.1);
                        state.transform.map_corner_x -= difference.0;
                        state.transform.map_corner_y -= difference.1;
                        b.redraw();
                        true
                    } else {
                        false
                    }
                }
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
        let rect = self.transform.rect_map_to_screen(&room.bounds);
        draw::draw_rect_fill(rect.x, rect.y, rect.width as i32, rect.height as i32, room_empty_color());
    }

    fn draw_room_simple(&mut self, room_idx: usize, foreground: bool) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let (tiles, color) = if foreground {
            (&room.fg_tiles, room_fg_color())
        } else {
            (&room.bg_tiles, room_bg_color())
        };

        let tstride = room.bounds.width / 8;
        let unit = self.transform.size_map_to_screen(8);
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let (sx, sy) = self.transform.point_map_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
                let tile = tiles[(tx + ty * tstride) as usize];
                if tile != '0' {
                    draw::draw_rect_fill(sx, sy, unit as i32, unit as i32, color);
                }
            }
        }
    }

    fn draw_room_complex(&mut self, room_idx: usize, clip_box: &Rect, foreground: bool, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        let (tiles, tiles_asset) = if foreground {
            (&room.fg_tiles, &*assets::FG_TILES)
        } else {
            (&room.bg_tiles, &*assets::BG_TILES)
        };

        let bounds = &self.map.as_ref().unwrap().levels[room_idx].bounds;
        let rect_screen = self.transform.rect_map_to_screen(bounds);
        let mut room_buffer = ImageBuffer::new(rect_screen.height, rect_screen.width);
        let mut room_buffer = room_buffer.as_mut();

        let world_in_view = self.transform.rect_screen_to_map(clip_box.clone());
        let room_in_view = Rect {
            x: world_in_view.x - bounds.x,
            y: world_in_view.y - bounds.y,
            width: world_in_view.width,
            height: world_in_view.height,
        };

        let tstride = room.bounds.width / 8;
        for ty in 0..room.bounds.height / 8 {
            for tx in 0..room.bounds.width / 8 {
                let rx = tx * 8;
                let ry = ty * 8;
                let tile = tiles[(tx + ty * tstride) as usize];
                let (sx, sy) = (tx * self.transform.map_scale, ty * self.transform.map_scale);
                let rect = Rect {
                    x: rx as i32,
                    y: ry as i32,
                    width: 8,
                    height: 8,
                };
                if !rect.intersects(&room_in_view) {
                    continue;
                }
                if let Some(tile) = tiles_asset.get(&tile).and_then(|tileset| tileset.tile(room, foreground, tx as i32, ty as i32)) {
                    assets::GAMEPLAY_ATLAS.draw_tile(tile, sx, sy, self.transform.map_scale, room_buffer.reborrow(), resized_sprite_cache);
                }
            }
        }
        room_buffer.as_ref().draw_clipped(clip_box, rect_screen.x, rect_screen.y);
    }

    fn draw_entities_complex(&mut self, room_idx: usize, clip_box: &Rect, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
        if self.map.as_ref().is_none() {
            return;
        }
        let room = &self.map.as_ref().unwrap().levels[room_idx];

        for entity in &room.entities {
            let cfg = assets::ENTITY_CONFIG.lock().unwrap();
            let config = cfg.get(&entity.name).unwrap_or_else(|| cfg.get("default").unwrap());

            for draw in &config.standard_draw.initial_draw {
                self.draw_entity_directive(room, entity, draw, None, resized_sprite_cache);
            }
        }
    }

    fn draw_entity_directive<'a>(&self, room: &CelesteMapLevel, entity: &'a CelesteMapEntity, draw: &DrawElement, node: Option<usize>, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) -> Result<(), String> {
        // TODO construct this one level up for reuse
        let mut env: HashMap<&'a str, Const> = HashMap::new();
        env.insert("x", Const::from_num(entity.x));
        env.insert("y", Const::from_num(entity.y));
        env.insert("width", Const::from_num(if entity.width == 0 {8} else {entity.width}));
        env.insert("height", Const::from_num(if entity.height == 0 {8} else {entity.height}));
        for (key, val) in &entity.attributes {
            env.insert(key.as_str(), Const::from_attr(val));
        }
        if let Some((x, y)) = entity.nodes.first() {
            env.insert("firstnodex", Const::from_num(*x));
            env.insert("firstnodey", Const::from_num(*y));
        }
        if let Some((x, y)) = entity.nodes.last() {
            env.insert("lastnodex", Const::from_num(*x));
            env.insert("lastnodey", Const::from_num(*y));
        }
        if let Some(node) = node {
            if let Some((x, y)) = entity.nodes.get(node) {
                env.insert("nodex", Const::from_num(*x));
                env.insert("nodey", Const::from_num(*y));
            }
            if let Some((x, y)) = entity.nodes.get(node + 1) {
                env.insert("nextnodex", Const::from_num(*x));
                env.insert("nextnodey", Const::from_num(*y));
                env.insert("nextnodexordefault", Const::from_num(*x));
                env.insert("nextnodeyordefault", Const::from_num(*y));
            } else {
                env.insert("nextnodexordefault", Const::from_num(entity.x));
                env.insert("nextnodeyordefault", Const::from_num(entity.y));
            }
            if let Some((x, y)) = entity.nodes.get(node - 1) {
                env.insert("prevnodex", Const::from_num(*x));
                env.insert("prevnodey", Const::from_num(*y));
                env.insert("prevnodexordefault", Const::from_num(*x));
                env.insert("prevnodeyordefault", Const::from_num(*y));
            } else {
                env.insert("prevnodexordefault", Const::from_num(entity.x));
                env.insert("prevnodeyordefault", Const::from_num(entity.y));
            }
        }

        match draw {
            DrawElement::DrawRect { rect, color, border_color, border_thickness } => {
                let const_rect = map_struct::Rect {
                    x: rect.topleft.x.evaluate(&env)?.as_number()?.to_int(),
                    y: rect.topleft.y.evaluate(&env)?.as_number()?.to_int(),
                    width: rect.size.x.evaluate(&env)?.as_number()?.to_int() as u32,
                    height: rect.size.y.evaluate(&env)?.as_number()?.to_int() as u32,
                };
                let screen_rect = self.transform.rect_map_to_screen(&room.rect_room_to_map(&const_rect));
                let (rgb, a) = color.evaluate(&env)?;
                fltk::draw::set_draw_color(rgb);
                fltk::draw::draw_rect_fill(screen_rect.x, screen_rect.y, screen_rect.width as i32, screen_rect.height as i32, rgb)
            }
            DrawElement::DrawLine { .. } => {}
            DrawElement::DrawCurve { .. } => {}
            DrawElement::DrawImage { texture, bounds, scale, rot, color, tiler } => {
                let texture = texture.evaluate(&env)?.as_string()?;
                let sprite = assets::GAMEPLAY_ATLAS.lookup(texture.as_str()).ok_or_else(|| format!("No such gameplay texture: {}", texture))?;
                let x = bounds.topleft.x.evaluate(&env)?.as_number()?.to_int();
                let y = bounds.topleft.y.evaluate(&env)?.as_number()?.to_int();
                let (x, y) = room.point_room_to_map(x, y);
                let (x, y) = self.transform.point_map_to_screen(x, y);
                assets::GAMEPLAY_ATLAS.resized_sprite(sprite, self.transform.map_scale, resized_sprite_cache).draw(x, y);
            }
        }
        Ok(())
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
