use euclid::{Angle, Point2D, Rect, Size2D, Transform2D, UnknownUnit, Vector2D};
use femtovg::{Color, ImageFlags, Paint, Path, PixelFormat, RenderTarget};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::time;
use vizia::*;

use crate::app_state::{AppSelection, AppState};
use crate::autotiler::{TextureTile, TileReference};
use crate::celeste_mod::entity_config::DrawElement;
use crate::celeste_mod::entity_expression::{Const, Number};
use crate::map_struct::{CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, FieldEntry};
use crate::tools::{Tool, TOOLS};
use crate::units::*;

lazy_static! {
    static ref PERF_MONITOR: bool = env::var("ARBORIO_PERF_MONITOR").is_ok();
}

const BACKDROP_COLOR: Color = Color {
    r: 0.08,
    g: 0.21,
    b: 0.08,
    a: 1.00,
};
const FILLER_COLOR: Color = Color {
    r: 0.50,
    g: 0.25,
    b: 0.00,
    a: 1.00,
};
const ROOM_EMPTY_COLOR: Color = Color {
    r: 0.13,
    g: 0.25,
    b: 0.13,
    a: 1.00,
};
const ROOM_DESELECTED_COLOR: Color = Color {
    r: 0.00,
    g: 0.00,
    b: 0.00,
    a: 0.30,
};

pub struct EditorWidget {}

impl EditorWidget {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {}.build(cx)
    }
}

impl View for EditorWidget {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(window_event) = event.message.downcast() {
            if let WindowEvent::SetCursor(_) = window_event {
                return;
            }

            // TODO: nuance
            cx.style.needs_redraw = true;

            if let WindowEvent::MouseDown(..) = &window_event {
                cx.focused = cx.current;
            }
            let app = cx
                .data::<AppState>()
                .expect("EditorWidget must have an AppState in its ancestry");
            let tool: &mut Box<dyn Tool> = &mut TOOLS.lock().unwrap()[app.current_tool];
            let events = tool.event(window_event, app, cx);
            let cursor = tool.cursor(cx, app);
            for event in events {
                cx.emit(event);
            }
            cx.emit(WindowEvent::SetCursor(cursor));
        }
    }

    fn draw(&self, cx: &mut Context, canvas: &mut Canvas) {
        let app = cx
            .data::<AppState>()
            .expect("EditorWidget must have an AppState in its ancestry");
        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);
        canvas.clear_rect(
            bounds.x as u32,
            bounds.y as u32,
            bounds.w as u32,
            bounds.h as u32,
            BACKDROP_COLOR,
        );
        if !app.map_tab_check() {
            // I am worried there are corner cases in vizia where data may hold stale references
            // for a single frame. if this is the case, I'd like to be able to see that via a single
            // debug print vs many debug prints.
            dbg!(&app.tabs, app.current_tab);
            println!("SOMETHING IS WRONG");
            return;
        }
        let t = &app.map_tab_unwrap().transform;
        canvas.set_transform(t.m11, t.m12, t.m21, t.m22, t.m31.round(), t.m32.round());

        let map = app.loaded_maps.get(&app.map_tab_unwrap().id).unwrap();
        if *PERF_MONITOR {
            let now = time::Instant::now();
            println!("Drew {}ms ago", (now - *app.last_draw.borrow()).as_millis());
            *app.last_draw.borrow_mut() = now;
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
                canvas
                    .create_image_empty(
                        room.bounds.width() as usize,
                        room.bounds.height() as usize,
                        PixelFormat::Rgba8,
                        ImageFlags::NEAREST | ImageFlags::FLIP_Y,
                    )
                    .expect("Failed to allocate ")
            };
            cache.render_cache = Some(target);

            if !cache.render_cache_valid {
                canvas.save();
                canvas.reset();
                canvas.set_render_target(RenderTarget::Image(target));

                canvas.clear_rect(
                    0,
                    0,
                    room.bounds.width() as u32,
                    room.bounds.height() as u32,
                    ROOM_EMPTY_COLOR,
                );
                draw_tiles(app, canvas, room, false);
                draw_decals(app, canvas, room, false);
                draw_triggers(
                    app,
                    canvas,
                    room,
                    if idx == app.map_tab_unwrap().current_room {
                        app.current_selected
                    } else {
                        None
                    },
                );
                draw_entities(
                    app,
                    canvas,
                    room,
                    if idx == app.map_tab_unwrap().current_room {
                        app.current_selected
                    } else {
                        None
                    },
                );
                draw_tiles(app, canvas, room, true);
                draw_decals(app, canvas, room, true);

                canvas.restore();
                canvas.set_render_target(RenderTarget::Screen);
                cache.render_cache_valid = true;
            }

            let mut path = Path::new();
            path.rect(
                0.0,
                0.0,
                room.bounds.width() as f32,
                room.bounds.height() as f32,
            );
            let paint = Paint::image(
                target,
                0.0,
                0.0,
                room.bounds.width() as f32,
                room.bounds.height() as f32,
                0.0,
                1.0,
            );
            canvas.fill_path(&mut path, paint);
            if idx != app.map_tab_unwrap().current_room {
                canvas.fill_path(&mut path, Paint::color(ROOM_DESELECTED_COLOR));
            }
            canvas.restore();
        }

        let tool: &mut Box<dyn Tool> = &mut TOOLS.lock().unwrap()[app.current_tool];
        tool.draw(canvas, app, cx);
    }
}

