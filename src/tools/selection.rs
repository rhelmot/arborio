use std::collections::HashMap;
use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::{Tool, generic_nav};
use crate::units::*;
use crate::app_state::TileFloat;
use crate::map_struct::CelesteMapLevel;
use crate::assets;

pub struct SelectionTool {
    current_selection: Vec<AppSelection>,
    pending_selection: Vec<AppSelection>,
    fg_float: Option<(TilePoint, TileFloat)>,
    bg_float: Option<(TilePoint, TileFloat)>,

    pointer_reference_point: Option<RoomPoint>,
    selection_reference_points: HashMap<AppSelection, RoomPoint>,
    fg_float_reference_point: Option<RoomPoint>,
    bg_float_reference_point: Option<RoomPoint>,
    dragging: bool,
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
            dragging: false,
            pointer_reference_point: None,
            selection_reference_points: HashMap::new(),
            fg_float_reference_point: None,
            bg_float_reference_point: None
        }
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let nav_events = generic_nav(event, state, cx);
        if nav_events.len() > 0 { return nav_events }

        let room = if let Some(room) = state.current_room_ref() { room } else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap { room_pos_snapped } else { room_pos };

        match event {
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.confirm_selection();
                self.pointer_reference_point = None;
                vec![]
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let got = self.selectable_at(room, state.current_layer, room_pos);
                let events = if self.touches_float(room_pos) || (got.is_some() && self.current_selection.contains(&got.unwrap())) {
                    self.dragging = true;
                    vec![]
                } else {
                    let events = if !cx.modifiers.contains(Modifiers::CTRL) {
                        self.deselect_all(state)
                    } else { vec![] };
                    self.clear_reference_points();
                    self.dragging = false;
                    if let Some(g) = got {
                        self.pending_selection = vec![g];
                    }
                    events
                };
                self.pointer_reference_point = Some(room_pos);
                events
            }
            WindowEvent::MouseMove(..) => {
                if let Some(ref_pos) = self.pointer_reference_point {
                    if cx.mouse.left.state == MouseButtonState::Pressed {
                        if self.dragging {
                            self.nudge(room, room_pos - ref_pos)
                        } else { // select rectangle
                            self.pending_selection = self.selectables_in(room, state.current_layer, RoomRect::new(ref_pos, (room_pos - ref_pos).to_size()));
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            WindowEvent::KeyDown(Code::ArrowDown, _) |
            WindowEvent::KeyDown(Code::ArrowUp, _) |
            WindowEvent::KeyDown(Code::ArrowLeft, _) |
            WindowEvent::KeyDown(Code::ArrowRight, _) => {
                self.clear_reference_points(); // TODO idk if this is necessary but let's be safe
                let events = match event {
                    WindowEvent::KeyDown(Code::ArrowDown, _) => self.nudge(room, RoomVector::new(0, 8)),
                    WindowEvent::KeyDown(Code::ArrowUp, _) => self.nudge(room, RoomVector::new(0, -8)),
                    WindowEvent::KeyDown(Code::ArrowRight, _) => self.nudge(room, RoomVector::new(8, 0)),
                    WindowEvent::KeyDown(Code::ArrowLeft, _) => self.nudge(room, RoomVector::new(-8, 0)),
                    _ => unreachable!(),
                };
                self.clear_reference_points();
                events
            }
            _ => vec![]
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &Context) {
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
            if data.get_at(relative_pos) != '\0' {
                return true;
            }
        }
        if let Some((offset, data)) = &self.bg_float {
            let relative_pos = tile_pos - offset.to_vector();
            if data.get_at(relative_pos) != '\0' {
                return true;
            }
        }
        false
    }

    fn selectable_at(&self, room: &CelesteMapLevel, layer: Layer, room_pos: RoomPoint) -> Option<AppSelection> {
        self.selectables_in(
            room,
            layer,
            RoomRect::new(room_pos, RoomSize::new(1, 1))
        ).first().cloned()
    }

    fn selectables_in(&self, room: &CelesteMapLevel, layer: Layer, room_rect: RoomRect) -> Vec<AppSelection> {
        let mut result = vec![];

        if layer == Layer::FgTiles || layer == Layer::All {
            for tile_pos_unaligned in rect_point_iter(room_rect, 8) {
                let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                if room.tile(tile_pos, true).unwrap_or('0') != '0' {
                    result.push(AppSelection::FgTile(tile_pos));
                }
            }
        }
        if layer == Layer::Entities || layer == Layer::All {
            for entity in room.entities.iter().rev() {
                let config = assets::ENTITY_CONFIG.get(&entity.name).unwrap_or_else(|| assets::ENTITY_CONFIG.get("default").unwrap());
                let env = entity.make_env();
                for node_idx in 0..entity.nodes.len() {
                    let env = entity.make_node_env(env.clone(), node_idx);
                    for rect_conf in &config.hitboxes.node_rects {
                        match rect_conf.evaluate(&env) {
                            Ok(rect) => {
                                if rect.intersects(&room_rect) {
                                    result.push(AppSelection::EntityNode(entity.id, node_idx));
                                }
                            }
                            Err(s) => {
                                println!("{}", s);
                            }
                        }
                    }
                }
                for rect_conf in &config.hitboxes.initial_rects {
                    match rect_conf.evaluate(&env) {
                        Ok(rect) => {
                            if rect.intersects(&room_rect) {
                                result.push(AppSelection::EntityBody(entity.id));
                            }
                        }
                        Err(s) => {
                            println!("{}", s);
                        }
                    }
                }
            }
        }
        if layer == Layer::BgTiles || layer == Layer::All {
            for tile_pos_unaligned in rect_point_iter(room_rect, 8) {
                let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                if room.tile(tile_pos, false).unwrap_or('0') != '0' {
                    result.push(AppSelection::BgTile(tile_pos));
                }
            }
        }

        dbg!(result)
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

    fn nudge(&mut self, room: &CelesteMapLevel, nudge: RoomVector) -> Vec<AppEvent> {
        let mut result = vec![];

        // STEP 1: offload any fgtile/bgtile selections into the floats
        // TODO: do this in an efficient order
        let mut i = 0_usize;
        while i < self.current_selection.len() {
            if let AppSelection::FgTile(pt) = self.current_selection[i] {
                self.add_to_float(room, pt, true);
                result.push(AppEvent::TileUpdate {
                    fg: true,
                    offset: pt,
                    data: TileFloat { tiles: vec!['0'], stride: 1 }
                });
                self.current_selection.remove(i);
                continue;
            } else if let AppSelection::BgTile(pt) = self.current_selection[i] {
                self.add_to_float(room, pt, false);
                result.push(AppEvent::TileUpdate {
                    fg: false,
                    offset: pt,
                    data: TileFloat { tiles: vec!['0'], stride: 1 }
                });
                self.current_selection.remove(i);
                continue;
            }

            i += 1;
        }

        // STEP 2: If we're just starting to move, mark where we started to move from
        if !self.has_reference_points() {
            self.collect_reference_points(room);
        }

        // STEP 3: Move everything
        if let Some((_, float)) = self.fg_float.take() {
            let offset = self.fg_float_reference_point.unwrap();
            self.fg_float = Some((point_room_to_tile(&(offset + nudge)), float));
        }
        if let Some((_, float)) = self.bg_float.take() {
            let offset = self.bg_float_reference_point.unwrap();
            self.bg_float = Some((point_room_to_tile(&(offset + nudge)), float));
        }
        let mut entity_updates = HashMap::new();
        for selected in &self.current_selection {
            let offset = self.selection_reference_points[selected];
            match selected {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) => unreachable!(),
                AppSelection::EntityBody(id) => {
                    let e = entity_updates.entry(*id).or_insert_with(|| room.entity(*id).unwrap().clone()); // one of the riskier unwraps I've written
                    e.x = offset.x + nudge.x;
                    e.y = offset.y + nudge.y;
                }
                AppSelection::EntityNode(id, node_idx) => {
                    let e = entity_updates.entry(*id).or_insert_with(|| room.entity(*id).unwrap().clone());
                    e.nodes[*node_idx] = (offset.x + nudge.x, offset.y + nudge.y);
                }
            }
        }
        for (_, entity) in entity_updates.into_iter() {
            result.push(AppEvent::EntityUpdate { entity })
        }
        result
    }

    fn add_to_float(&mut self, room: &CelesteMapLevel, pt: TilePoint, fg: bool) {
        if let Some(ch) = room.tile(pt, fg) {
            let float = if fg { &mut self.fg_float } else { &mut self.bg_float };
            let mut default = (pt, TileFloat { tiles: vec![], stride: 1 });
            let (old_origin, old_dat) = float.as_mut().unwrap_or_else(|| &mut default);
            let old_size = TileVector::new(old_dat.stride as i32, (old_dat.tiles.len() / old_dat.stride) as i32);
            let old_supnum = *old_origin + old_size;

            let new_origin = old_origin.min(pt);
            let new_supnum = old_supnum.max(pt + TileVector::new(1, 1));
            let new_size = (new_supnum - new_origin);

            let new_dat = if new_size != old_size {
                let mut new_dat = TileFloat { tiles: vec!['0'; (new_size.x * new_size.y) as usize], stride: new_size.x as usize };
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

    fn collect_reference_points(&mut self, room: &CelesteMapLevel) {
        if let Some((pt, _)) = &self.fg_float {
            self.fg_float_reference_point = Some(point_tile_to_room(pt));
        }
        if let Some((pt, _)) = &self.bg_float {
            self.bg_float_reference_point = Some(point_tile_to_room(pt));
        }
        for selection in &self.current_selection {
            let pt = match selection {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) => unreachable!(),
                AppSelection::EntityBody(id) => {
                    let e = room.entity(*id).unwrap();
                    RoomPoint::new(e.x, e.y)
                }
                AppSelection::EntityNode(id, node_idx) => {
                    let e = room.entity(*id).unwrap();
                    RoomPoint::new(e.nodes[*node_idx].0, e.nodes[*node_idx].1)
                }
            };
            self.selection_reference_points.insert(*selection, pt);
        }
    }

    fn has_reference_points(&self) -> bool {
        !self.selection_reference_points.is_empty() ||
            self.fg_float_reference_point.is_some() ||
            self.bg_float_reference_point.is_some()
    }

    fn clear_reference_points(&mut self) {
        self.selection_reference_points = HashMap::new();
        self.fg_float_reference_point = None;
        self.bg_float_reference_point = None;
    }
}
