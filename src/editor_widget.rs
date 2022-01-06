use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::time;
use std::env;
use lazy_static::lazy_static;
use vizia::*;
use femtovg::{Color, ImageFlags, Paint, Path, PixelFormat, RenderTarget};
use euclid::{Rect, Point2D, Vector2D, Size2D, UnknownUnit, Transform2D, Angle};

use crate::map_struct::{CelesteMapEntity, CelesteMapLevel};
use crate::entity_config::{DrawElement, AutotilerType};
use crate::entity_expression::Const;
use crate::map_struct;
use crate::atlas_img::SpriteReference;
use crate::assets;
use crate::app_state::AppState;
use crate::units::*;
use crate::autotiler;
use crate::tools::{TOOLS, Tool};

lazy_static! {
    static ref PERF_MONITOR: bool = {
        env::var("ARBORIO_PERF_MONITOR").is_ok()
    };
}

const BACKDROP_COLOR: Color          = Color { r: 0.08, g: 0.21, b: 0.08, a: 1.00 };
const FILLER_COLOR: Color            = Color { r: 0.50, g: 0.25, b: 0.00, a: 1.00 };
const ROOM_EMPTY_COLOR: Color        = Color { r: 0.13, g: 0.25, b: 0.13, a: 1.00 };
const ROOM_DESELECTED_COLOR: Color   = Color { r: 0.00, g: 0.00, b: 0.00, a: 0.30 };
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
            let mut tool: &mut Box<dyn Tool> = &mut TOOLS.lock().unwrap()[state.current_tool];
            let events = tool.event(window_event, state, cx);
            for event in events {
                cx.emit(event);
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

            for (idx, room) in map.levels.iter().enumerate() {
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
                    draw_tiles(canvas, room, false);
                    draw_entities(canvas, room);
                    draw_tiles(canvas, room, true);

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
                if idx != state.current_room {
                    canvas.fill_path(&mut path, Paint::color(ROOM_DESELECTED_COLOR));
                }
                canvas.restore();
            }
        }
    }
}

fn draw_tiles(canvas: &mut Canvas, room: &CelesteMapLevel, fg: bool) {
    let (tiles, tiles_asset) = if fg {
        (&room.fg_tiles, &*assets::FG_TILES)
    } else {
        (&room.bg_tiles, &*assets::BG_TILES)
    };

    let tstride = room.bounds.width() / 8;
    for ty in 0..room.bounds.height() / 8 {
        for tx in 0..room.bounds.width() / 8 {
            let pt = TilePoint::new(tx, ty);
            let rx = (tx * 8) as f32;
            let ry = (ty * 8) as f32;
            let tile = tiles[(tx + ty * tstride) as usize];
            if let Some(tile) = tiles_asset.get(&tile).and_then(|tileset| tileset.tile(room, fg, pt)) {
                assets::GAMEPLAY_ATLAS.draw_tile(canvas, tile, rx, ry);
            }
        }
    }
}

fn draw_entities(canvas: &mut Canvas, room: &CelesteMapLevel) {
    for entity in &room.entities {
        let mut env: HashMap<&str, Const> = HashMap::new();
        env.insert("x", Const::from_num(entity.x));
        env.insert("y", Const::from_num(entity.y));
        env.insert("width", Const::from_num(entity.width));
        env.insert("height", Const::from_num(entity.height));
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

        let cfg = assets::ENTITY_CONFIG.lock().unwrap();
        let config = cfg.get(&entity.name).unwrap_or_else(|| cfg.get("default").unwrap());

        for draw in &config.standard_draw.node_draw {
            for node_idx in 0..entity.nodes.len() {
                let mut env = env.clone();
                if let Some((x, y)) = entity.nodes.get(node_idx) {
                    env.insert("nodex", Const::from_num(*x));
                    env.insert("nodey", Const::from_num(*y));
                }
                if let Some((x, y)) = entity.nodes.get(node_idx + 1) {
                    env.insert("nextnodex", Const::from_num(*x));
                    env.insert("nextnodey", Const::from_num(*y));
                    env.insert("nextnodexordefault", Const::from_num(*x));
                    env.insert("nextnodeyordefault", Const::from_num(*y));
                } else {
                    env.insert("nextnodexordefault", Const::from_num(entity.x));
                    env.insert("nextnodeyordefault", Const::from_num(entity.y));
                }
                if let Some((x, y)) = entity.nodes.get(node_idx.overflowing_sub(1).0) {
                    env.insert("prevnodex", Const::from_num(*x));
                    env.insert("prevnodey", Const::from_num(*y));
                    env.insert("prevnodexordefault", Const::from_num(*x));
                    env.insert("prevnodeyordefault", Const::from_num(*y));
                } else {
                    env.insert("prevnodexordefault", Const::from_num(entity.x));
                    env.insert("prevnodeyordefault", Const::from_num(entity.y));
                }
                if let Err(s) = draw_entity_directive(canvas, draw, &env) {
                    println!("{}", s);
                }
            }
        }

        for draw in &config.standard_draw.initial_draw {
            if let Err(s) = draw_entity_directive(canvas, draw, &env) {
                println!("{}", s);
            }
        }
    }
}

