use std::collections::{HashMap, HashSet};

use arborio_maploader::map_struct::Node;
use arborio_modloader::mapstruct_plus_config::{make_entity_env, make_node_env};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg;

use crate::data::action::{MapAction, RoomAction};
use crate::data::app::{AppEvent, AppState};
use crate::data::project_map::{LevelFloatState, LevelState};
use crate::data::selection::{AppInRoomSelectable, AppSelectable, AppSelection};
use crate::data::tabs::MapTab;
use crate::data::{EventPhase, Layer};
use crate::rendering::decal_texture;
use crate::tools::{generic_nav, Tool};

pub struct SelectionTool {
    pending_selection: HashSet<AppSelection>,

    status: SelectionStatus,
    draw_phase: EventPhase,
}

#[derive(Eq, PartialEq, Debug)]
enum SelectionStatus {
    None,
    Selecting(RoomPoint),
    CouldStartDragging(RoomPoint, RoomPoint),
    Dragging(DraggingStatus),
    Resizing(ResizingStatus),
}

#[derive(Eq, PartialEq, Debug)]
struct DraggingStatus {
    pointer_reference_point: RoomPoint,
    selection_reference_points: HashMap<AppSelection, RoomPoint>,
}

#[derive(Eq, PartialEq, Debug)]
struct ResizingStatus {
    pointer_reference_point: RoomPoint,
    selection_reference_sizes: HashMap<AppSelection, RoomRect>,
    side: ResizeSide,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum ResizeSide {
    None,
    Top,
    Left,
    Bottom,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeSide {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::Top | Self::TopLeft | Self::TopRight)
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom | Self::BottomLeft | Self::BottomRight)
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left | Self::TopLeft | Self::BottomLeft)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right | Self::TopRight | Self::BottomRight)
    }

    pub fn from_sides(at_top: bool, at_bottom: bool, at_left: bool, at_right: bool) -> Self {
        match (at_top, at_bottom, at_left, at_right) {
            (true, false, false, false) => ResizeSide::Top,
            (false, true, false, false) => ResizeSide::Bottom,
            (false, false, true, false) => ResizeSide::Left,
            (false, false, false, true) => ResizeSide::Right,
            (true, false, true, false) => ResizeSide::TopLeft,
            (true, false, false, true) => ResizeSide::TopRight,
            (false, true, true, false) => ResizeSide::BottomLeft,
            (false, true, false, true) => ResizeSide::BottomRight,
            _ => ResizeSide::None,
        }
    }

    fn filter_out_top_bottom(&self) -> Self {
        Self::from_sides(false, false, self.is_left(), self.is_right())
    }

    fn filter_out_left_right(&self) -> Self {
        Self::from_sides(self.is_top(), self.is_bottom(), false, false)
    }

    pub fn to_cursor_icon(self) -> CursorIcon {
        match self {
            ResizeSide::None => CursorIcon::Default,
            ResizeSide::Top => CursorIcon::NResize,
            ResizeSide::Left => CursorIcon::WResize,
            ResizeSide::Bottom => CursorIcon::SResize,
            ResizeSide::Right => CursorIcon::EResize,
            ResizeSide::TopLeft => CursorIcon::NwResize,
            ResizeSide::TopRight => CursorIcon::NeResize,
            ResizeSide::BottomLeft => CursorIcon::SwResize,
            ResizeSide::BottomRight => CursorIcon::SeResize,
        }
    }
}

#[derive(Default)]
pub struct AppEventStaging(Vec<AppEvent>, Vec<MapAction>, Vec<RoomAction>);

#[allow(unused)] // this guy is in a trial period. if we like it we might move it to data
impl AppEventStaging {
    pub fn accumulate(&mut self, other: Self) {
        self.0.extend(other.0);
        self.1.extend(other.1);
        self.2.extend(other.2);
    }

    pub fn push_ind(&mut self, event: AppEvent) {
        self.0.push(event);
    }

    pub fn push_map(&mut self, event: MapAction) {
        self.1.push(event);
    }

    pub fn push_room(&mut self, event: RoomAction) {
        self.2.push(event);
    }

    pub fn finalize(mut self, app: &AppState, draw_phase: EventPhase) -> Vec<AppEvent> {
        self.1
            .extend(self.2.into_iter().map(|r| MapAction::RoomAction {
                idx: app.map_tab_unwrap().current_room,
                event: r,
            }));
        if !self.1.is_empty() {
            self.0.push(app.batch_action(self.1, draw_phase));
        }
        self.0
    }

    pub fn finalize_unique(self, app: &AppState) -> Vec<AppEvent> {
        self.finalize(app, EventPhase::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty() && self.2.is_empty()
    }
}

impl SelectionTool {
    pub fn new() -> Self {
        Self {
            pending_selection: HashSet::new(),
            status: SelectionStatus::None,
            draw_phase: EventPhase::null(),
        }
    }
}

impl Default for SelectionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for SelectionTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let nav_events = generic_nav(event, app, cx, true);
        if !nav_events.is_empty() {
            return nav_events;
        }

        let Some(room) = app.current_room_ref() else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos_unsnapped = (map_pos - room.data.bounds.origin).to_point().cast_unit();
        let room_pos = if app.config.snap {
            let tile_pos = point_room_to_tile(&room_pos_unsnapped);
            point_tile_to_room(&tile_pos)
        } else {
            room_pos_unsnapped
        };