fn draw_decals(app: &AppState, canvas: &mut Canvas, room: &CelesteMapLevel, fg: bool) {
    let decals = if fg { &room.fg_decals } else { &room.bg_decals };
    for decal in decals {
        let texture = decal_texture(decal);
        let scale = Point2D::new(decal.scale_x, decal.scale_y);
        app.current_palette_unwrap()
            .gameplay_atlas
            .draw_sprite(
                canvas,
                &texture,
                Point2D::new(decal.x, decal.y).cast(),
                None,
                None,
                Some(scale),
                None,
                0.0,
            )
            .unwrap_or_else(|| {
                dbg!(&decal);
            });
    }
}

pub fn decal_texture(decal: &CelesteMapDecal) -> String {
    let path = std::path::Path::new("decals")
        .join(std::path::Path::new(&decal.texture).with_extension(""));
    path.to_str().unwrap().to_owned()
}

fn draw_tiles(app: &AppState, canvas: &mut Canvas, room: &CelesteMapLevel, fg: bool) {
    let (tiles, tiles_asset) = if fg {
        (
            &room.fg_tiles,
            app.current_palette_unwrap().autotilers.get("fg").unwrap(),
        )
    } else {
        (
            &room.bg_tiles,
            app.current_palette_unwrap().autotilers.get("bg").unwrap(),
        )
    };

    // TODO use point_iter
    for ty in 0..room.bounds.height() / 8 {
        for tx in 0..room.bounds.width() / 8 {
            let pt = TilePoint::new(tx, ty);
            let rx = (tx * 8) as f32;
            let ry = (ty * 8) as f32;
            let tile = tiles.get(pt).unwrap();
            if let Some(tile) = tiles_asset
                .get(tile)
                .and_then(|tileset| tileset.tile(pt, &mut |pt| room.tile(pt, fg)))
            {
                app.current_palette_unwrap().gameplay_atlas.draw_tile(
                    canvas,
                    tile,
                    rx,
                    ry,
                    Color::white(),
                );
            }
        }
    }
}

fn draw_entities(
    app: &AppState,
    canvas: &mut Canvas,
    room: &CelesteMapLevel,
    selection: Option<AppSelection>,
) {
    let field = room.occupancy_field();
    for entity in &room.entities {
        let selected = matches!(selection, Some(AppSelection::EntityBody(id, false)) | Some(AppSelection::EntityNode(id, _, false)) if id == entity.id);
        draw_entity(
            app,
            canvas,
            entity,
            &field,
            selected,
            false,
            &room.object_tiles,
        );
    }
}

