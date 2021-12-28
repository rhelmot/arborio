use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::time;
use std::env;
use lazy_static::lazy_static;
use vizia::*;
use femtovg::{Color, ImageFlags, Paint, Path, PixelFormat, RenderTarget};

use crate::map_struct::{CelesteMapEntity, CelesteMapLevel};
use crate::entity_config::{DrawElement, AutotilerType};
use crate::entity_expression::Const;
use crate::map_struct;
use crate::atlas_img::SpriteReference;
use crate::assets;
use crate::app_state::AppState;
use crate::units::*;
use crate::autotiler;

lazy_static! {
    static ref PERF_MONITOR: bool = {
        env::var("ARBORIO_PERF_MONITOR").is_ok()
    };
}

const BACKDROP_COLOR: Color          = Color { r: 0.08, g: 0.21, b: 0.08, a: 1.00 };
const FILLER_COLOR: Color            = Color { r: 0.50, g: 0.25, b: 0.00, a: 1.00 };
const ROOM_EMPTY_COLOR: Color        = Color { r: 0.13, g: 0.25, b: 0.13, a: 1.00 };
const ROOM_FG_COLOR: Color           = Color { r: 0.21, g: 0.38, b: 0.88, a: 1.00 };
const ROOM_BG_COLOR: Color           = Color { r: 0.08, g: 0.08, b: 0.25, a: 1.00 };
const ROOM_ENTITY_COLOR: Color       = Color { r: 1.00, g: 0.00, b: 0.00, a: 1.00 };

pub struct EditorWidget {
}

impl EditorWidget {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {}.build(cx)
    }
}

impl View for EditorWidget {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(window_event) = event.message.downcast() {
            let state = cx.data::<AppState>().expect("EditorWidget must have an AppState in its ancestry");
            if let Some(app_event) = state.tool().translate_event(window_event, state, cx) {
                cx.emit(app_event);
            }
        }
    }

    fn draw(&self, cx: &Context, canvas: &mut Canvas) {
        let state = cx.data::<AppState>().expect("EditorWidget must have an AppState in its ancestry");
        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);
        canvas.clear_rect(bounds.x as u32, bounds.y as u32, bounds.w as u32, bounds.h as u32, BACKDROP_COLOR);
        let t = &state.transform;
        canvas.set_transform(t.m11, t.m12, t.m21, t.m22, t.m31.round(), t.m32.round());

        if let Some(map) = &state.map {
            if *PERF_MONITOR {
                let now = time::Instant::now();
                println!("Drew {}ms ago", (now - *state.last_draw.borrow()).as_millis());
                *state.last_draw.borrow_mut() = now;
            }

            let mut path = Path::new();
            for filler in &map.filler {
                path.rect(
                    filler.origin.x as f32,
                    filler.origin.y as f32,
                    filler.width() as f32,
                    filler.height() as f32,
                );
            }
            canvas.fill_path(&mut path, Paint::color(FILLER_COLOR));

            for room in &map.levels {
                canvas.save();
                canvas.translate(room.bounds.min_x() as f32, room.bounds.min_y() as f32);
                let mut cache = room.cache.borrow_mut();
                let target = if let Some(target) = cache.render_cache {
                    target
                } else {
                    canvas.create_image_empty(room.bounds.width() as usize, room.bounds.height() as usize, PixelFormat::Rgba8, ImageFlags::NEAREST | ImageFlags::FLIP_Y)
                        .expect("Failed to allocate ")
                };
                cache.render_cache = Some(target);

                if !cache.render_cache_valid {
                    canvas.save();
                    canvas.reset();
                    canvas.set_render_target(RenderTarget::Image(target));

                    canvas.clear_rect(
                        0, 0,
                        room.bounds.width() as u32,
                        room.bounds.height() as u32,
                        ROOM_EMPTY_COLOR,
                    );
                    self.draw_tiles(cx, canvas, room, false);
                    self.draw_tiles(cx, canvas, room, true);

                    canvas.restore();
                    canvas.set_render_target(RenderTarget::Screen);
                    cache.render_cache_valid = true;
                }

                let mut path = Path::new();
                path.rect(
                    0.0, 0.0,
                    room.bounds.width() as f32,
                    room.bounds.height() as f32,
                );
                let paint = Paint::image(
                    target, 0.0, 0.0,
                    room.bounds.width() as f32,
                    room.bounds.height() as f32,
                    0.0, 1.0,
                );
                canvas.fill_path(&mut path, paint);
                canvas.restore();
            }
        }
    }
}