        match event {
            WindowEvent::MouseUp(MouseButton::Left) => {
                let events = if let SelectionStatus::Selecting(_) = self.status {
                    self.confirm_selection(app)
                } else {
                    AppEventStaging::default()
                };
                self.status = SelectionStatus::None;
                events
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                if self.status == SelectionStatus::None {
                    let got = self.selectable_at(app, room, app.current_layer, room_pos_unsnapped);
                    if matches!(got, Some(got) if app.map_tab_unwrap().current_selected.contains(&got)) {
                        self.draw_phase = EventPhase::new();
                        self.status =
                            SelectionStatus::CouldStartDragging(room_pos, room_pos_unsnapped);
                        AppEventStaging::default()
                    } else {
                        self.status = SelectionStatus::Selecting(room_pos);
                        if let Some(g) = got {
                            self.pending_selection = HashSet::from([g]);
                        }
                        if !cx.modifiers.contains(Modifiers::SHIFT) {
                            self.clear_selection(app, &room.floats)
                        } else {
                            AppEventStaging::default()
                        }
                    }
                } else {
                    AppEventStaging::default()
                }
            }
            WindowEvent::MouseMove(..) => {
                let (mut events, floats) =
                    if let SelectionStatus::CouldStartDragging(pt, unsn) = self.status {
                        let (events, floats) = self.begin_dragging(app, room, pt, unsn); // sets self.status = Dragging | Resizing
                        (events, Some(floats))
                    } else {
                        (AppEventStaging::default(), None)
                    };

                events.accumulate(match self.status {
                    SelectionStatus::None => AppEventStaging::default(),
                    SelectionStatus::CouldStartDragging(_, _) => unreachable!(),
                    SelectionStatus::Selecting(ref_pos) => {
                        self.pending_selection = self.selectables_in(
                            app,
                            room,
                            app.current_layer,
                            RoomRect::new(ref_pos, (room_pos - ref_pos).to_size()),
                        );
                        AppEventStaging::default()
                    }
                    SelectionStatus::Dragging(DraggingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.nudge(
                        app,
                        room,
                        room_pos - pointer_reference_point,
                        floats.unwrap_or_else(|| room.floats.clone()),
                    ),
                    SelectionStatus::Resizing(ResizingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.resize(app, room, room_pos - pointer_reference_point),
                });

                events
            }
            WindowEvent::KeyDown(code, _) => {
                if self.status == SelectionStatus::None {
                    let mut old_draw_phase = EventPhase::new();
                    std::mem::swap(&mut old_draw_phase, &mut self.draw_phase);
                    let events = match code {
                        Code::ArrowDown => {
                            self.nudge(app, room, RoomVector::new(0, 8), room.floats.clone())
                        }
                        Code::ArrowUp => {
                            self.nudge(app, room, RoomVector::new(0, -8), room.floats.clone())
                        }
                        Code::ArrowRight => {
                            self.nudge(app, room, RoomVector::new(8, 0), room.floats.clone())
                        }
                        Code::ArrowLeft => {
                            self.nudge(app, room, RoomVector::new(-8, 0), room.floats.clone())
                        }
                        Code::KeyA if cx.modifiers == &Modifiers::CTRL => {
                            self.pending_selection = self.selectables_in(
                                app,
                                room,
                                app.current_layer,
                                RoomRect::new(
                                    RoomPoint::new(-1000000, -1000000),
                                    RoomSize::new(2000000, 2000000),
                                ),
                            );
                            self.confirm_selection(app)
                        }
                        Code::KeyC if cx.modifiers == &Modifiers::CTRL => {
                            self.clipboard_copy(app, room)
                        }
                        Code::KeyX if cx.modifiers == &Modifiers::CTRL => {
                            let mut result = self.clipboard_copy(app, room);
                            result.accumulate(self.delete_all(app, room));
                            result
                        }
                        Code::KeyV if cx.modifiers == &Modifiers::CTRL => {
                            if let Ok(s) = cx.get_clipboard() {
                                let app = cx.data().unwrap();
                                self.clipboard_paste(app, s)
                            } else {
                                AppEventStaging::default()
                            }
                        }
                        Code::Backspace | Code::Delete => self.delete_all(app, room),
                        _ => AppEventStaging::default(),
                    };
                    if events.is_empty() {
                        self.draw_phase = old_draw_phase;
                    }
                    events
                } else {
                    AppEventStaging::default()
                }
            }
            _ => AppEventStaging::default(),
        }
        .finalize(cx.data::<AppState>().unwrap(), self.draw_phase)
    }

    /*
    fn internal_event(&mut self, event: &AppInternalEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let Some(room) = app.current_room_ref() else { return vec![] };
        match event {
            AppInternalEvent::SelectMeEntity { id, trigger } => {
                self.current_selection
                    .insert(AppSelection::EntityBody(*id, *trigger));
                for (idx, _) in room.entity(*id, *trigger).unwrap().nodes.iter().enumerate() {
                    self.current_selection
                        .insert(AppSelection::EntityNode(*id, idx, *trigger));
                }
            }
            AppInternalEvent::SelectMeDecal { id, fg } => {
                self.current_selection.insert(AppSelection::Decal(*id, *fg));
            }
            _ => {}
        }
        self.notify_selection(app).finalize(app, self.draw_phase)
    }
     */

    fn switch_off(&mut self, app: &AppState, _cx: &EventContext) -> Vec<AppEvent> {
        let Some(room) = app.current_room_ref() else { return vec![] };
        self.clear_selection(app, &room.floats)
            .finalize(app, self.draw_phase)
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {
        let Some(room) = state.current_room_ref() else { return };
        canvas.save();
        canvas.translate(
            room.data.bounds.origin.x as f32,
            room.data.bounds.origin.y as f32,
        );
        // no scissor!

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = state
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos = (map_pos - room.data.bounds.origin).to_point().cast_unit();
        let room_pos = if state.config.snap {
            let tile_pos = point_room_to_tile(&room_pos);
            point_tile_to_room(&tile_pos)
        } else {
            room_pos
        };

        if let SelectionStatus::Selecting(ref_pos) = &self.status {
            let selection =
                rect_normalize(&RoomRect::new(*ref_pos, (room_pos - *ref_pos).to_size()));
            let mut path = vg::Path::new();
            path.rect(
                selection.min_x() as f32,
                selection.min_y() as f32,
                selection.width() as f32,
                selection.height() as f32,
            );
            canvas.stroke_path(
                &mut path,
                &vg::Paint::color(vg::Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        let mut path = vg::Path::new();
        for selectable in self
            .pending_selection
            .iter()
            .chain(state.map_tab_unwrap().current_selected.iter())
        {
            for rect in self.rects_of(state, room, *selectable) {
                path.rect(
                    rect.min_x() as f32,
                    rect.min_y() as f32,
                    rect.width() as f32,
                    rect.height() as f32,
                )
            }
        }

        canvas.fill_path(
            &mut path,
            &vg::Paint::color(vg::Color::rgba(255, 255, 0, 128)),
        );

        if self.status == SelectionStatus::None {
            if let Some(sel) = self.selectable_at(state, room, state.current_layer, room_pos) {
                if !state.map_tab_unwrap().current_selected.contains(&sel) {
                    let mut path = vg::Path::new();
                    for rect in self.rects_of(state, room, sel) {
                        path.rect(
                            rect.min_x() as f32,
                            rect.min_y() as f32,
                            rect.width() as f32,
                            rect.height() as f32,
                        );
                    }
                    canvas.fill_path(
                        &mut path,
                        &vg::Paint::color(vg::Color::rgba(100, 100, 255, 128)),
                    );
                }
            }
        }

        canvas.restore();
    }

    fn cursor(&self, cx: &mut EventContext) -> CursorIcon {
        let app = cx.data::<AppState>().unwrap();
        let Some(room) = app.current_room_ref() else { return CursorIcon::Default };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos = (map_pos - room.data.bounds.origin).to_point().cast_unit();
        // let tile_pos = point_room_to_tile(&room_pos);
        // let room_pos_snapped = point_tile_to_room(&tile_pos);
        // let room_pos = if state.snap { room_pos_snapped } else { room_pos };

        match &self.status {
            SelectionStatus::CouldStartDragging(_, _) | SelectionStatus::None => {
                self.can_resize(app, room, room_pos).to_cursor_icon()
            }
            SelectionStatus::Dragging(_) | SelectionStatus::Selecting(_) => CursorIcon::Default,
            SelectionStatus::Resizing(info) => info.side.to_cursor_icon(),
        }
    }
}

impl SelectionTool {
    #[must_use]
    fn confirm_selection(&mut self, app: &AppState) -> AppEventStaging {
        let mut result = AppEventStaging::default();
        result.push_ind(AppEvent::SelectObjects {
            tab: app.current_tab,
            selection: self.pending_selection.drain().collect(),
        });
        result
    }

    fn rects_of(
        &self,
        app: &AppState,
        room: &LevelState,
        selectable: AppSelection,
    ) -> Vec<RoomRect> {
        fn rects_of_layer<T: Copy + Eq>(
            layer: Option<&(Point2D<i32, TileSpace>, TileGrid<T>)>,
            nil: T,
        ) -> Vec<RoomRect> {
            if let Some((origin, grid)) = layer {
                rect_point_iter(TileRect::new(TilePoint::zero(), grid.size()), 1)
                    .filter_map(|pt| {
                        let tile = grid.get(pt);
                        if *tile.unwrap() != nil {
                            Some(RoomRect::new(
                                point_tile_to_room(&(*origin + pt.to_vector())),
                                RoomSize::new(8, 8),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        }
        match selectable {
            AppSelection::FgTile(pt) | AppSelection::BgTile(pt) | AppSelection::ObjectTile(pt) => {
                vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))]
            }
            AppSelection::FgFloat => rects_of_layer(room.floats.fg.as_ref(), '\0'),
            AppSelection::BgFloat => rects_of_layer(room.floats.bg.as_ref(), '\0'),
            AppSelection::ObjFloat => rects_of_layer(room.floats.obj.as_ref(), -2),
            AppSelection::EntityBody(id, trigger) => {
                if let Some(entity) = room.entity(id, trigger) {
                    let config = app
                        .current_palette_unwrap()
                        .get_entity_config(&entity.name, trigger);
                    let env = make_entity_env(entity);
                    config
                        .hitboxes
                        .initial_rects
                        .iter()
                        .filter_map(|r| r.evaluate_int(&env).ok())
                        .collect()
                } else {
                    vec![]
                }
            }
            AppSelection::EntityNode(id, node_idx, trigger) => {
                if let Some(entity) = room.entity(id, trigger) {
                    let config = app
                        .current_palette_unwrap()
                        .get_entity_config(&entity.name, trigger);
                    let env = make_node_env(entity, make_entity_env(entity), node_idx);
                    config
                        .hitboxes
                        .node_rects
                        .iter()
                        .filter_map(|r| r.evaluate_int(&env).ok())
                        .collect()
                } else {
                    vec![]
                }
            }
            AppSelection::Decal(id, fg) => {
                if let Some(decal) = room.decal(id, fg) {
                    let dim = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(decal))
                        .unwrap_or(Size2D::new(16, 16));
                    let size = dim
                        .cast()
                        .cast_unit()
                        .to_vector()
                        .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                        .cast()
                        .to_size()
                        .abs();
                    vec![Rect::new(RoomPoint::new(decal.x, decal.y) - size / 2, size)]
                } else {
                    vec![]
                }
            }
        }
    }

    fn selectable_at(
        &self,
        app: &AppState,
        room: &LevelState,
        layer: Layer,
        room_pos: RoomPoint,
    ) -> Option<AppSelection> {
        self.selectables_in(
            app,
            room,
            layer,
            RoomRect::new(room_pos, RoomSize::new(1, 1)),
        )
        .iter()
        .next()
        .cloned()
    }

    fn selectables_in(
        &self,
        app: &AppState,
        room: &LevelState,
        layer: Layer,
        room_rect: RoomRect,
    ) -> HashSet<AppSelection> {
        let room_rect = rect_normalize(&room_rect);
        let mut result = HashSet::new();
        let room_rect_cropped = room_rect.intersection(&RoomRect::new(
            RoomPoint::zero(),
            room.data.bounds.size.cast_unit(),
        ));

        if let Layer::FgDecals | Layer::All = layer {
            for (idx, decal) in room.data.fg_decals.iter().enumerate().rev() {
                room.cache_decal_idx(idx);
                let sel = AppSelection::Decal(decal.id, true);
                if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                    result.insert(sel);
                }
            }
        }
        if let Layer::ObjectTiles | Layer::All = layer {
            if let Some(room_rect_cropped) = room_rect_cropped {
                for tile_pos_unaligned in rect_point_iter(room_rect_cropped, 8) {
                    let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                    if room
                        .data
                        .object_tiles
                        .get(tile_pos)
                        .map_or(false, |&tile| tile != -1)
                    {
                        result.insert(AppSelection::ObjectTile(tile_pos));
                    }
                }
            }
        }
        if let Layer::FgTiles | Layer::All = layer {
            if let Some(room_rect_cropped) = room_rect_cropped {
                for tile_pos_unaligned in rect_point_iter(room_rect_cropped, 8) {
                    let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                    if room.tile(tile_pos, true).unwrap_or('0') != '0' {
                        result.insert(AppSelection::FgTile(tile_pos));
                    }
                }
            }
        }
        if layer == Layer::Entities || layer == Layer::All {
            for (idx, entity) in room.data.entities.iter().enumerate().rev() {
                room.cache_entity_idx(idx);
                for node_idx in 0..entity.nodes.len() {
                    let sel = AppSelection::EntityNode(entity.id, node_idx, false);
                    if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                        result.insert(sel);
                    }
                }
                let sel = AppSelection::EntityBody(entity.id, false);
                if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                    result.insert(sel);
                }
            }
        }
        if layer == Layer::Triggers || layer == Layer::All {
            for (idx, entity) in room.data.triggers.iter().enumerate().rev() {
                room.cache_entity_idx(idx);
                for node_idx in 0..entity.nodes.len() {
                    let sel = AppSelection::EntityNode(entity.id, node_idx, true);
                    if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                        result.insert(sel);
                    }
                }
                let sel = AppSelection::EntityBody(entity.id, true);
                if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                    result.insert(sel);
                }
            }
        }
        if layer == Layer::BgDecals || layer == Layer::All {
            for (idx, decal) in room.data.bg_decals.iter().enumerate().rev() {
                room.cache_decal_idx(idx);
                let sel = AppSelection::Decal(decal.id, false);
                if intersects_any(&self.rects_of(app, room, sel), &room_rect) {
                    result.insert(sel);
                }
            }
        }
        if layer == Layer::BgTiles || layer == Layer::All {
            if let Some(room_rect_cropped) = room_rect_cropped {
                for tile_pos_unaligned in rect_point_iter(room_rect_cropped, 8) {
                    let tile_pos = point_room_to_tile(&tile_pos_unaligned);
                    if room.tile(tile_pos, false).unwrap_or('0') != '0' {
                        result.insert(AppSelection::BgTile(tile_pos));
                    }
                }
            }
        }
        if let Some((pt, grid)) = &room.floats.bg {
            if let Some(float_rect_cropped) =
                rect_tile_to_room(&TileRect::new(*pt, grid.size())).intersection(&room_rect)
            {
                for tile_pos in
                    rect_point_iter(float_rect_cropped, 8).map(|point| point_room_to_tile(&point))
                {
                    if grid.get(tile_pos - pt.to_vector()).copied().unwrap_or('\0') != '\0' {
                        result.insert(AppSelection::BgFloat);
                        break;
                    }
                }
            }
        }

        result
    }

    #[must_use]
    fn clear_selection(&mut self, app: &AppState, floats: &LevelFloatState) -> AppEventStaging {
        let mut result = AppEventStaging::default();
        result.push_ind(AppEvent::ClearSelection {
            tab: app.current_tab,
        });
        for action in drop_float(floats) {
            result.push_room(action);
        }
        result
    }

    /// This function interprets nudge as relative to the reference positions in Dragging mode and
    /// relative to the current position in other modes.
    #[must_use]
    fn nudge(
        &mut self,
        app: &AppState,
        room: &LevelState,
        nudge: RoomVector,
        floats: LevelFloatState,
    ) -> AppEventStaging {
        let mut result = AppEventStaging::default();
        let mut result_floats = floats;

        let dragging = if let SelectionStatus::Dragging(dragging) = &self.status {
            Some(dragging)
        } else {
            // edge case: if we're not dragging, we didn't call begin_dragging, so there might not be any floats floated
            // fix that
            let (e, f) = self.float_tiles(room, app.current_tab, app.map_tab_unwrap());
            result.accumulate(e);
            add_floats_to_floats(&mut result_floats, &f);
            None
        };
        let mut entity_updates = HashMap::new();
        let mut trigger_updates = HashMap::new();
        for selected in app.map_tab_unwrap().current_selected.iter() {
            match selected {
                AppSelection::FgTile(_)
                | AppSelection::BgTile(_)
                | AppSelection::ObjectTile(_)
                | AppSelection::FgFloat
                | AppSelection::BgFloat
                | AppSelection::ObjFloat => {}
                AppSelection::EntityBody(id, trigger) => {
                    let updates = if *trigger {
                        &mut trigger_updates
                    } else {
                        &mut entity_updates
                    };
                    let e = updates
                        .entry(*id)
                        .or_insert_with(|| room.entity(*id, *trigger).unwrap().clone()); // one of the riskier unwraps I've written
                    let base = dragging
                        .map(|d| d.selection_reference_points[selected])
                        .unwrap_or_else(|| RoomPoint::new(e.x, e.y));
                    e.x = base.x + nudge.x;
                    e.y = base.y + nudge.y;
                }
                AppSelection::EntityNode(id, node_idx, trigger) => {
                    let updates = if *trigger {
                        &mut trigger_updates
                    } else {
                        &mut entity_updates
                    };
                    let e = updates
                        .entry(*id)
                        .or_insert_with(|| room.entity(*id, *trigger).unwrap().clone());
                    let base = dragging
                        .map(|d| d.selection_reference_points[selected])
                        .unwrap_or_else(|| {
                            RoomPoint::new(e.nodes[*node_idx].x, e.nodes[*node_idx].y)
                        });
                    e.nodes[*node_idx] = Node {
                        x: base.x + nudge.x,
                        y: base.y + nudge.y,
                    };
                }
                AppSelection::Decal(id, fg) => {
                    if let Some(decal) = room.decal(*id, *fg) {
                        let mut decal = decal.clone();
                        let base = dragging
                            .map(|d| d.selection_reference_points[selected])
                            .unwrap_or_else(|| RoomPoint::new(decal.x, decal.y));
                        let new = base + nudge;
                        decal.x = new.x;
                        decal.y = new.y;
                        result.push_room(RoomAction::DecalUpdate {
                            fg: *fg,
                            decal: Box::new(decal),
                        });
                    }
                }
            }
        }
        if let Some(float) = result_floats.fg.as_mut() {
            let base = dragging
                .map(|d| d.selection_reference_points[&AppSelection::FgFloat])
                .unwrap_or_else(|| point_tile_to_room(&float.0));
            float.0 = point_room_to_tile(&(base + nudge));
        }
        if let Some(float) = result_floats.bg.as_mut() {
            let base = dragging
                .map(|d| d.selection_reference_points[&AppSelection::BgFloat])
                .unwrap_or_else(|| point_tile_to_room(&float.0));
            float.0 = point_room_to_tile(&(base + nudge));
        }
        if let Some(float) = result_floats.obj.as_mut() {
            let base = dragging
                .map(|d| d.selection_reference_points[&AppSelection::ObjFloat])
                .unwrap_or_else(|| point_tile_to_room(&float.0));
            float.0 = point_room_to_tile(&(base + nudge));
        }

        for entity in entity_updates.into_values() {
            result.push_room(RoomAction::EntityUpdate {
                entity: Box::new(entity),
                trigger: false,
            });
        }
        for entity in trigger_updates.into_values() {
            result.push_room(RoomAction::EntityUpdate {
                entity: Box::new(entity),
                trigger: true,
            });
        }
        result.accumulate(floats_to_events(result_floats));
        result
    }

    #[must_use]
    fn resize(&mut self, app: &AppState, room: &LevelState, resize: RoomVector) -> AppEventStaging {
        let mut result = AppEventStaging::default();

        let dragging = if let SelectionStatus::Resizing(dragging) = &self.status {
            Some(dragging)
        } else {
            None
        };
        let side = if let Some(dragging) = dragging {
            dragging.side
        } else {
            ResizeSide::TopLeft
        };
        let pos_vec = RoomVector::new(
            if side.is_left() { resize.x } else { 0 },
            if side.is_top() { resize.y } else { 0 },
        );
        let size_vec = RoomVector::new(
            if side.is_left() {
                -resize.x
            } else if side.is_right() {
                resize.x
            } else {
                0
            },
            if side.is_top() {
                -resize.y
            } else if side.is_bottom() {
                resize.y
            } else {
                0
            },
        );
        for sel in app.map_tab_unwrap().current_selected.iter() {
            match sel {
                AppSelection::FgTile(_)
                | AppSelection::BgTile(_)
                | AppSelection::ObjectTile(_)
                | AppSelection::EntityNode(_, _, _)
                | AppSelection::FgFloat
                | AppSelection::BgFloat
                | AppSelection::ObjFloat => {
                    unreachable!()
                }
                AppSelection::EntityBody(id, trigger) => {
                    let mut e = room.entity(*id, *trigger).unwrap().clone();
                    let config = app
                        .current_palette_unwrap()
                        .get_entity_config(e.name.as_str(), false);
                    let start_rect = dragging
                        .map(|d| *d.selection_reference_sizes.get(sel).unwrap())
                        .unwrap_or_else(|| {
                            RoomRect::new(
                                RoomPoint::new(e.x, e.y),
                                RoomSize::new(e.width as i32, e.height as i32),
                            )
                        });
                    let new_rect = RoomRect::new(
                        start_rect.origin + pos_vec,
                        start_rect.size + size_vec.to_size(),
                    );
                    e.x = new_rect.origin.x;
                    e.y = new_rect.origin.y;
                    e.width = new_rect.size.width.max(config.minimum_size_x as i32) as u32;
                    e.height = new_rect.size.height.max(config.minimum_size_y as i32) as u32;
                    result.push_room(RoomAction::EntityUpdate {
                        entity: Box::new(e),
                        trigger: *trigger,
                    });
                }
                AppSelection::Decal(id, fg) => {
                    let mut d = room.decal(*id, *fg).unwrap().clone();
                    let dim = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(&d))
                        .unwrap_or(Size2D::new(16, 16));
                    let texture_size = dim.cast().cast_unit();
                    let start_rect = dragging
                        .map(|d| *d.selection_reference_sizes.get(sel).unwrap())
                        .unwrap_or_else(|| {
                            let size = texture_size
                                .to_vector()
                                .component_mul(Vector2D::new(d.scale_x, d.scale_y))
                                .cast()
                                .to_size();
                            RoomRect::new(RoomPoint::new(d.x, d.y) - size / 2, size)
                        });
                    let new_rect = RoomRect::new(
                        start_rect.origin + pos_vec,
                        start_rect.size + size_vec.to_size(),
                    );
                    let new_stretch = new_rect
                        .size
                        .to_vector()
                        .cast::<f32>()
                        .component_div(texture_size.to_vector().cast());
                    d.x = new_rect.center().x;
                    d.y = new_rect.center().y;
                    d.scale_x = new_stretch.x;
                    d.scale_y = new_stretch.y;
                    result.push_room(RoomAction::DecalUpdate {
                        fg: *fg,
                        decal: Box::new(d),
                    })
                }
            }
        }

        result
    }

    #[must_use]
    fn float_tiles(
        &mut self,
        room: &LevelState,
        tabid: usize,
        tab: &MapTab,
    ) -> (AppEventStaging, LevelFloatState) {
        // TODO: do this in an efficient order to avoid frequent reallocations of the float
        let mut result = AppEventStaging::default();
        let mut floats = LevelFloatState::default();
        let mut sel_del = HashSet::new();
        for sel in tab.current_selected.iter().cloned().collect::<Vec<_>>() {
            match sel {
                AppSelection::FgTile(pt) => {
                    add_to_float(&mut floats.fg, pt, room.data.solids.get(pt), '\0');
                    result.push_room(RoomAction::TileUpdate {
                        fg: true,
                        offset: pt,
                        data: TileGrid {
                            tiles: vec!['0'],
                            stride: 1,
                        },
                    });
                    sel_del.insert(sel);
                    continue;
                }
                AppSelection::BgTile(pt) => {
                    add_to_float(&mut floats.bg, pt, room.data.bg.get(pt), '\0');
                    result.push_room(RoomAction::TileUpdate {
                        fg: false,
                        offset: pt,
                        data: TileGrid {
                            tiles: vec!['0'],
                            stride: 1,
                        },
                    });
                    sel_del.insert(sel);
                    continue;
                }
                AppSelection::ObjectTile(pt) => {
                    add_to_float(&mut floats.obj, pt, room.data.object_tiles.get(pt), -2);
                    result.push_room(RoomAction::ObjectTileUpdate {
                        offset: pt,
                        data: TileGrid {
                            tiles: vec![-1],
                            stride: 1,
                        },
                    });
                    sel_del.insert(sel);
                    continue;
                }
                _ => {}
            }
        }

        result.push_ind(AppEvent::DeselectObjects {
            tab: tabid,
            selection: sel_del,
        });
        (result, floats)
    }

    fn elaborate_nodes(&mut self, app: &AppState, room: &LevelState) -> AppEventStaging {
        let mut result = AppEventStaging::default();
        let mut select = HashSet::new();
        for sel in app
            .map_tab_unwrap()
            .current_selected
            .iter()
            .cloned()
            .collect::<Vec<AppSelection>>()
        // TODO why did I collect here & elsewhere
        {
            if let AppSelection::EntityNode(id, _, trigger) = sel {
                select.insert(AppSelection::EntityBody(id, trigger));
                for (idx, _) in room.entity(id, trigger).unwrap().nodes.iter().enumerate() {
                    select.insert(AppSelection::EntityNode(id, idx, trigger));
                }
            }
        }
        result.push_ind(AppEvent::SelectObjects {
            tab: app.current_tab,
            selection: select,
        });
        result
    }

    fn can_resize(&self, app: &AppState, room: &LevelState, pointer: RoomPoint) -> ResizeSide {
        // get which side of the rectangle we're on
        let mut side = ResizeSide::None;
        'outer: for sel in app.map_tab_unwrap().current_selected.iter() {
            for rect in self.rects_of(app, room, *sel) {
                if rect.contains(pointer) {
                    let smaller_rect = rect.inflate(-2, -2);
                    let at_top = pointer.y < smaller_rect.min_y();
                    let at_bottom = pointer.y >= smaller_rect.max_y();
                    let at_left = pointer.x < smaller_rect.min_x();
                    let at_right = pointer.x >= smaller_rect.max_x();

                    side = ResizeSide::from_sides(at_top, at_bottom, at_left, at_right);
                    break 'outer;
                }
            }
        }

        // filter that rectangle by which sides are appropriate for the current selection
        if side != ResizeSide::None {
            for sel in app.map_tab_unwrap().current_selected.iter() {
                side = match sel {
                    AppSelection::FgTile(_)
                    | AppSelection::BgTile(_)
                    | AppSelection::ObjectTile(_) => ResizeSide::None,
                    AppSelection::FgFloat | AppSelection::BgFloat | AppSelection::ObjFloat => {
                        ResizeSide::None
                    }
                    AppSelection::EntityBody(id, trigger) => {
                        if let Some(entity) = room.entity(*id, *trigger) {
                            let config = app
                                .current_palette_unwrap()
                                .get_entity_config(&entity.name, *trigger);
                            if !config.resizable_x {
                                side = side.filter_out_left_right();
                            }
                            if !config.resizable_y {
                                side = side.filter_out_top_bottom();
                            }
                        }
                        side
                    }
                    AppSelection::EntityNode(_, _, _) => ResizeSide::None,
                    AppSelection::Decal(_, _) => side,
                };

                if side == ResizeSide::None {
                    break;
                }
            }
        }

        side
    }