fn draw_triggers(
    app: &AppState,
    canvas: &mut Canvas,
    room: &CelesteMapLevel,
    selection: Option<AppSelection>,
) {
    for trigger in &room.triggers {
        let selected = matches!(selection, Some(AppSelection::EntityBody(id, true)) | Some(AppSelection::EntityNode(id, _, true)) if id == trigger.id);
        draw_entity(
            app,
            canvas,
            trigger,
            &TileGrid::empty(),
            selected,
            true,
            &TileGrid::empty(),
        );
    }
}

pub fn draw_entity(
    app: &AppState,
    canvas: &mut Canvas,
    entity: &CelesteMapEntity,
    field: &TileGrid<FieldEntry>,
    selected: bool,
    trigger: bool,
    object_tiles: &TileGrid<i32>,
) {
    let config = app
        .current_palette_unwrap()
        .get_entity_config(&entity.name, trigger);
    let env = entity.make_env();

    for node_idx in 0..entity.nodes.len() {
        for draw in &config.standard_draw.node_draw {
            let env = entity.make_node_env(env.clone(), node_idx);
            if let Err(s) = draw_entity_directive(app, canvas, draw, &env, field, object_tiles) {
                println!("{}", s);
            }
        }
    }

    for draw in &config.standard_draw.initial_draw {
        if let Err(s) = draw_entity_directive(app, canvas, draw, &env, field, object_tiles) {
            println!("{}", s);
        }
    }

    if selected {
        for node_idx in 0..entity.nodes.len() {
            for draw in &config.selected_draw.node_draw {
                let env = entity.make_node_env(env.clone(), node_idx);
                if let Err(s) = draw_entity_directive(app, canvas, draw, &env, field, object_tiles)
                {
                    println!("{}", s);
                }
            }
        }

        for draw in &config.selected_draw.initial_draw {
            if let Err(s) = draw_entity_directive(app, canvas, draw, &env, field, object_tiles) {
                println!("{}", s);
            }
        }
    }
}