impl EditorWidget {
    fn draw_tiles(&self, cx: &Context, canvas: &mut Canvas, room: &CelesteMapLevel, fg: bool) {
         let (tiles, tiles_asset) = if fg {
             (&room.fg_tiles, &*assets::FG_TILES)
         } else {
             (&room.bg_tiles, &*assets::BG_TILES)
         };

         let tstride = room.bounds.width() / 8;
         for ty in 0..room.bounds.height() / 8 {
             for tx in 0..room.bounds.width() / 8 {
                 let rx = (tx * 8) as f32;
                 let ry = (ty * 8) as f32;
                 let tile = tiles[(tx + ty * tstride) as usize];
                 if let Some(tile) = tiles_asset.get(&tile).and_then(|tileset| tileset.tile(room, fg, tx as i32, ty as i32)) {
                     let paint = assets::GAMEPLAY_ATLAS.tile_paint(tile, canvas, rx, ry);
                     //let paint = Paint::color(if fg { ROOM_FG_COLOR } else { ROOM_BG_COLOR });
                     let mut path = Path::new();
                     path.rect(rx as f32, ry as f32, 8.0, 8.0);
                     canvas.fill_path(&mut path, paint);
                 }
             }
         }
    }
}

// impl EditorWidget {
//     fn draw(&mut self) {
//         let state = self.state.clone();
//         self.widget.draw(move |b| {
//             let mut state = state.borrow_mut();
//             if *PERF_MONITOR {
//                 let now = time::Instant::now();
//                 println!("Drew {}ms ago", (now - state.last_draw).as_millis());
//                 state.last_draw = now;
//             }
//             let screen = Rect::from_widget(b);
//             draw::push_clip(b.x(), b.y(), b.w(), b.h());
//             draw::draw_rect_fill(b.x(), b.y(), b.w(), b.h(), backdrop_color());
//
//             if let Some(ref map) = state.map {
//                 for filler in &map.filler {
//                     let filler_screen = state.transform.rect_map_to_screen(filler);
//                     if filler_screen.intersects(&screen) {
//                         draw::draw_rect_fill(
//                             filler_screen.x,
//                             filler_screen.y,
//                             filler_screen.width as i32,
//                             filler_screen.height as i32,
//                             filler_color())
//                     }
//                 }
//                 let mut resized_sprite_cache = assets::SPRITE_CACHE.lock().unwrap();
//                 if resized_sprite_cache.len() <= state.transform.map_scale as usize {
//                     resized_sprite_cache.resize_with(state.transform.map_scale as usize + 1, HashMap::new);
//                 }
//                 let resized_sprite_cache = &mut resized_sprite_cache[state.transform.map_scale as usize];
//                 for room_idx in 0..state.map.as_ref().unwrap().levels.len() {
//                     let rect_screen = state.transform.rect_map_to_screen(&state.map.as_ref().unwrap().levels[room_idx].bounds);
//                     if rect_screen.intersects(&screen) {
//                         state.draw_room_backdrop(room_idx);
//
//                         let should_draw_complex = state.transform.map_scale >= 1;
//                         if should_draw_complex {
//                             state.draw_room_complex(room_idx, &screen, false, resized_sprite_cache);
//                             state.draw_entities_complex(room_idx, &screen, resized_sprite_cache);
//                             state.draw_room_complex(room_idx, &screen, true, resized_sprite_cache);
//                         } else {
//                             state.draw_room_simple(room_idx, false);
//                             state.draw_room_simple(room_idx, true);
//                         }
//                     }
//                 }
//             }
//             draw::pop_clip();
//
//             if *PERF_MONITOR {
//                 let now = time::Instant::now();
//                 println!("Draw took {}ms", (now - state.last_draw).as_millis());
//             }
//         });
//     }
// }