    #[must_use]
    fn begin_dragging(
        &mut self,
        app: &AppState,
        room: &LevelState,
        pointer_reference_point: RoomPoint,
        pointer_reference_point_unsnapped: RoomPoint,
    ) -> (AppEventStaging, LevelFloatState) {
        // Offload all fg/bg selections into the floats
        let (result, floats) = self.float_tiles(room, app.current_tab, app.map_tab_unwrap());
        let mut result_floats = room.floats.clone();
        add_floats_to_floats(&mut result_floats, &floats);

        let side = self.can_resize(app, room, pointer_reference_point_unsnapped);
        if side != ResizeSide::None {
            // collect reference sizes
            let selection_reference_sizes = app
                .map_tab_unwrap()
                .current_selected
                .iter()
                .filter_map(|sel| match sel {
                    AppSelection::FgTile(_)
                    | AppSelection::BgTile(_)
                    | AppSelection::ObjectTile(_)
                    | AppSelection::FgFloat
                    | AppSelection::BgFloat
                    | AppSelection::ObjFloat
                    | AppSelection::EntityNode(_, _, _) => unreachable!(),
                    AppSelection::EntityBody(id, trigger) => {
                        room.entity(*id, *trigger).map(|entity| {
                            (
                                *sel,
                                RoomRect::new(
                                    RoomPoint::new(entity.x, entity.y),
                                    RoomSize::new(entity.width as i32, entity.height as i32),
                                ),
                            )
                        })
                    }
                    AppSelection::Decal(id, fg) => {
                        if let Some(decal) = room.decal(*id, *fg) {
                            let dim = app
                                .current_palette_unwrap()
                                .gameplay_atlas
                                .sprite_dimensions(&decal_texture(decal))
                                .unwrap_or(Size2D::new(16, 16));
                            let size = dim
                                .cast()
                                .cast_unit()
                                .to_vector()
                                .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                                .cast()
                                .to_size();
                            Some((
                                *sel,
                                RoomRect::new(RoomPoint::new(decal.x, decal.y) - size / 2, size),
                            ))
                        } else {
                            None
                        }
                    }
                })
                .collect::<HashMap<_, _>>();
            self.status = SelectionStatus::Resizing(ResizingStatus {
                pointer_reference_point,
                selection_reference_sizes,
                side,
            });
        } else {
            // collect reference points
            let mut selection_reference_points = app
                .map_tab_unwrap()
                .current_selected
                .iter()
                .filter_map(|sel| {
                    match sel {
                        AppSelection::FgTile(_)
                        | AppSelection::BgTile(_)
                        | AppSelection::ObjectTile(_)
                        | AppSelection::FgFloat
                        | AppSelection::BgFloat
                        | AppSelection::ObjFloat => None,
                        AppSelection::EntityBody(id, trigger) => {
                            let e = room.entity(*id, *trigger).unwrap();
                            Some(RoomPoint::new(e.x, e.y))
                        }
                        AppSelection::EntityNode(id, node_idx, trigger) => {
                            let e = room.entity(*id, *trigger).unwrap();
                            Some(RoomPoint::new(e.nodes[*node_idx].x, e.nodes[*node_idx].y))
                        }
                        AppSelection::Decal(id, fg) => {
                            let d = room.decal(*id, *fg).unwrap();
                            Some(RoomPoint::new(d.x, d.y))
                        }
                    }
                    .map(|pt| (*sel, pt))
                })
                .collect::<HashMap<AppSelection, RoomPoint>>();
            if let Some(float) = &result_floats.fg {
                selection_reference_points
                    .insert(AppSelection::FgFloat, point_tile_to_room(&float.0));
            }
            if let Some(float) = &result_floats.bg {
                selection_reference_points
                    .insert(AppSelection::BgFloat, point_tile_to_room(&float.0));
            }
            if let Some(float) = &result_floats.obj {
                selection_reference_points
                    .insert(AppSelection::ObjFloat, point_tile_to_room(&float.0));
            }

            // here's your status!
            self.status = SelectionStatus::Dragging(DraggingStatus {
                pointer_reference_point,
                selection_reference_points,
            });
        }

        (result, result_floats)
    }