fn draw_entity_directive(canvas: &mut Canvas, draw: &DrawElement, env: &HashMap<&str, Const>) -> Result<(), String> {
    match draw {
        DrawElement::DrawRect { rect, color, border_color, border_thickness } => {
            let x = rect.topleft.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y = rect.topleft.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let width = rect.size.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let height = rect.size.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let fill = Paint::color(color.evaluate(&env)?);
            let border_color_eval = border_color.evaluate(&env)?;
            let border_thickness = if border_color_eval.a == 0.0 { 0.0 } else { *border_thickness as f32 };
            let mut border = Paint::color(border_color_eval);
            border.set_line_width(border_thickness);
            let x = x + border_thickness;
            let y = y + border_thickness;
            let width = width - border_thickness * 2.0;
            let height = height - border_thickness * 2.0;

            let mut path = Path::new();
            path.rect(x, y, width, height);
            canvas.fill_path(&mut path, fill);
            canvas.stroke_path(&mut path, border);
        }
        DrawElement::DrawLine { start, end, color, arrowhead, thickness } => {
            let x1 = start.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y1 = start.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let x2 = end.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y2 = end.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let mut line = Paint::color(color.evaluate(&env)?);
            line.set_line_width(*thickness as f32);

            let mut path = Path::new();
            path.move_to(x1 as f32, y1 as f32);
            path.line_to(x2 as f32, y2 as f32);
            if *arrowhead {
                let vec: Vector2D<f32, UnknownUnit> = Vector2D::new(x2 - x1, y2 - y1).normalize() * 4.0;
                let vec1: Vector2D<f32, UnknownUnit> = Transform2D::rotation(Angle::radians(1.0)).transform_vector(vec);
                let vec2: Vector2D<f32, UnknownUnit> = Transform2D::rotation(Angle::radians(-1.0)).transform_vector(vec);
                let endpoint: Point2D<f32, UnknownUnit> = Point2D::new(x2, y2);
                let tail1 = endpoint - vec1;
                let tail2 = endpoint - vec2;
                path.move_to(tail1.x, tail1.y);
                path.line_to(endpoint.x, endpoint.y);
                path.line_to(tail2.x, tail2.y);
            }
            canvas.stroke_path(&mut path, line);
        }
        DrawElement::DrawCurve { start, end, middle, color, thickness } => {
            let x1 = start.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y1 = start.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let x4 = end.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y4 = end.y.evaluate(&env)?.as_number()?.to_int() as f32;
            // the control point for the quadratic bezier
            let xq = middle.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let yq = middle.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let mut line = Paint::color(color.evaluate(&env)?);
            line.set_line_width(*thickness as f32);

            // the control points for the cubic bezier
            let x2 = (x1 + xq * 2.0) / 3.0;
            let y2 = (y1 + yq * 2.0) / 3.0;
            let x3 = (x4 + xq * 2.0) / 3.0;
            let y3 = (y4 + yq * 2.0) / 3.0;

            let mut path = Path::new();
            path.move_to(x1, y1);
            path.bezier_to(x2, y2, x3, y3, x4, y4);
            canvas.stroke_path(&mut path, line);
        }
        DrawElement::DrawPointImage { texture, point, justify_x, justify_y, scale, rot, color, } => {
            let texture = texture.evaluate(&env)?.as_string()?;
            let sprite = assets::GAMEPLAY_ATLAS.lookup(texture.as_str()).ok_or_else(|| format!("No such gameplay texture: {}", texture))?;
            let x = point.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let y = point.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let dim = assets::GAMEPLAY_ATLAS.sprite_dimensions(sprite);
            let dx = dim.width as f32;
            let dy = dim.height as f32;
            let x = x - (dx * justify_x);
            let y = y - (dy * justify_y);
            // TODO: scale, rot, color
            let slice = Rect::new(Point2D::zero(), dim.cast());
            assets::GAMEPLAY_ATLAS.draw_sprite(canvas, sprite, x, y, slice);
        }
        DrawElement::DrawRectImage { texture, bounds, slice, scale, color, tiler } => {
            let texture = texture.evaluate(&env)?.as_string()?;
            let slice_x = slice.topleft.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let slice_y = slice.topleft.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let slice_w = slice.size.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let slice_h = slice.size.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let bounds_x = bounds.topleft.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let bounds_y = bounds.topleft.y.evaluate(&env)?.as_number()?.to_int() as f32;
            let bounds_w = bounds.size.x.evaluate(&env)?.as_number()?.to_int() as f32;
            let bounds_h = bounds.size.y.evaluate(&env)?.as_number()?.to_int() as f32;

            let sprite = assets::GAMEPLAY_ATLAS.lookup(texture.as_str()).ok_or_else(|| format!("No such gameplay texture: {}", texture))?;
            let slice: Rect<f32, UnknownUnit> = if slice_w == 0.0 {
                Rect {
                    origin: Point2D::zero(),
                    size: assets::GAMEPLAY_ATLAS.sprite_dimensions(sprite).cast(),
                }
            } else {
                Rect {
                    origin: Point2D::new(slice_x, slice_y),
                    size: Size2D::new(slice_w, slice_h),
                }
            };
            let bounds = Rect {
                origin: Point2D::new(bounds_x, bounds_y),
                size: Size2D::new(bounds_w, bounds_h),
            };

            match tiler {
                AutotilerType::Repeat => {
                    draw_tiled(canvas, sprite, &bounds, &slice);
                }
                AutotilerType::NineSlice => {
                    let dim: Size2D<f32, UnknownUnit> = assets::GAMEPLAY_ATLAS.sprite_dimensions(sprite).cast();
                    if dim.width < 17.0 || dim.height < 17.0 {
                        return Err(format!("Cannot draw {} as 9slice: must be at least 17x17", texture))
                    }

                    let slice_starts_x = [0.0, 8.0, dim.width - 8.0];
                    let slice_starts_y = [0.0, 8.0, dim.height - 8.0];
                    let slice_sizes_x = [8.0, dim.width - 16.0, 8.0];
                    let slice_sizes_y = [8.0, dim.height - 16.0, 8.0];
                    let bounds_starts_x = [0.0, 8.0, bounds.width() - 8.0];
                    let bounds_starts_y = [0.0, 8.0, bounds.height() - 8.0];
                    let bounds_sizes_x = [8.0, bounds.width() - 16.0, 8.0];
                    let bounds_sizes_y = [8.0, bounds.height() - 16.0, 8.0];

                    for x in 0..3_usize {
                        for y in 0..3_usize {
                            draw_tiled(
                                canvas, sprite,
                            &Rect::new(
                                Point2D::new(bounds_starts_x[x], bounds_starts_y[y]) + bounds.origin.to_vector(),
                                Size2D::new(bounds_sizes_x[x], bounds_sizes_y[y])),
                            &Rect::new(
                                 Point2D::new(slice_starts_x[x], slice_starts_y[y]),
                                Size2D::new(slice_sizes_x[x], slice_sizes_y[y]))
                            );
                        }
                    }
                }
                AutotilerType::Fg => {}
                AutotilerType::Bg => {}
                AutotilerType::Cassette => {}
                AutotilerType::JumpThru => {}
            }
        }
    }
    Ok(())
}