fn draw_entity_directive(
    app: &AppState,
    canvas: &mut Canvas,
    draw: &DrawElement,
    env: &HashMap<&str, Const>,
    field: &TileGrid<FieldEntry>,
    object_tiles: &TileGrid<i32>,
) -> Result<(), String> {
    match draw {
        DrawElement::DrawRect {
            rect,
            color,
            border_color,
            border_thickness,
        } => {
            let x = rect.topleft.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y = rect.topleft.y.evaluate(env)?.as_number()?.to_int() as f32;
            let width = rect.size.x.evaluate(env)?.as_number()?.to_int() as f32;
            let height = rect.size.y.evaluate(env)?.as_number()?.to_int() as f32;
            let fill = Paint::color(color.evaluate(env)?);
            let border_color_eval = border_color.evaluate(env)?;
            let border_thickness = if border_color_eval.a == 0.0 {
                0.0
            } else {
                *border_thickness as f32
            };
            let mut border = Paint::color(border_color_eval);
            border.set_line_width(border_thickness);
            border.set_anti_alias(false);
            let x = x + border_thickness;
            let y = y + border_thickness;
            let width = width - border_thickness * 2.0;
            let height = height - border_thickness * 2.0;

            let mut path = Path::new();
            path.rect(x, y, width, height);
            canvas.fill_path(&mut path, fill);
            canvas.stroke_path(&mut path, border);
        }
        DrawElement::DrawEllipse {
            rect,
            color,
            border_color,
            border_thickness,
        } => {
            let x = rect.topleft.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y = rect.topleft.y.evaluate(env)?.as_number()?.to_int() as f32;
            let width = rect.size.x.evaluate(env)?.as_number()?.to_int() as f32;
            let height = rect.size.y.evaluate(env)?.as_number()?.to_int() as f32;
            let fill = Paint::color(color.evaluate(env)?);
            let border_color_eval = border_color.evaluate(env)?;
            let border_thickness = if border_color_eval.a == 0.0 {
                0.0
            } else {
                *border_thickness as f32
            };
            let mut border = Paint::color(border_color_eval);
            border.set_line_width(border_thickness);
            border.set_anti_alias(false);
            let x = x + border_thickness;
            let y = y + border_thickness;
            let width = width - border_thickness * 2.0;
            let height = height - border_thickness * 2.0;

            let mut path = Path::new();
            path.ellipse(x + width / 2.0, y + width / 2.0, width / 2.0, height / 2.0);
            canvas.fill_path(&mut path, fill);
            canvas.stroke_path(&mut path, border);
        }
        DrawElement::DrawLine {
            start,
            end,
            color,
            arrowhead,
            thickness,
        } => {
            let x1 = start.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y1 = start.y.evaluate(env)?.as_number()?.to_int() as f32;
            let x2 = end.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y2 = end.y.evaluate(env)?.as_number()?.to_int() as f32;
            let mut line = Paint::color(color.evaluate(env)?);
            line.set_line_width(*thickness as f32);
            line.set_anti_alias(false);

            let mut path = Path::new();
            path.move_to(x1 as f32, y1 as f32);
            path.line_to(x2 as f32, y2 as f32);
            if *arrowhead {
                let vec: Vector2D<f32, UnknownUnit> =
                    Vector2D::new(x2 - x1, y2 - y1).normalize() * 8.0;
                let vec1: Vector2D<f32, UnknownUnit> =
                    Transform2D::rotation(Angle::radians(1.0)).transform_vector(vec);
                let vec2: Vector2D<f32, UnknownUnit> =
                    Transform2D::rotation(Angle::radians(-1.0)).transform_vector(vec);
                let endpoint: Point2D<f32, UnknownUnit> = Point2D::new(x2, y2);
                let tail1 = endpoint - vec1;
                let tail2 = endpoint - vec2;
                path.move_to(tail1.x, tail1.y);
                path.line_to(endpoint.x, endpoint.y);
                path.line_to(tail2.x, tail2.y);
            }
            canvas.stroke_path(&mut path, line);
        }
        DrawElement::DrawCurve {
            start,
            end,
            middle,
            color,
            thickness,
        } => {
            let x1 = start.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y1 = start.y.evaluate(env)?.as_number()?.to_int() as f32;
            let x4 = end.x.evaluate(env)?.as_number()?.to_int() as f32;
            let y4 = end.y.evaluate(env)?.as_number()?.to_int() as f32;
            // the control point for the quadratic bezier
            let xq = middle.x.evaluate(env)?.as_number()?.to_int() as f32;
            let yq = middle.y.evaluate(env)?.as_number()?.to_int() as f32;
            let mut line = Paint::color(color.evaluate(env)?);
            line.set_line_width(*thickness as f32);
            line.set_anti_alias(false);

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
        DrawElement::DrawPointImage {
            texture,
            point,
            justify_x,
            justify_y,
            scale,
            rot,
            color,
        } => {
            let texture = texture.evaluate(env)?.as_string()?;
            if texture.is_empty() {
                return Ok(());
            }
            let point = point.evaluate_float(env)?.to_point().cast_unit();
            let justify = Vector2D::new(*justify_x, *justify_y);
            let color = color.evaluate(env)?;
            let scale = scale.evaluate_float(env)?.to_point().cast_unit();
            let rot = rot.evaluate(env)?.as_number()?.to_float();
            if app
                .current_palette_unwrap()
                .gameplay_atlas
                .draw_sprite(
                    canvas,
                    &texture,
                    point,
                    None,
                    Some(justify),
                    Some(scale),
                    Some(color),
                    rot,
                )
                .is_none()
            {
                return Err(format!("No such gameplay texture: {}", texture));
            }
        }
        DrawElement::DrawRectImage {
            texture,
            bounds,
            slice,
            scale: _, // TODO: do we want to allow scale?
            color,
            tiler,
        } => {
            let texture = texture.evaluate(env)?.as_string()?;
            if texture.is_empty() {
                return Ok(());
            }
            let slice_x = slice.topleft.x.evaluate(env)?.as_number()?.to_int() as f32;
            let slice_y = slice.topleft.y.evaluate(env)?.as_number()?.to_int() as f32;
            let slice_w = slice.size.x.evaluate(env)?.as_number()?.to_int() as f32;
            let slice_h = slice.size.y.evaluate(env)?.as_number()?.to_int() as f32;
            let bounds_x = bounds.topleft.x.evaluate(env)?.as_number()?.to_int() as f32;
            let bounds_y = bounds.topleft.y.evaluate(env)?.as_number()?.to_int() as f32;
            let bounds_w = bounds.size.x.evaluate(env)?.as_number()?.to_int() as f32;
            let bounds_h = bounds.size.y.evaluate(env)?.as_number()?.to_int() as f32;
            let color = color.evaluate(env)?;
            let tiler = tiler.evaluate(env)?.as_string()?;

            let bounds = Rect {
                origin: Point2D::new(bounds_x, bounds_y),
                size: Size2D::new(bounds_w, bounds_h),
            };

            match tiler.as_str() {
                "repeat" => {
                    let dim: Size2D<f32, UnknownUnit> = if let Some(dim) = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&texture)
                    {
                        dim
                    } else {
                        return Err(format!("No such gameplay texture: {}", texture));
                    }
                    .cast();
                    let slice: Rect<f32, UnknownUnit> = if slice_w == 0.0 {
                        Rect {
                            origin: Point2D::zero(),
                            size: dim,
                        }
                    } else {
                        Rect {
                            origin: Point2D::new(slice_x, slice_y),
                            size: Size2D::new(slice_w, slice_h),
                        }
                    };
                    draw_tiled(app, canvas, &texture, &bounds, &slice, color);
                }
                "9slice" => {
                    let dim: Size2D<f32, UnknownUnit> = if let Some(dim) = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&texture)
                    {
                        dim
                    } else {
                        return Err(format!("No such gameplay texture: {}", texture));
                    }
                    .cast();
                    if dim.width < 17.0 || dim.height < 17.0 {
                        return Err(format!(
                            "Cannot draw {} as 9slice: must be at least 17x17",
                            texture
                        ));
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
                                app,
                                canvas,
                                &texture,
                                &Rect::new(
                                    Point2D::new(bounds_starts_x[x], bounds_starts_y[y])
                                        + bounds.origin.to_vector(),
                                    Size2D::new(bounds_sizes_x[x], bounds_sizes_y[y]),
                                ),
                                &Rect::new(
                                    Point2D::new(slice_starts_x[x], slice_starts_y[y]),
                                    Size2D::new(slice_sizes_x[x], slice_sizes_y[y]),
                                ),
                                color,
                            );
                        }
                    }
                }
                _ => {
                    if texture.len() != 1 {
                        return Err(format!(
                            "Texture for {} tiler ({}) must be one char (for now)",
                            tiler, texture
                        ));
                    }
                    let (tiler, ignore) = if tiler == "fg_ignore" {
                        ("fg", true)
                    } else {
                        (tiler.as_str(), false)
                    };
                    let texture = texture.chars().next().unwrap();
                    let tilemap =
                        if let Some(tilemap) = app.current_palette_unwrap().autotilers.get(tiler) {
                            tilemap
                        } else {
                            return Err(format!("No such tiler {}", tiler));
                        };
                    let tileset = if let Some(tileset) = tilemap.get(&texture) {
                        tileset
                    } else {
                        return Err(format!("No such texture {} for tiler {}", texture, tiler));
                    };

                    let tile_bounds =
                        rect_room_to_tile(&bounds.cast::<i32>().cast_unit::<RoomSpace>());
                    let self_entity =
                        if let Some(FieldEntry::Entity(e)) = field.get(tile_bounds.origin) {
                            Some(e)
                        } else {
                            None
                        };
                    let mut tiler = |pt| {
                        if tile_bounds.contains(pt) {
                            return Some(texture);
                        }
                        if ignore {
                            return Some('0');
                        }
                        Some(match field.get_or_default(pt) {
                            FieldEntry::None => '0',
                            FieldEntry::Fg => '2',
                            FieldEntry::Entity(e) => {
                                if self_entity.is_some()
                                    && self_entity.unwrap().attributes == e.attributes
                                {
                                    '1'
                                } else if let Some(conf) = app
                                    .current_palette_unwrap()
                                    .entity_config
                                    .get(e.name.as_str())
                                {
                                    if conf.solid {
                                        '2'
                                    } else {
                                        '0'
                                    }
                                } else {
                                    '0'
                                }
                            }
                        })
                    };
                    for pt in rect_point_iter(tile_bounds, 1) {
                        let fp_pt = point_tile_to_room(&pt).cast::<f32>();
                        if let Some(objtile_idx) = object_tiles.get(pt) {
                            if *objtile_idx > 0 {
                                app.current_palette_unwrap().gameplay_atlas.draw_tile(
                                    canvas,
                                    TileReference {
                                        tile: TextureTile {
                                            x: (*objtile_idx % 32) as u32,
                                            y: (*objtile_idx / 32) as u32,
                                        },
                                        texture: "tilesets/scenery".into(), // TODO see similar TODO in selection.rs
                                    },
                                    fp_pt.x,
                                    fp_pt.y,
                                    color,
                                );
                                continue;
                            }
                        }
                        if let Some(tile) = tileset.tile(pt, &mut tiler) {
                            app.current_palette_unwrap()
                                .gameplay_atlas
                                .draw_tile(canvas, tile, fp_pt.x, fp_pt.y, color);
                        }
                    }
                }
            }
        }
        DrawElement::DrawRectCustom {
            interval,
            rect,
            draw,
        } => {
            let rect = rect.evaluate_float(env)?;
            let mut env2 = env.clone();
            for point in rect_point_iter(rect, *interval) {
                env2.insert("customx", Const::Number(Number(point.x as f64)));
                env2.insert("customy", Const::Number(Number(point.y as f64)));

                for draw_element in draw {
                    draw_entity_directive(app, canvas, draw_element, &env2, field, object_tiles)?;
                }
            }
        }
    }
    Ok(())
}