    #[must_use]
    fn delete_all(&mut self, app: &AppState, room: &LevelState) -> AppEventStaging {
        let (mut result, _) = self.float_tiles(room, app.current_tab, app.map_tab_unwrap());

        let mut entity_nodes_removed = HashMap::new();
        let mut trigger_nodes_removed = HashMap::new();
        let mut entities_removed = HashSet::new();
        let mut triggers_removed = HashSet::new();
        for sel in app.map_tab_unwrap().current_selected.iter() {
            match sel {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) | AppSelection::ObjectTile(_) => {
                }
                AppSelection::FgFloat | AppSelection::BgFloat | AppSelection::ObjFloat => {}
                AppSelection::EntityBody(id, trigger) => {
                    result.push_room(RoomAction::EntityRemove {
                        id: *id,
                        trigger: *trigger,
                    });
                    if *trigger {
                        &mut triggers_removed
                    } else {
                        &mut entities_removed
                    }
                    .insert(id);
                }
                AppSelection::EntityNode(id, node_idx, trigger) => {
                    let e = if *trigger {
                        &mut trigger_nodes_removed
                    } else {
                        &mut entity_nodes_removed
                    }
                    .entry(id)
                    .or_insert_with(HashSet::new);
                    e.insert(node_idx);
                }
                AppSelection::Decal(id, fg) => {
                    result.push_room(RoomAction::DecalRemove { id: *id, fg: *fg });
                }
            }
        }

