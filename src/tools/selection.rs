use std::collections::HashMap;
use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::{Tool, generic_nav};
use crate::units::*;
use crate::map_struct::CelesteMapLevel;
use crate::assets;
use crate::autotiler::AutoTiler;

pub struct SelectionTool {
    current_selection: Vec<AppSelection>,
    pending_selection: Vec<AppSelection>,
    fg_float: Option<(TilePoint, TileGrid<char>)>,
    bg_float: Option<(TilePoint, TileGrid<char>)>,

    status: SelectionStatus,
}

#[derive(Eq, PartialEq, Debug)]
enum SelectionStatus {
    None,
    Selecting(RoomPoint),
    CouldStartDragging(RoomPoint),
    Dragging(DraggingStatus),
}

#[derive(Eq, PartialEq, Debug)]
struct DraggingStatus {
    pointer_reference_point: RoomPoint,
    selection_reference_points: HashMap<AppSelection, RoomPoint>,
    fg_float_reference_point: Option<RoomPoint>,
    bg_float_reference_point: Option<RoomPoint>,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    EntityBody(i32),
    EntityNode(i32, usize),
}

impl Tool for SelectionTool {
    fn name(&self) -> &'static str {
        "Select"
    }

    fn new() -> Self where Self: Sized {
        Self {
            current_selection: vec![],
            pending_selection: vec![],
            fg_float: None,
            bg_float: None,
            status: SelectionStatus::None,
        }
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let nav_events = generic_nav(event, state, cx);
        if nav_events.len() > 0 { return nav_events }

        let room = if let Some(room) = state.current_room_ref() { room } else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos_unsnapped = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos_unsnapped);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap { room_pos_snapped } else { room_pos_unsnapped };

        match event {
            WindowEvent::MouseUp(MouseButton::Left) => {
                if let SelectionStatus::Selecting(_) = self.status {
                    self.confirm_selection();
                }
                self.status = SelectionStatus::None;
                vec![]
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                if self.status == SelectionStatus::None {
                    let got = self.selectable_at(room, state.current_layer, room_pos_unsnapped);
                    if self.touches_float(room_pos) || (got.is_some() && self.current_selection.contains(&got.unwrap())) {
                        self.status = SelectionStatus::CouldStartDragging(room_pos);
                        vec![]
                    } else {
                        self.status = SelectionStatus::Selecting(room_pos);
                        if let Some(g) = got {
                            self.pending_selection = vec![g];
                        }
                        if !cx.modifiers.contains(Modifiers::SHIFT) {
                            self.deselect_all(state)
                        } else {
                            vec![]
                        }
                    }
                } else {
                    vec![]
                }
            }
            WindowEvent::MouseMove(..) => {
                let mut events = if let SelectionStatus::CouldStartDragging(pt) = self.status {
                    self.begin_dragging(room, pt) // sets self.status = Dragging
                } else {
                    vec![]
                };

                events.extend(match self.status {
                    SelectionStatus::None => vec![],
                    SelectionStatus::CouldStartDragging(_) => unreachable!(),
                    SelectionStatus::Selecting(ref_pos) => {
                        self.pending_selection = self.selectables_in(room, state.current_layer, RoomRect::new(ref_pos, (room_pos - ref_pos).to_size()));
                        vec![]
                    }
                    SelectionStatus::Dragging(DraggingStatus { pointer_reference_point, .. }) =>
                        self.nudge(room, room_pos - pointer_reference_point),
                });

                events
            }
            WindowEvent::KeyDown(code, key) if self.status == SelectionStatus::None => {
                match code {
                    Code::ArrowDown => self.nudge(room, RoomVector::new(0, 8)),
                    Code::ArrowUp => self.nudge(room, RoomVector::new(0, -8)),
                    Code::ArrowRight => self.nudge(room, RoomVector::new(8, 0)),
                    Code::ArrowLeft => self.nudge(room, RoomVector::new(-8, 0)),
                    Code::KeyA if cx.modifiers.contains(Modifiers::CTRL) => {
                        self.current_selection = self.selectables_in(room, state.current_layer, RoomRect::new(RoomPoint::new(-1000000, -1000000), RoomSize::new(2000000, 2000000)));
                        vec![]
                    }
                    _ => vec![]
                }
            }
            _ => vec![]
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &Context) {
        canvas.save();
        let room = if let Some(room) = state.current_room_ref() { room } else { return };
        canvas.translate(room.bounds.origin.x as f32, room.bounds.origin.y as f32);
        // no scissor!

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap { room_pos_snapped } else { room_pos };

        if let SelectionStatus::Selecting(ref_pos) = &self.status {
            let selection = rect_normalize(&RoomRect::new(*ref_pos, (room_pos - *ref_pos).to_size()));
            let mut path = femtovg::Path::new();
            path.rect(selection.min_x() as f32, selection.min_y() as f32, selection.width() as f32, selection.height() as f32);
            canvas.stroke_path(&mut path, femtovg::Paint::color(femtovg::Color::rgb(0, 0, 0)).with_line_width(1.5));
        }

        let mut path = femtovg::Path::new();
        for selectable in self.pending_selection.iter().chain(self.current_selection.iter()) {
            for rect in self.rects_of(room, *selectable) {
                path.rect(rect.min_x() as f32, rect.min_y() as f32, rect.width() as f32, rect.height() as f32)
            }
        }

        if let Some((float_pos, float_dat)) = &self.bg_float {
            let mut tiler = |pt| -> Option<char> {
                Some(float_dat.get_or_default(pt))
            };
            let rect = TileRect::new(*float_pos, float_dat.size());
            for pt in rect_point_iter(rect, 1) {
                let float_pt = pt - float_pos.to_vector();
                let ch = float_dat.get_or_default(float_pt);
                if ch != '\0' {
                    if let Some(tile) = assets::BG_TILES.get(&ch).and_then(|tileset| tileset.tile(float_pt, &mut tiler)) {
                        let room_pos = point_tile_to_room(&pt);
                        assets::GAMEPLAY_ATLAS.draw_tile(canvas, tile, room_pos.x as f32, room_pos.y as f32, Color::white().into());
                        path.rect(room_pos.x as f32, room_pos.y as f32, 8.0, 8.0);
                    }
                }
            }
        }
        if let Some((float_pos, float_dat)) = &self.fg_float {
            let mut tiler = |pt| -> Option<char> {
                Some(float_dat.get_or_default(pt))
            };
            let rect = TileRect::new(*float_pos, float_dat.size());
            for pt in rect_point_iter(rect, 1) {
                let float_pt = pt - float_pos.to_vector();
                let ch = float_dat.get_or_default(float_pt);
                if ch != '\0' {
                    if let Some(tile) = assets::FG_TILES.get(&ch).and_then(|tileset| tileset.tile(float_pt, &mut tiler)) {
                        let room_pos = point_tile_to_room(&pt);
                        assets::GAMEPLAY_ATLAS.draw_tile(canvas, tile, room_pos.x as f32, room_pos.y as f32, Color::white().into());
                        path.rect(room_pos.x as f32, room_pos.y as f32, 8.0, 8.0);
                    }
                }
            }
        }

        canvas.fill_path(&mut path, femtovg::Paint::color(femtovg::Color::rgba(255, 255, 0, 128)));

        if self.status == SelectionStatus::None {
            if let Some(sel) = self.selectable_at(room, state.current_layer, room_pos) {
                if !self.current_selection.contains(&sel) {
                    let mut path = femtovg::Path::new();
                    for rect in self.rects_of(room, sel) {
                        path.rect(rect.min_x() as f32, rect.min_y() as f32, rect.width() as f32, rect.height() as f32);
                    }
                    canvas.fill_path(&mut path, femtovg::Paint::color(femtovg::Color::rgba(100, 100, 255, 128)));
                }
            }
        }

        canvas.restore();
    }
}

