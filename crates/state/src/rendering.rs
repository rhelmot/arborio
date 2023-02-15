use arborio_gfxloader::autotiler::{TextureTile, TileReference};
use arborio_maploader::map_struct::{
    Attribute, CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, CelesteMapStyleground,
    FieldEntry,
};
use arborio_modloader::aggregate::ModuleAggregate;
use arborio_modloader::config::{Const, DrawElement, EntityConfig, Number};
use arborio_modloader::mapstruct_plus_config::{make_entity_env, make_node_env};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::Canvas;
use arborio_utils::vizia::vg::{Color, Paint, Path};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use crate::data::project_map::LevelState;
use crate::data::selection::AppSelection;

pub fn draw_entity(
    config: &EntityConfig,
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    entity: &CelesteMapEntity,
    field: &TileGrid<FieldEntry>,
    selected: bool,
    object_tiles: &TileGrid<i32>,
) {
    let env = make_entity_env(entity);

    for node_idx in 0..entity.nodes.len() {
        for draw in &config.standard_draw.node_draw {
            let env = make_node_env(entity, env.clone(), node_idx);
            if let Err(e) = draw_entity_directive(palette, canvas, draw, &env, field, object_tiles)
            {
                log::warn!("Error drawing {}: {}", &entity.name, e);
            }
        }
    }

    for draw in &config.standard_draw.initial_draw {
        if let Err(e) = draw_entity_directive(palette, canvas, draw, &env, field, object_tiles) {
            log::warn!("Error drawing {}: {}", &entity.name, e);
        }
    }

    if selected {
        for node_idx in 0..entity.nodes.len() {
            for draw in &config.selected_draw.node_draw {
                let env = make_node_env(entity, env.clone(), node_idx);
                if let Err(e) =
                    draw_entity_directive(palette, canvas, draw, &env, field, object_tiles)
                {
                    log::warn!("Error drawing {}: {}", &entity.name, e);
                }
            }
        }

        for draw in &config.selected_draw.initial_draw {
            if let Err(e) = draw_entity_directive(palette, canvas, draw, &env, field, object_tiles)
            {
                log::warn!("Error drawing {}: {}", &entity.name, e);
            }
        }
    }
}