        result.push_room(RoomAction::TileFloatSet {
            fg: true,
            float: None,
        });
        result.push_room(RoomAction::TileFloatSet {
            fg: false,
            float: None,
        });
        result.push_room(RoomAction::ObjFloatSet { float: None });

        for (id, indices) in entity_nodes_removed {
            if !entities_removed.contains(&id) {
                if let Some(entity) = room.entity(*id, false) {
                    let mut entity = entity.clone();
                    for idx in (0..entity.nodes.len()).rev() {
                        if indices.contains(&idx) {
                            entity.nodes.remove(idx);
                        }
                    }
                    result.push_room(RoomAction::EntityUpdate {
                        entity: Box::new(entity),
                        trigger: false,
                    });
                }
            }
        }

        for (id, indices) in trigger_nodes_removed {
            if !triggers_removed.contains(&id) {
                if let Some(entity) = room.entity(*id, true) {
                    let mut entity = entity.clone();
                    for idx in (0..entity.nodes.len()).rev() {
                        if indices.contains(&idx) {
                            entity.nodes.remove(idx);
                        }
                    }
                    result.push_room(RoomAction::EntityUpdate {
                        entity: Box::new(entity),
                        trigger: true,
                    });
                }
            }
        }

        result.push_ind(AppEvent::ClearSelection {
            tab: app.current_tab,
        });
        result
    }

    pub fn clipboard_copy(&mut self, app: &AppState, room: &LevelState) -> AppEventStaging {
        let (mut result, float) = self.float_tiles(room, app.current_tab, app.map_tab_unwrap());
        let mut result_float = room.floats.clone();
        add_floats_to_floats(&mut result_float, &float);

        result.accumulate(self.elaborate_nodes(app, room));

        let mut clipboard_data: Vec<AppInRoomSelectable> = vec![];
        for sel in app.map_tab_unwrap().current_selected.iter() {
            match sel {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) | AppSelection::ObjectTile(_) => {
                }
                AppSelection::FgFloat | AppSelection::BgFloat | AppSelection::ObjFloat => {}
                AppSelection::EntityNode(_, _, _) => {}
                AppSelection::EntityBody(id, trigger) => {
                    clipboard_data.push(AppInRoomSelectable::Entity(
                        room.entity(*id, *trigger).unwrap().clone(),
                        *trigger,
                    ));
                }
                AppSelection::Decal(id, fg) => {
                    clipboard_data.push(AppInRoomSelectable::Decal(
                        room.decal(*id, *fg).unwrap().clone(),
                        *fg,
                    ));
                }
            }
        }
        if let Some((pt, grid)) = result_float.fg.take() {
            clipboard_data.push(AppInRoomSelectable::FgTiles(pt, grid));
        }
        if let Some((pt, grid)) = result_float.bg.take() {
            clipboard_data.push(AppInRoomSelectable::BgTiles(pt, grid));
        }
        if let Some((pt, grid)) = result_float.obj.take() {
            clipboard_data.push(AppInRoomSelectable::ObjectTiles(pt, grid));
        }
        let s = serde_yaml::to_string(&AppSelectable::InRoom(clipboard_data))
            .expect("Failed to serialize copied data");
        result.push_ind(AppEvent::SetClipboard { contents: s });
        result.accumulate(floats_to_events(result_float));
        result
    }

    pub fn clipboard_paste(&mut self, app: &AppState, data: String) -> AppEventStaging {
        let mut result = self.clear_selection(app, &app.current_room_ref().unwrap().floats);
        let mut result_float = LevelFloatState::default();

        let Ok(AppSelectable::InRoom(clipboard_data)) = serde_yaml::from_str(&data) else { return result };
        let Some(room) = app.current_room_ref() else { return result };
        if clipboard_data.is_empty() {
            return result;
        }
        let mut min_tile = TilePoint::new(i32::MAX, i32::MAX);
        let mut max_tile = TilePoint::new(i32::MIN, i32::MIN);
        for obj in &clipboard_data {
            match obj {
                AppInRoomSelectable::FgTiles(point, float)
                | AppInRoomSelectable::BgTiles(point, float) => {
                    min_tile = min_tile.min(*point);
                    max_tile = max_tile.max(*point + float.size());
                }
                AppInRoomSelectable::ObjectTiles(point, float) => {
                    min_tile = min_tile.min(*point);
                    max_tile = max_tile.max(*point + float.size());
                }
                AppInRoomSelectable::Entity(entity, trigger) => {
                    let config = app
                        .current_palette_unwrap()
                        .get_entity_config(&entity.name, *trigger);
                    let env = make_entity_env(entity);
                    for hitbox in config
                        .hitboxes
                        .initial_rects
                        .iter()
                        .filter_map(|r| r.evaluate_int(&env).ok())
                    {
                        min_tile = min_tile.min(point_room_to_tile(&hitbox.min()));
                        max_tile = max_tile.max(point_room_to_tile(&hitbox.max()));
                    }
                }
                AppInRoomSelectable::Decal(decal, _) => {
                    let dim = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(decal))
                        .unwrap_or(Size2D::new(16, 16));
                    let size = dim
                        .cast()
                        .cast_unit()
                        .to_vector()
                        .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                        .cast()
                        .to_size()
                        .abs();
                    let hitbox = Rect::new(RoomPoint::new(decal.x, decal.y) - size / 2, size);
                    min_tile = min_tile.min(point_room_to_tile(&hitbox.min()));
                    max_tile = max_tile.max(point_room_to_tile(&hitbox.max()));
                }
            }
        }
        let center = (min_tile.to_vector() + max_tile.to_vector()) / 2;
        let real_center =
            (size_room_to_tile(&room.data.bounds.size.cast_unit::<RoomSpace>()) / 2).to_vector();
        let offset = real_center - center;
        for obj in clipboard_data {
            match obj {
                AppInRoomSelectable::FgTiles(point, float) => {
                    result_float.fg = Some((point + offset, float));
                }
                AppInRoomSelectable::BgTiles(point, float) => {
                    result_float.bg = Some((point + offset, float));
                }
                AppInRoomSelectable::ObjectTiles(point, float) => {
                    result_float.obj = Some((point + offset, float));
                }
                AppInRoomSelectable::Entity(mut entity, trigger) => {
                    entity.x += vector_tile_to_room(&offset).x;
                    entity.y += vector_tile_to_room(&offset).y;
                    for node in &mut entity.nodes {
                        node.x += vector_tile_to_room(&offset).x;
                        node.y += vector_tile_to_room(&offset).y;
                    }
                    result.push_room(RoomAction::EntityAdd {
                        entity: Box::new(entity),
                        trigger,
                        genid: true,
                    });
                }
                AppInRoomSelectable::Decal(mut decal, fg) => {
                    decal.x += vector_tile_to_room(&offset).x;
                    decal.y += vector_tile_to_room(&offset).y;
                    result.push_room(RoomAction::DecalAdd {
                        decal: Box::new(decal),
                        fg,
                        genid: true,
                    });
                }
            }
        }
        result.accumulate(floats_to_events(result_float));
        result
    }
}