impl SelectionTool {
    fn confirm_selection(&mut self) {
        self.current_selection.extend(self.pending_selection.drain(..));
    }

    fn touches_float(&self, room_pos: RoomPoint) -> bool {
        let tile_pos = point_room_to_tile(&room_pos);
        if let Some((offset, data)) = &self.fg_float {
            let relative_pos = tile_pos - offset.to_vector();
            if data.get_or_default(relative_pos) != '\0' {
                return true;
            }
        }
        if let Some((offset, data)) = &self.bg_float {
            let relative_pos = tile_pos - offset.to_vector();
            if data.get_or_default(relative_pos) != '\0' {
                return true;
            }
        }
        false
    }

    fn rects_of(&self, room: &CelesteMapLevel, selectable: AppSelection) -> Vec<RoomRect> {
        match selectable {
            AppSelection::FgTile(pt) => vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))],
            AppSelection::BgTile(pt) => vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))],
            AppSelection::EntityBody(id) => {
                if let Some(entity) = room.entity(id) {
                    let config = assets::ENTITY_CONFIG.get(&entity.name).unwrap_or_else(|| assets::ENTITY_CONFIG.get("default").unwrap());
                    let env = entity.make_env();
                    config.hitboxes.initial_rects.iter().filter_map(|r| {
                        match r.evaluate(&env) {
                            Ok(r) => Some(r),
                            Err(s) => {
                                println!("{}", s);
                                None
                            }
                        }
                    }).collect()
                } else {
                    vec![]
                }
            }
            AppSelection::EntityNode(id, node_idx) => {
                if let Some(entity) = room.entity(id) {
                    let config = assets::ENTITY_CONFIG.get(&entity.name).unwrap_or_else(|| assets::ENTITY_CONFIG.get("default").unwrap());
                    let env = entity.make_node_env(entity.make_env(), node_idx);
                    config.hitboxes.node_rects.iter().filter_map(|r| {
                        match r.evaluate(&env) {
                            Ok(r) => Some(r),
                            Err(s) => {
                                println!("{}", s);
                                None
                            }
                        }
                    }).collect()
                } else {
                    vec![]
                }
            }
        }
    }

    fn selectable_at(&self, room: &CelesteMapLevel, layer: Layer, room_pos: RoomPoint) -> Option<AppSelection> {
        self.selectables_in(
            room,
            layer,
            RoomRect::new(room_pos, RoomSize::new(1, 1))
        ).first().cloned()
    }

    fn selectables_in(&self, room: &CelesteMapLevel, layer: Layer, room_rect: RoomRect) -> Vec<AppSelection> {
        let room_rect = rect_normalize(&room_rect);
        let mut result = vec![];

        let room_rect_cropped = room_rect.intersection(&RoomRect::new(RoomPoint::zero(), room.bounds.size.cast_unit()));
        if (layer == Layer::FgTiles || layer == Layer::All) && room_rect_cropped.is_some() {
            for tile_pos_unaligned in rect_point_iter(room_rect_cropped.unwrap(), 8) {
                let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                if room.tile(tile_pos, true).unwrap_or('0') != '0' {
                    result.push(AppSelection::FgTile(tile_pos));
                }
            }
        }
        if layer == Layer::Entities || layer == Layer::All {
            for (idx, entity) in room.entities.iter().enumerate().rev() {
                room.cache_entity_idx(idx);
                for node_idx in 0..entity.nodes.len() {
                    let sel = AppSelection::EntityNode(entity.id, node_idx);
                    if intersects_any(&self.rects_of(room, sel), &room_rect) {
                        result.push(sel);
                    }
                }
                let sel = AppSelection::EntityBody(entity.id);
                if intersects_any(&self.rects_of(room, sel), &room_rect) {
                    result.push(sel);
                }
            }
        }
        if layer == Layer::BgTiles || layer == Layer::All && room_rect_cropped.is_some() {
            for tile_pos_unaligned in rect_point_iter(room_rect_cropped.unwrap(), 8) {
                let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                if room.tile(tile_pos, false).unwrap_or('0') != '0' {
                    result.push(AppSelection::BgTile(tile_pos));
                }
            }
        }

        result
    }

    fn deselect_all(&mut self, state: &AppState) -> Vec<AppEvent> {
        self.current_selection = vec![];
        let mut result = vec![];
        if let Some((offset, data)) = self.fg_float.take() {
            result.push(AppEvent::TileUpdate {
                fg: true, offset, data
            })
        }
        if let Some((offset, data)) = self.bg_float.take() {
            result.push(AppEvent::TileUpdate {
                fg: false, offset, data
            })
        }
        result
    }

    /// This function interprets nudge as relative to the reference positions in Dragging mode and
    /// relative to the current position in other modes.
    fn nudge(&mut self, room: &CelesteMapLevel, nudge: RoomVector) -> Vec<AppEvent> {
        let events = self.float_tiles(room);

        let dragging = if let SelectionStatus::Dragging(dragging) = &self.status { Some(dragging) } else { None };
        if let Some((cur_pt, float)) = self.fg_float.take() {
            let base = dragging
                .map(|d| d.fg_float_reference_point.unwrap())
                .unwrap_or_else(|| point_tile_to_room(&cur_pt));
            self.fg_float = Some((point_room_to_tile(&(base + nudge)), float));
        }
        if let Some((cur_pt, float)) = self.bg_float.take() {
            let base = dragging
                .map(|d| d.bg_float_reference_point.unwrap())
                .unwrap_or_else(|| point_tile_to_room(&cur_pt));
            self.bg_float = Some((point_room_to_tile(&(base + nudge)), float));
        }
        let mut entity_updates = HashMap::new();
        for selected in &self.current_selection {
            match selected {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) => unreachable!(),
                AppSelection::EntityBody(id) => {
                    let e = entity_updates.entry(*id).or_insert_with(|| room.entity(*id).unwrap().clone()); // one of the riskier unwraps I've written
                    let base = dragging
                        .map(|d| d.selection_reference_points[selected])
                        .unwrap_or_else(|| RoomPoint::new(e.x, e.y));
                    e.x = base.x + nudge.x;
                    e.y = base.y + nudge.y;
                }
                AppSelection::EntityNode(id, node_idx) => {
                    let e = entity_updates.entry(*id).or_insert_with(|| room.entity(*id).unwrap().clone());
                    let base = dragging
                        .map(|d| d.selection_reference_points[selected])
                        .unwrap_or_else(|| RoomPoint::new(e.nodes[*node_idx].0, e.nodes[*node_idx].1));
                    e.nodes[*node_idx] = (base.x + nudge.x, base.y + nudge.y);
                }
            }
        }

        entity_updates
            .into_iter()
            .map(|(_, entity)| AppEvent::EntityUpdate { entity })
            .chain(events.into_iter())
            .collect()
    }

    fn add_to_float(&mut self, room: &CelesteMapLevel, pt: TilePoint, fg: bool) {
        if let Some(ch) = room.tile(pt, fg) {
            let float = if fg { &mut self.fg_float } else { &mut self.bg_float };
            let mut default = (pt, TileGrid { tiles: vec![], stride: 1 });
            let (old_origin, old_dat) = float.as_mut().unwrap_or_else(|| &mut default);
            let old_size = TileVector::new(old_dat.stride as i32, (old_dat.tiles.len() / old_dat.stride) as i32);
            let old_supnum = *old_origin + old_size;

            let new_origin = old_origin.min(pt);
            let new_supnum = old_supnum.max(pt + TileVector::new(1, 1));
            let new_size = (new_supnum - new_origin);

            let new_dat = if new_size != old_size {
                let mut new_dat = TileGrid { tiles: vec!['\0'; (new_size.x * new_size.y) as usize], stride: new_size.x as usize };
                let movement = *old_origin - new_origin;
                let dest_start_offset = movement.x + movement.y * new_size.x;
                for line in 0..old_size.y {
                    let src = &old_dat.tiles.as_slice()[(line * old_size.x) as usize..((line+1) * old_size.x) as usize];
                    new_dat.tiles.as_mut_slice()[
                        (dest_start_offset + line * new_size.x) as usize..
                            (dest_start_offset + line * new_size.x + old_size.x) as usize
                        ].clone_from_slice(src);
                }
                let movement = pt - new_origin;
                if fg {
                    self.fg_float = Some((new_origin, new_dat));
                    &mut self.fg_float.as_mut().unwrap().1
                } else {
                    self.bg_float = Some((new_origin, new_dat));
                    &mut self.bg_float.as_mut().unwrap().1
                }
            } else {
                old_dat
            };

            let movement = pt - new_origin;
            new_dat.tiles[(movement.x + movement.y * new_size.x) as usize] = ch;
        }
    }

    fn float_tiles(&mut self, room: &CelesteMapLevel) -> Vec<AppEvent> {
        // TODO: do this in an efficient order to avoid frequent reallocations of the float
        let mut i = 0_usize;
        let mut events = vec![];
        while i < self.current_selection.len() {
            if let AppSelection::FgTile(pt) = self.current_selection[i] {
                self.add_to_float(room, pt, true);
                events.push(AppEvent::TileUpdate {
                    fg: true,
                    offset: pt,
                    data: TileGrid { tiles: vec!['0'], stride: 1 }
                });
                self.current_selection.remove(i);
                continue;
            } else if let AppSelection::BgTile(pt) = self.current_selection[i] {
                self.add_to_float(room, pt, false);
                events.push(AppEvent::TileUpdate {
                    fg: false,
                    offset: pt,
                    data: TileGrid { tiles: vec!['0'], stride: 1 }
                });
                self.current_selection.remove(i);
                continue;
            }

            i += 1;
        }

        events
    }

    fn begin_dragging(&mut self, room: &CelesteMapLevel, pointer_reference_point: RoomPoint) -> Vec<AppEvent> {
        // Offload all fg/bg selections into the floats
        let events = self.float_tiles(room);

        // collect reference points
        let fg_float_reference_point = self.fg_float.as_ref().map(|f| point_tile_to_room(&f.0));
        let bg_float_reference_point = self.bg_float.as_ref().map(|f| point_tile_to_room(&f.0));
        let selection_reference_points = self.current_selection.iter().map(|sel| {
            (*sel, match sel {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) => unreachable!(),
                AppSelection::EntityBody(id) => {
                    let e = room.entity(*id).unwrap();
                    RoomPoint::new(e.x, e.y)
                }
                AppSelection::EntityNode(id, node_idx) => {
                    let e = room.entity(*id).unwrap();
                    RoomPoint::new(e.nodes[*node_idx].0, e.nodes[*node_idx].1)
                }
            })
        }).collect::<HashMap<AppSelection, RoomPoint>>();

        // here's your status!
        self.status = SelectionStatus::Dragging(DraggingStatus {
            pointer_reference_point,
            selection_reference_points,
            fg_float_reference_point,
            bg_float_reference_point
        });

        events
    }
}

// oh would it were that rust iterators weren't a fucking pain to write
fn intersects_any(haystack: &Vec<RoomRect>, needle: &RoomRect) -> bool {
    for hay in haystack {
        if hay.intersects(needle) {
            return true;
        }
    }
    return false;
}