// impl EditorState {
//
//     fn draw_room_backdrop(&mut self, room_idx: usize) {
//         if self.map.as_ref().is_none() {
//             return;
//         }
//
//         let room = &self.map.as_ref().unwrap().levels[room_idx];
//         let rect = self.transform.rect_map_to_screen(&room.bounds);
//         draw::draw_rect_fill(rect.x, rect.y, rect.width as i32, rect.height as i32, room_empty_color());
//     }
//
//     fn draw_room_simple(&mut self, room_idx: usize, foreground: bool) {
//         if self.map.as_ref().is_none() {
//             return;
//         }
//         let room = &self.map.as_ref().unwrap().levels[room_idx];
//
//         let (tiles, color) = if foreground {
//             (&room.fg_tiles, room_fg_color())
//         } else {
//             (&room.bg_tiles, room_bg_color())
//         };
//
//         let tstride = room.bounds.width / 8;
//         let unit = self.transform.size_map_to_screen(8);
//         for ty in 0..room.bounds.height / 8 {
//             for tx in 0..room.bounds.width / 8 {
//                 let rx = tx * 8;
//                 let ry = ty * 8;
//                 let (sx, sy) = self.transform.point_map_to_screen(rx as i32 + room.bounds.x, ry as i32 + room.bounds.y);
//                 let tile = tiles[(tx + ty * tstride) as usize];
//                 if tile != '0' {
//                     draw::draw_rect_fill(sx, sy, unit as i32, unit as i32, color);
//                 }
//             }
//         }
//     }
//
//     fn draw_room_complex(&mut self, room_idx: usize, clip_box: &Rect, foreground: bool, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
//         if self.map.as_ref().is_none() {
//             return;
//         }
//         let room = &self.map.as_ref().unwrap().levels[room_idx];
//
//         let (tiles, tiles_asset) = if foreground {
//             (&room.fg_tiles, &*assets::FG_TILES)
//         } else {
//             (&room.bg_tiles, &*assets::BG_TILES)
//         };
//
//         let bounds = &self.map.as_ref().unwrap().levels[room_idx].bounds;
//         let rect_screen = self.transform.rect_map_to_screen(bounds);
//         let mut room_buffer = ImageBuffer::new(rect_screen.height, rect_screen.width);
//         let mut room_buffer = room_buffer.as_mut();
//
//         let world_in_view = self.transform.rect_screen_to_map(clip_box.clone());
//         let room_in_view = Rect {
//             x: world_in_view.x - bounds.x,
//             y: world_in_view.y - bounds.y,
//             width: world_in_view.width,
//             height: world_in_view.height,
//         };
//
//         let tstride = room.bounds.width / 8;
//         for ty in 0..room.bounds.height / 8 {
//             for tx in 0..room.bounds.width / 8 {
//                 let rx = tx * 8;
//                 let ry = ty * 8;
//                 let tile = tiles[(tx + ty * tstride) as usize];
//                 let (sx, sy) = (tx * self.transform.map_scale, ty * self.transform.map_scale);
//                 let rect = Rect {
//                     x: rx as i32,
//                     y: ry as i32,
//                     width: 8,
//                     height: 8,
//                 };
//                 if !rect.intersects(&room_in_view) {
//                     continue;
//                 }
//                 if let Some(tile) = tiles_asset.get(&tile).and_then(|tileset| tileset.tile(room, foreground, tx as i32, ty as i32)) {
//                     assets::GAMEPLAY_ATLAS.draw_tile(tile, sx, sy, self.transform.map_scale, room_buffer.reborrow(), resized_sprite_cache);
//                 }
//             }
//         }
//         room_buffer.as_ref().draw_clipped(clip_box, rect_screen.x, rect_screen.y);
//     }
//
//     fn draw_entities_complex(&mut self, room_idx: usize, clip_box: &Rect, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
//         if self.map.as_ref().is_none() {
//             return;
//         }
//         let room = &self.map.as_ref().unwrap().levels[room_idx];
//
//         for entity in &room.entities {
//             let cfg = assets::ENTITY_CONFIG.lock().unwrap();
//             let config = cfg.get(&entity.name).unwrap_or_else(|| cfg.get("default").unwrap());
//
//             for draw in &config.standard_draw.node_draw {
//                 for node_idx in 0..entity.nodes.len() {
//                     if let Err(s) = self.draw_entity_directive(room, entity, draw, Some(node_idx), resized_sprite_cache) {
//                         println!("{}", s);
//                     }
//                 }
//             }
//             for draw in &config.standard_draw.initial_draw {
//                 if let Err(s) = self.draw_entity_directive(room, entity, draw, None, resized_sprite_cache) {
//                     println!("{}", s);
//                 }
//             }
//         }
//     }
//
//     fn draw_entity_directive<'a>(&self, room: &CelesteMapLevel, entity: &'a CelesteMapEntity, draw: &DrawElement, node: Option<usize>, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) -> Result<(), String> {
//         // TODO construct this one level up for reuse
//         let mut env: HashMap<&'a str, Const> = HashMap::new();
//         env.insert("x", Const::from_num(entity.x));
//         env.insert("y", Const::from_num(entity.y));
//         env.insert("width", Const::from_num(if entity.width == 0 {8} else {entity.width}));
//         env.insert("height", Const::from_num(if entity.height == 0 {8} else {entity.height}));
//         for (key, val) in &entity.attributes {
//             env.insert(key.as_str(), Const::from_attr(val));
//         }
//         if let Some((x, y)) = entity.nodes.first() {
//             env.insert("firstnodex", Const::from_num(*x));
//             env.insert("firstnodey", Const::from_num(*y));
//         }
//         if let Some((x, y)) = entity.nodes.last() {
//             env.insert("lastnodex", Const::from_num(*x));
//             env.insert("lastnodey", Const::from_num(*y));
//         }
//         if let Some(node) = node {
//             if let Some((x, y)) = entity.nodes.get(node) {
//                 env.insert("nodex", Const::from_num(*x));
//                 env.insert("nodey", Const::from_num(*y));
//             }
//             if let Some((x, y)) = entity.nodes.get(node + 1) {
//                 env.insert("nextnodex", Const::from_num(*x));
//                 env.insert("nextnodey", Const::from_num(*y));
//                 env.insert("nextnodexordefault", Const::from_num(*x));
//                 env.insert("nextnodeyordefault", Const::from_num(*y));
//             } else {
//                 env.insert("nextnodexordefault", Const::from_num(entity.x));
//                 env.insert("nextnodeyordefault", Const::from_num(entity.y));
//             }
//             if let Some((x, y)) = entity.nodes.get(node.overflowing_sub(1).0) {
//                 env.insert("prevnodex", Const::from_num(*x));
//                 env.insert("prevnodey", Const::from_num(*y));
//                 env.insert("prevnodexordefault", Const::from_num(*x));
//                 env.insert("prevnodeyordefault", Const::from_num(*y));
//             } else {
//                 env.insert("prevnodexordefault", Const::from_num(entity.x));
//                 env.insert("prevnodeyordefault", Const::from_num(entity.y));
//             }
//         }
//
//         match draw {
//             DrawElement::DrawRect { rect, color, border_color, border_thickness } => {
//                 let const_rect = Rect {
//                     x: rect.topleft.x.evaluate(&env)?.as_number()?.to_int(),
//                     y: rect.topleft.y.evaluate(&env)?.as_number()?.to_int(),
//                     width: rect.size.x.evaluate(&env)?.as_number()?.to_int() as u32,
//                     height: rect.size.y.evaluate(&env)?.as_number()?.to_int() as u32,
//                 };
//                 let screen_rect = self.transform.rect_map_to_screen(&room.rect_room_to_map(&const_rect));
//                 let (rgb, a) = color.evaluate(&env)?;
//                 fltk::draw::set_draw_color(rgb);
//                 fltk::draw::draw_rect_fill(screen_rect.x, screen_rect.y, screen_rect.width as i32, screen_rect.height as i32, rgb)
//             }
//             DrawElement::DrawLine { start, end, color, arrowhead, thickness } => {
//                 let x1 = start.x.evaluate(&env)?.as_number()?.to_int();
//                 let y1 = start.y.evaluate(&env)?.as_number()?.to_int();
//                 let x2 = end.x.evaluate(&env)?.as_number()?.to_int();
//                 let y2 = end.y.evaluate(&env)?.as_number()?.to_int();
//                 let (rgb, a) = color.evaluate(&env)?;
//
//                 let (x1, y1) = room.point_room_to_map(x1, y1);
//                 let (x1, y1) = self.transform.point_map_to_screen(x1, y1);
//                 let (x2, y2) = room.point_room_to_map(x2, y2);
//                 let (x2, y2) = self.transform.point_map_to_screen(x2, y2);
//
//                 // TODO render line manually so we get pixels at higher zoom levels
//                 // http://members.chello.at/~easyfilter/bresenham.html
//                 fltk::draw::set_line_style(fltk::draw::LineStyle::Solid, u32::max(1, thickness * self.transform.map_scale / 8) as i32);
//                 fltk::draw::set_draw_color(rgb);
//                 fltk::draw::draw_line(x1, y1, x2, y2);
//             }
//             DrawElement::DrawCurve { start, end, middle, color, thickness } => {
//                 // TODO as above
//                 let x1 = start.x.evaluate(&env)?.as_number()?.to_int();
//                 let y1 = start.y.evaluate(&env)?.as_number()?.to_int();
//                 let x4 = end.x.evaluate(&env)?.as_number()?.to_int();
//                 let y4 = end.y.evaluate(&env)?.as_number()?.to_int();
//                 // the control point for the quadratic bezier
//                 let xq = middle.x.evaluate(&env)?.as_number()?.to_int();
//                 let yq = middle.y.evaluate(&env)?.as_number()?.to_int();
//                 let (rgb, a) = color.evaluate(&env)?;
//
//                 let (x1, y1) = room.point_room_to_map(x1, y1);
//                 let (x1, y1) = self.transform.point_map_to_screen(x1, y1);
//                 let (x4, y4) = room.point_room_to_map(x4, y4);
//                 let (x4, y4) = self.transform.point_map_to_screen(x4, y4);
//                 let (xq, yq) = room.point_room_to_map(xq, yq);
//                 let (xq, yq) = self.transform.point_map_to_screen(xq, yq);
//
//                 // the control points for the cubic bezier
//                 let x2 = (x1 + xq * 2) / 3;
//                 let y2 = (y1 + yq * 2) / 3;
//                 let x3 = (x4 + xq * 2) / 3;
//                 let y3 = (y4 + yq * 2) / 3;
//
//                 fltk::draw::set_line_style(fltk::draw::LineStyle::Solid, u32::max(1, thickness * self.transform.map_scale / 8) as i32);
//                 fltk::draw::set_draw_color(rgb);
//                 fltk::draw::begin_line();
//                 fltk::draw::draw_curve(
//                     fltk::draw::Coord(x1 as f64, y1 as f64),
//                     fltk::draw::Coord(x2 as f64, y2 as f64),
//                     fltk::draw::Coord(x3 as f64, y3 as f64),
//                     fltk::draw::Coord(x4 as f64, y4 as f64),
//                 );
//                 fltk::draw::end_line();
//             }
//             DrawElement::DrawPointImage { texture, point, justify_x, justify_y, scale, rot, color, } => {
//                 let texture = texture.evaluate(&env)?.as_string()?;
//                 let sprite = assets::GAMEPLAY_ATLAS.lookup(texture.as_str()).ok_or_else(|| format!("No such gameplay texture: {}", texture))?;
//                 let mut x = point.x.evaluate(&env)?.as_number()?.to_int();
//                 let mut y = point.y.evaluate(&env)?.as_number()?.to_int();
//                 let (dx, dy) = assets::GAMEPLAY_ATLAS.dimensions(sprite);
//                 x -= (dx as f32 * justify_x) as i32;
//                 y -= (dy as f32 * justify_y) as i32;
//                 let (x, y) = room.point_room_to_map(x, y);
//                 let (x, y) = self.transform.point_map_to_screen(x, y);
//                 assets::GAMEPLAY_ATLAS.resized_sprite(sprite, self.transform.map_scale, resized_sprite_cache).draw(x, y);
//             }
//             DrawElement::DrawRectImage { texture, bounds, slice, scale, color, tiler } => {
//                 let texture = texture.evaluate(&env)?.as_string()?;
//                 let slice_x = slice.topleft.x.evaluate(&env)?.as_number()?.to_int();
//                 let slice_y = slice.topleft.y.evaluate(&env)?.as_number()?.to_int();
//                 let slice_w = slice.size.x.evaluate(&env)?.as_number()?.to_int();
//                 let slice_h = slice.size.y.evaluate(&env)?.as_number()?.to_int();
//                 let bounds_x = bounds.topleft.x.evaluate(&env)?.as_number()?.to_int();
//                 let bounds_y = bounds.topleft.y.evaluate(&env)?.as_number()?.to_int();
//                 let bounds_w = bounds.size.x.evaluate(&env)?.as_number()?.to_int();
//                 let bounds_h = bounds.size.y.evaluate(&env)?.as_number()?.to_int();
//
//                 let bounds = Rect {
//                     x: bounds_x, y: bounds_y, width: bounds_w as u32, height: bounds_h as u32
//                 };
//                 let bounds = self.transform.rect_map_to_screen(&room.rect_room_to_map(&bounds));
//
//                 let mut slice = Rect {
//                     x: slice_x * self.transform.map_scale as i32 / 8,
//                     y: slice_y * self.transform.map_scale as i32 / 8,
//                     width: slice_w as u32 * self.transform.map_scale / 8,
//                     height: slice_h as u32 * self.transform.map_scale / 8,
//                 };
//
//                 let sprite = assets::GAMEPLAY_ATLAS.lookup(texture.as_str()).ok_or_else(|| format!("No such gameplay texture: {}", texture))?;
//                 let image = assets::GAMEPLAY_ATLAS.resized_sprite(sprite, self.transform.map_scale, resized_sprite_cache);
//                 let image = if slice_w == 0 {
//                     slice.width = image.width();
//                     slice.height = image.height();
//                     image
//                 } else {
//                     let new_width = if slice.x as u32 + slice.width > image.width() {
//                         image.width() - slice.x as u32
//                     } else {
//                         slice.width
//                     };
//                     let new_height = if slice.y as u32 + slice.height > image.height() {
//                         image.height() - slice.y as u32
//                     } else {
//                         slice.height
//                     };
//                     image.subsection(&Rect {
//                         x: slice.x,
//                         y: slice.y,
//                         width: new_width,
//                         height: new_height,
//                     })
//                 };
//
//                 match tiler {
//                     AutotilerType::Repeat => {
//                         image.draw_tiled(&bounds, slice.width, slice.height);
//                     }
//                     AutotilerType::NineSlice => {
//                         if image.width() < 17 * self.transform.map_scale / 8 || image.height() < 17 * self.transform.map_scale / 8 {
//                             return Err(format!("Cannot draw {} as 9slice: must be at least 17x17", texture))
//                         }
//                         let t = self.transform.map_scale;
//                         let ti = t as i32;
//                         let t2 = t * 2;
//                         let w = bounds.width as i32;
//                         let h = bounds.height as i32;
//
//                         let slice1 = image.subsection(&Rect { corner: Point(0, 0), width: t, height: t });
//                         let slice2 = image.subsection(&Rect { corner: Point(ti, 0), width: image.width() - t2, height: t });
//                         let slice3 = image.subsection(&Rect { corner: Point(image.width() as i32 - ti, 0), width: t, height: t });
//                         let slice4 = image.subsection(&Rect { corner: Point(0, ti), width: t, height: image.height() - t2 });
//                         let slice5 = image.subsection(&Rect { corner: Point(ti, ti), width: image.width() - t2, height: image.height() - t2 });
//                         let slice6 = image.subsection(&Rect { corner: Point(image.width() as i32 - ti, ti), width: t, height: image.height() - t2 });
//                         let slice7 = image.subsection(&Rect { corner: Point(0, image.height() as i32 - ti), width: t, height: t });
//                         let slice8 = image.subsection(&Rect { corner: Point(ti, image.height() as i32 - ti), width: image.width() - t2, height: t });
//                         let slice9 = image.subsection(&Rect { corner: Point(image.width() as i32 - ti, image.height() as i32 - ti), width: t, height: t });
//                         slice1.draw_tiled( &Rect { corner: Point(bounds.x, bounds.y), width: t, height: t}, slice1.width(), slice1.height());
//                         slice2.draw_tiled( &Rect { corner: Point(bounds.x + ti, bounds.y), width: bounds.width - t2, height: t}, slice2.width(), slice2.height());
//                         slice3.draw_tiled( &Rect { corner: Point(bounds.x + w - ti, bounds.y), width: t, height: t}, slice3.width(), slice3.height());
//                         slice4.draw_tiled( &Rect { corner: Point(bounds.x, bounds.y + ti), width: t, height: bounds.height - t2}, slice4.width(), slice4.height());
//                         slice5.draw_tiled( &Rect { corner: Point(bounds.x + ti, bounds.y + ti), width: bounds.width - t2, height: bounds.height - t2}, slice5.width(), slice5.height());
//                         slice6.draw_tiled( &Rect { corner: Point(bounds.x + w - ti, bounds.y + ti), width: t, height: bounds.height - t2}, slice6.width(), slice6.height());
//                         slice7.draw_tiled( &Rect { corner: Point(bounds.x, bounds.y + h - ti), width: t, height: t}, slice7.width(), slice7.height());
//                         slice8.draw_tiled( &Rect { corner: Point(bounds.x + ti, bounds.y + h - ti), width: bounds.width - t2, height: t}, slice8.width(), slice8.height());
//                         slice9.draw_tiled( &Rect { corner: Point(bounds.x + w - ti, bounds.y + h - ti), width: t, height: t}, slice9.width(), slice9.height());
//                     }
//                     AutotilerType::Fg => {}
//                     AutotilerType::Bg => {}
//                     AutotilerType::Cassette => {}
//                     AutotilerType::JumpThru => {}
//                 }
//             }
//         }
//         Ok(())
//     }
// }