// oh would it were that rust iterators weren't a fucking pain to write
fn intersects_any(haystack: &[RoomRect], needle: &RoomRect) -> bool {
    for hay in haystack {
        if hay.intersects(needle) {
            return true;
        }
    }
    false
}
pub(crate) fn add_to_float<T: Copy>(
    float: &mut Option<(TilePoint, TileGrid<T>)>,
    pt: TilePoint,
    src: Option<&T>,
    filler: T,
) {
    if let Some(ch) = src {
        let mut default = (
            pt,
            TileGrid {
                tiles: vec![],
                stride: 1,
            },
        );
        let (old_origin, old_dat) = float.as_mut().unwrap_or(&mut default);
        let old_size = TileVector::new(
            old_dat.stride as i32,
            (old_dat.tiles.len() / old_dat.stride) as i32,
        );
        let old_supnum = *old_origin + old_size;

        let new_origin = old_origin.min(pt);
        let new_supnum = old_supnum.max(pt + TileVector::new(1, 1));
        let new_size = new_supnum - new_origin;

        let new_dat = if new_size != old_size {
            let mut new_dat = TileGrid {
                tiles: vec![filler; (new_size.x * new_size.y) as usize],
                stride: new_size.x as usize,
            };
            let movement = *old_origin - new_origin;
            let dest_start_offset = movement.x + movement.y * new_size.x;
            for line in 0..old_size.y {
                let src = &old_dat.tiles[(line * old_size.x) as usize..][..old_size.x as usize];
                new_dat.tiles[(dest_start_offset + line * new_size.x) as usize..]
                    [..old_size.x as usize]
                    .clone_from_slice(src);
            }
            *float = Some((new_origin, new_dat));
            &mut float.as_mut().unwrap().1
        } else {
            old_dat
        };

        let movement = pt - new_origin;
        new_dat.tiles[(movement.x + movement.y * new_size.x) as usize] = *ch;
    }
}