fn draw_tiled(
    app: &AppState,
    canvas: &mut Canvas,
    sprite: &str,
    bounds: &Rect<f32, UnknownUnit>,
    slice: &Rect<f32, UnknownUnit>,
    color: femtovg::Color,
) {
    tile(bounds, slice.width(), slice.height(), |piece| {
        app.current_palette_unwrap().gameplay_atlas.draw_sprite(
            canvas,
            sprite,
            piece.origin,
            Some(Rect::new(slice.origin, piece.size)),
            Some(Vector2D::zero()),
            None,
            Some(color),
            0.0,
        );
    });
}

pub fn tile<F>(bounds: &Rect<f32, UnknownUnit>, width: f32, height: f32, mut func: F)
where
    F: FnMut(Rect<f32, UnknownUnit>),
{
    if width == 0.0 || height == 0.0 || bounds.width() == 0.0 || bounds.height() == 0.0 {
        return;
    }

    let whole_count_x = (bounds.width() / width).ceil() as i32;
    let whole_count_y = (bounds.height() / height).ceil() as i32;
    for x in 0..whole_count_x {
        for y in 0..whole_count_y {
            func(
                Rect {
                    origin: Point2D::new(
                        bounds.min_x() + (x as f32 * width),
                        bounds.min_y() + (y as f32 * height),
                    ),
                    size: Size2D::new(width, height),
                }
                .intersection(bounds)
                .unwrap(),
            );
        }
    }
}