fn draw_tiled(
    canvas: &mut Canvas,
    sprite: SpriteReference,
    bounds: &Rect<f32, UnknownUnit>,
    slice: &Rect<f32, UnknownUnit>
) {
    tile(bounds, slice.width(), slice.height(), |piece| {
        //let draw_x = piece.x;
        //let draw_y = piece.y;
        //if piece.x < 0.0 {
        //    piece.x = 0.0;
        //}
        //if piece.y < 0.0 {
        //    piece.y = 0.0;
        //}
        assets::GAMEPLAY_ATLAS.draw_sprite(
            canvas,
            sprite,
            piece.min_x(),
            piece.min_y(),
            Rect::new(slice.origin, piece.size)
        );
    });
}

pub fn tile<F>(bounds: &Rect<f32, UnknownUnit>, width: f32, height: f32, mut func: F)
where F: FnMut(Rect<f32, UnknownUnit>) -> ()
{
    if width == 0.0 || height == 0.0 || bounds.width() == 0.0 || bounds.height() == 0.0 {
        return;
    }

    let whole_count_x = (bounds.width() / width).ceil() as i32;
    let whole_count_y = (bounds.height() / height).ceil() as i32;
    for x in 0..whole_count_x {
        for y in 0..whole_count_y {
            func(Rect {
                origin: Point2D::new(bounds.min_x() + (x as f32 * width), bounds.min_y() + (y as f32 * height)),
                size: Size2D::new(width, height)
            }.intersection(bounds).unwrap());
        }
    }
}