pub(crate) fn add_float_to_float<T: Copy + PartialEq>(
    float: &mut Option<(TilePoint, TileGrid<T>)>,
    src_pt: TilePoint,
    src_grid: &TileGrid<T>,
    filler: T,
) {
    // TODO do this in an efficient order
    for pt in rect_point_iter(TileRect::new(src_pt, src_grid.size()), 1) {
        let tile = src_grid.get((pt - src_pt).to_point());
        if tile != Some(&filler) {
            add_to_float(float, pt, tile, filler);
        }
    }
}

fn add_floats_to_floats(floats: &mut LevelFloatState, src_grid: &LevelFloatState) {
    if let Some((pt, grid)) = &src_grid.fg {
        add_float_to_float(&mut floats.fg, *pt, grid, '\0');
    }
    if let Some((pt, grid)) = &src_grid.bg {
        add_float_to_float(&mut floats.bg, *pt, grid, '\0');
    }
    if let Some((pt, grid)) = &src_grid.obj {
        add_float_to_float(&mut floats.obj, *pt, grid, -2);
    }
}

fn floats_to_events(floats: LevelFloatState) -> AppEventStaging {
    let mut result = AppEventStaging::default();
    result.push_room(RoomAction::TileFloatSet {
        fg: true,
        float: floats.fg,
    });
    result.push_room(RoomAction::TileFloatSet {
        fg: false,
        float: floats.bg,
    });
    result.push_room(RoomAction::ObjFloatSet { float: floats.obj });
    result
}

pub(crate) fn drop_float(floats: &LevelFloatState) -> Vec<RoomAction> {
    let mut result = vec![];
    if let Some((offset, data)) = &floats.fg {
        result.push(RoomAction::TileUpdate {
            fg: true,
            offset: *offset,
            data: data.clone(),
        });
        result.push(RoomAction::TileFloatSet {
            fg: true,
            float: None,
        });
    }
    if let Some((offset, data)) = &floats.bg {
        result.push(RoomAction::TileUpdate {
            fg: false,
            offset: *offset,
            data: data.clone(),
        });
        result.push(RoomAction::TileFloatSet {
            fg: false,
            float: None,
        });
    }
    if let Some((offset, data)) = &floats.obj {
        result.push(RoomAction::ObjectTileUpdate {
            offset: *offset,
            data: data.clone(),
        });
        result.push(RoomAction::ObjFloatSet { float: None });
    }
    result
}