fn draw_entity_directive(
    palette: &ModuleAggregate,
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
            canvas.fill_path(&mut path, &fill);
            canvas.stroke_path(&mut path, &border);
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
            canvas.fill_path(&mut path, &fill);
            canvas.stroke_path(&mut path, &border);
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
            path.move_to(x1, y1);
            path.line_to(x2, y2);
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
            canvas.stroke_path(&mut path, &line);
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
            canvas.stroke_path(&mut path, &line);
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
            let texture = texture.evaluate(env)?;
            let texture = texture.as_string()?;
            if texture.is_empty() {
                return Ok(());
            }
            let point = point.evaluate_float(env)?.to_point().cast_unit();
            let justify = Vector2D::new(*justify_x, *justify_y);
            let color = color.evaluate(env)?;
            let scale = scale.evaluate_float(env)?.to_point().cast_unit();
            let rot = rot.evaluate(env)?.as_number()?.to_float();
            return palette.gameplay_atlas.draw_sprite(
                canvas,
                &texture,
                point,
                None,
                Some(justify),
                Some(scale),
                Some(color),
                rot,
            );
        }
        DrawElement::DrawRectImage {
            texture,
            bounds,
            slice,
            scale: _, // TODO: do we want to allow scale?
            color,
            tiler,
        } => {
            let texture = texture.evaluate(env)?;
            let texture = texture.as_string()?;
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
            let tiler = tiler.evaluate(env)?;
            let tiler = tiler.as_string()?;

            let bounds = Rect {
                origin: Point2D::new(bounds_x, bounds_y),
                size: Size2D::new(bounds_w, bounds_h),
            };

            match tiler.deref() {
                "repeat" => {
                    let Some(dim) = palette.gameplay_atlas.sprite_dimensions(&texture) else { return Err(format!("No such gameplay texture: {texture}")) };
                    let dim: Size2D<f32, UnknownUnit> = dim.cast();
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
                    draw_tiled(palette, canvas, &texture, &bounds, &slice, color)?;
                }
                "9slice" => {
                    let Some(dim) = palette.gameplay_atlas.sprite_dimensions(&texture) else { return Err(format!("No such gameplay texture: {texture}")) };
                    let dim: Size2D<f32, UnknownUnit> = dim.cast();
                    if dim.width < 17.0 || dim.height < 17.0 {
                        return Err(format!(
                            "Cannot draw {texture} as 9slice: must be at least 17x17",
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
                                palette,
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
                            )?;
                        }
                    }
                }
                _ => {
                    if texture.len() != 1 {
                        return Err(format!(
                            "Texture for {tiler} tiler ({texture}) must be one char (for now)",
                        ));
                    }
                    let (tiler, ignore) = if tiler == "fg_ignore" {
                        ("fg", true)
                    } else {
                        (tiler.deref(), false)
                    };
                    let texture = texture.chars().next().unwrap();
                    let Some(tilemap) = palette.autotilers.get(tiler) else { return Err(format!("No such tiler {tiler}")) };
                    let Some(tileset) = tilemap.get(&texture) else { return Err(format!("No such texture {texture} for tiler {tiler}")) };

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
                                } else if let Some(conf) =
                                    palette.entity_config.get(e.name.as_str())
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
                        let tile_ref = if let Some(objtile_idx @ 1..) = object_tiles.get(pt) {
                            TileReference {
                                tile: TextureTile {
                                    x: (*objtile_idx % 32) as u32,
                                    y: (*objtile_idx / 32) as u32,
                                },
                                texture: "tilesets/scenery".into(), // TODO see similar TODO in selection.rs
                            }
                        } else {
                            let Some(tile) = tileset.tile(pt, &mut tiler) else { continue };
                            tile
                        };
                        palette
                            .gameplay_atlas
                            .draw_tile(canvas, tile_ref, fp_pt.x, fp_pt.y, color)?;
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
                    draw_entity_directive(
                        palette,
                        canvas,
                        draw_element,
                        &env2,
                        field,
                        object_tiles,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn draw_tiled(
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    sprite: &str,
    bounds: &Rect<f32, UnknownUnit>,
    slice: &Rect<f32, UnknownUnit>,
    color: Color,
) -> Result<(), String> {
    tile(bounds, slice.width(), slice.height(), |piece| {
        palette.gameplay_atlas.draw_sprite(
            canvas,
            sprite,
            piece.origin,
            Some(Rect::new(slice.origin, piece.size)),
            Some(Vector2D::zero()),
            None,
            Some(color),
            0.0,
        )
    })
}

pub fn tile<F>(
    bounds: &Rect<f32, UnknownUnit>,
    width: f32,
    height: f32,
    mut func: F,
) -> Result<(), String>
where
    F: FnMut(Rect<f32, UnknownUnit>) -> Result<(), String>,
{
    if width == 0.0 || height == 0.0 || bounds.width() == 0.0 || bounds.height() == 0.0 {
        return Ok(());
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
            )?;
        }
    }

    Ok(())
}

pub fn draw_decals(
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    room: &CelesteMapLevel,
    fg: bool,
) {
    let decals = if fg { &room.fg_decals } else { &room.bg_decals };
    for decal in decals {
        draw_decal(palette, canvas, decal);
    }
}

pub fn draw_decal(palette: &ModuleAggregate, canvas: &mut Canvas, decal: &CelesteMapDecal) {
    let texture = decal_texture(decal);
    let scale = Point2D::new(decal.scale_x, decal.scale_y);
    if let Err(e) = palette.gameplay_atlas.draw_sprite(
        canvas,
        &texture,
        Point2D::new(decal.x, decal.y).cast(),
        None,
        None,
        Some(scale),
        None,
        0.0,
    ) {
        log::warn!("Failed drawing decal: {}", e);
        palette
            .gameplay_atlas
            .draw_sprite(
                canvas,
                "__fallback",
                Point2D::new(decal.x, decal.y).cast(),
                None,
                None,
                Some(scale),
                None,
                0.0,
            )
            .unwrap();
    }
}

pub fn decal_texture(decal: &CelesteMapDecal) -> String {
    let path = std::path::Path::new("decals")
        .join(std::path::Path::new(&decal.texture).with_extension(""));
    path.to_str().unwrap().to_owned()
}

pub fn draw_tiles(palette: &ModuleAggregate, canvas: &mut Canvas, room: &LevelState, fg: bool) {
    let (tiles, tiles_asset) = if fg {
        (&room.data.solids, palette.autotilers.get("fg").unwrap())
    } else {
        (&room.data.bg, palette.autotilers.get("bg").unwrap())
    };

    // TODO use point_iter
    for ty in 0..room.data.bounds.height() / 8 {
        for tx in 0..room.data.bounds.width() / 8 {
            let pt = TilePoint::new(tx, ty);
            let rx = (tx * 8) as f32;
            let ry = (ty * 8) as f32;
            let tile = tiles.get(pt).unwrap();
            if let Some(tile) = tiles_asset
                .get(tile)
                .and_then(|tileset| tileset.tile(pt, &mut |pt| room.data.tile(pt, fg)))
            {
                if let Err(e) =
                    palette
                        .gameplay_atlas
                        .draw_tile(canvas, tile, rx, ry, Color::white())
                {
                    log::error!("Failed drawing tile: {}", e);
                }
            }
        }
    }

    let float = if fg { &room.floats.fg } else { &room.floats.bg };

    if let Some((float_pos, float_dat)) = float {
        let mut tiler = |pt| -> Option<char> { Some(float_dat.get_or_default(pt)) };
        let rect = TileRect::new(*float_pos, float_dat.size());
        for pt in rect_point_iter(rect, 1) {
            let float_pt = pt - float_pos.to_vector();
            let ch = float_dat.get_or_default(float_pt);
            if ch != '\0' {
                if let Some(tile) = palette
                    .autotilers
                    .get(if fg { "fg" } else { "bg" })
                    .unwrap()
                    .get(&ch)
                    .and_then(|tileset| tileset.tile(float_pt, &mut tiler))
                {
                    let room_pos = point_tile_to_room(&pt);
                    if let Err(e) = palette.gameplay_atlas.draw_tile(
                        canvas,
                        tile,
                        room_pos.x as f32,
                        room_pos.y as f32,
                        Color::white(),
                    ) {
                        log::error!("{}", e);
                    }
                }
            }
        }
    }
}

pub fn draw_entities(
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    room: &CelesteMapLevel,
    selection: &HashSet<AppSelection>,
) {
    let field = room.occupancy_field();
    for entity in &room.entities {
        let selected = selection.contains(&AppSelection::EntityBody(entity.id, false))
            || (0..entity.nodes.len())
                .any(|i| selection.contains(&AppSelection::EntityNode(entity.id, i, false)));
        draw_entity(
            palette.get_entity_config(&entity.name, false),
            palette,
            canvas,
            entity,
            &field,
            selected,
            &room.object_tiles,
        );
    }
}

pub fn draw_triggers(
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    room: &CelesteMapLevel,
    selection: &HashSet<AppSelection>,
) {
    for trigger in &room.triggers {
        let selected = selection.contains(&AppSelection::EntityBody(trigger.id, true))
            || (0..trigger.nodes.len())
                .any(|i| selection.contains(&AppSelection::EntityNode(trigger.id, i, true)));
        draw_entity(
            palette.get_entity_config(&trigger.name, true),
            palette,
            canvas,
            trigger,
            &TileGrid::empty(),
            selected,
            &TileGrid::empty(),
        );
    }
}

pub fn draw_stylegrounds(
    palette: &ModuleAggregate,
    canvas: &mut Canvas,
    preview: MapPointStrict,
    styles: &[CelesteMapStyleground],
    current_room: &str,
    flags: &HashSet<String>,
    dreaming: bool,
) {
    for bg in styles {
        if !bg.visible(current_room, flags, dreaming) {
            continue;
        }
        let mut color = parse_color(&bg.color).unwrap_or_else(Color::white);
        color.a = bg.alpha;

        if bg.name == "parallax" {
            let posx = bg.x + preview.x as f32 * (1.0 - bg.scroll_x);
            let posy = bg.y + preview.y as f32 * (1.0 - bg.scroll_y);
            let texture = bg
                .attributes
                .get("texture")
                .map_or("".to_owned(), |t| match t {
                    Attribute::Bool(b) => b.to_string(),
                    Attribute::Int(i) => i.to_string(),
                    Attribute::Float(f) => f.to_string(),
                    Attribute::Text(s) => s.to_owned(),
                });

            let atlas = &palette.gameplay_atlas;
            let Some(dim) = atlas.sprite_dimensions(&texture) else { continue };
            let dim = dim.cast().cast_unit();
            let matters = MapRectPrecise::new(
                MapPointPrecise::new(preview.x as f32, preview.y as f32),
                MapSizePrecise::new(320.0, 180.0),
            );
            let mut available = MapRectPrecise::new(MapPointPrecise::new(posx, posy), dim);
            if bg.loop_x {
                available.origin.x = f32::MIN / 2.0;
                available.size.width = f32::MAX;
            }
            if bg.loop_y {
                available.origin.y = f32::MIN / 2.0;
                available.size.height = f32::MAX;
            }
            let Some(intersection) = matters.intersection(&available) else { continue };
            let offset_from_base = intersection.origin - MapPointPrecise::new(posx, posy);
            let chunk_id = offset_from_base.component_div(dim.to_vector()).floor();
            let chunk_offset_from_base = chunk_id.component_mul(dim.to_vector());
            let aligned_intersection = MapRectPrecise::new(
                MapPointPrecise::new(posx, posy) + chunk_offset_from_base,
                intersection.size + dim,
            );
            let scale = Point2D::new(
                if bg.flip_x { -1.0 } else { 1.0 },
                if bg.flip_y { -1.0 } else { 1.0 },
            );
            for point in rect_point_iter2(aligned_intersection, dim.to_vector()) {
                if let Err(e) = atlas.draw_sprite(
                    canvas,
                    &texture,
                    point.cast_unit() + dim.to_vector().cast_unit() / 2.0,
                    None,
                    None,
                    Some(scale),
                    Some(color),
                    0.0,
                ) {
                    log::error!("Failed drawing styleground: {}", e)
                }
            }
        }
    }
}

fn parse_color(color: &str) -> Option<Color> {
    let trimmed = color.trim_start_matches('#');
    if trimmed.len() == 6 {
        let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
        let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
        let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
        Some(Color::rgb(r, g, b))
    } else {
        None
    }
}

pub fn draw_objtiles_float(palette: &ModuleAggregate, canvas: &mut Canvas, room: &LevelState) {
    let Some((float_pos, float_dat)) = &room.floats.obj else { return };
    let rect = TileRect::new(*float_pos, float_dat.size());
    for pt in rect_point_iter(rect, 1) {
        let float_pt = pt - float_pos.to_vector();
        let ch = float_dat.get_or_default(float_pt);
        if ch < 0 {
            continue;
        }
        let tile = TileReference {
            tile: TextureTile {
                x: (ch % 32) as u32,
                y: (ch / 32) as u32,
            },
            texture: "tilesets/scenery".into(), // TODO we shouldn't be doing this lookup during draw. cache this string statically?
        };
        let room_pos = point_tile_to_room(&pt);
        if let Err(e) = palette.gameplay_atlas.draw_tile(
            canvas,
            tile,
            room_pos.x as f32,
            room_pos.y as f32,
            Color::white(),
        ) {
            log::error!("{}", e)
        }
    }
}
