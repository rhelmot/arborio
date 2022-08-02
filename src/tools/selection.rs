use std::collections::{HashMap, HashSet};
use vizia::vg;
use vizia::*;

use crate::app_state::{
    AppEvent, AppInRoomSelectable, AppInternalEvent, AppSelectable, AppSelection, AppState,
    EventPhase, Layer, MapEvent, RoomEvent,
};
use crate::autotiler::{TextureTile, TileReference};
use crate::map_struct::{CelesteMapLevel, Node};
use crate::tools::{generic_nav, Tool};
use crate::units::*;
use crate::widgets::editor::decal_texture;

pub struct SelectionTool {
    current_selection: HashSet<AppSelection>,
    pending_selection: HashSet<AppSelection>,
    fg_float: Option<(TilePoint, TileGrid<char>)>,
    bg_float: Option<(TilePoint, TileGrid<char>)>,
    obj_float: Option<(TilePoint, TileGrid<i32>)>,

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
    fg_float_reference_point: Option<RoomPoint>,
    bg_float_reference_point: Option<RoomPoint>,
    obj_float_reference_point: Option<RoomPoint>,
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
    pub(crate) fn is_top(&self) -> bool {
        matches!(self, Self::Top | Self::TopLeft | Self::TopRight)
    }

    pub(crate) fn is_bottom(&self) -> bool {
        matches!(self, Self::Bottom | Self::BottomLeft | Self::BottomRight)
    }

    pub(crate) fn is_left(&self) -> bool {
        matches!(self, Self::Left | Self::TopLeft | Self::BottomLeft)
    }

    pub(crate) fn is_right(&self) -> bool {
        matches!(self, Self::Right | Self::TopRight | Self::BottomRight)
    }

    pub(crate) fn from_sides(at_top: bool, at_bottom: bool, at_left: bool, at_right: bool) -> Self {
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

    pub(crate) fn to_cursor_icon(self) -> CursorIcon {
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

pub struct AppEventStaging(Vec<AppEvent>, Vec<MapEvent>, Vec<RoomEvent>);

#[allow(unused)] // this guy is in a trial period. if we like it we might move it to app_state
impl AppEventStaging {
    pub fn new() -> Self {
        Self(vec![], vec![], vec![])
    }

    pub fn accumulate(&mut self, other: Self) {
        self.0.extend(other.0);
        self.1.extend(other.1);
        self.2.extend(other.2);
    }

    pub fn push_ind(&mut self, event: AppEvent) {
        self.0.push(event);
    }

    pub fn push_map(&mut self, event: MapEvent) {
        self.1.push(event);
    }

    pub fn push_room(&mut self, event: RoomEvent) {
        self.2.push(event);
    }

    pub fn finalize(mut self, app: &AppState, draw_phase: EventPhase) -> Vec<AppEvent> {
        self.1
            .extend(self.2.into_iter().map(|r| MapEvent::RoomEvent {
                idx: app.map_tab_unwrap().current_room,
                event: r,
            }));
        if !self.1.is_empty() {
            self.0.push(app.batch_event(self.1, draw_phase));
        }
        self.0
    }

    pub fn finalize_unique(self, app: &AppState) -> Vec<AppEvent> {
        self.finalize(app, EventPhase::new())
    }
}

impl SelectionTool {
    pub fn new(app: &AppState) -> Self {
        let selection = app
            .map_tab_unwrap()
            .current_selected
            .iter()
            .cloned()
            .collect();
        Self {
            current_selection: selection,
            pending_selection: HashSet::new(),
            fg_float: None,
            bg_float: None,
            obj_float: None,
            status: SelectionStatus::None,
            draw_phase: EventPhase::null(),
        }
    }
}

impl Tool for SelectionTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut Context) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let nav_events = generic_nav(event, app, cx, true);
        if !nav_events.is_empty() {
            return nav_events;
        }

        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return vec![];
        };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos_unsnapped = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos_unsnapped);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if app.snap {
            room_pos_snapped
        } else {
            room_pos_unsnapped
        };

        match event {
            WindowEvent::MouseUp(MouseButton::Left) => {
                let events = if let SelectionStatus::Selecting(_) = self.status {
                    self.confirm_selection(app)
                } else {
                    AppEventStaging::new()
                };
                self.status = SelectionStatus::None;
                events
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                if self.status == SelectionStatus::None {
                    self.draw_phase = EventPhase::new();
                    let got = self.selectable_at(app, room, app.current_layer, room_pos_unsnapped);
                    if self.touches_float(room_pos)
                        || (got.is_some() && self.current_selection.contains(&got.unwrap()))
                    {
                        self.status =
                            SelectionStatus::CouldStartDragging(room_pos, room_pos_unsnapped);
                        AppEventStaging::new()
                    } else {
                        self.status = SelectionStatus::Selecting(room_pos);
                        if let Some(g) = got {
                            self.pending_selection = HashSet::from([g]);
                        }
                        if !cx.modifiers.contains(Modifiers::SHIFT) {
                            self.clear_selection(app)
                        } else {
                            AppEventStaging::new()
                        }
                    }
                } else {
                    AppEventStaging::new()
                }
            }
            WindowEvent::MouseMove(..) => {
                let mut events = if let SelectionStatus::CouldStartDragging(pt, unsn) = self.status
                {
                    self.begin_dragging(app, room, pt, unsn) // sets self.status = Dragging | Resizing
                } else {
                    AppEventStaging::new()
                };

                events.accumulate(match self.status {
                    SelectionStatus::None => AppEventStaging::new(),
                    SelectionStatus::CouldStartDragging(_, _) => unreachable!(),
                    SelectionStatus::Selecting(ref_pos) => {
                        self.pending_selection = self.selectables_in(
                            app,
                            room,
                            app.current_layer,
                            RoomRect::new(ref_pos, (room_pos - ref_pos).to_size()),
                        );
                        AppEventStaging::new()
                    }
                    SelectionStatus::Dragging(DraggingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.nudge(room, room_pos - pointer_reference_point),
                    SelectionStatus::Resizing(ResizingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.resize(app, room, room_pos - pointer_reference_point),
                });

                events
            }
            WindowEvent::KeyDown(code, _) => {
                if self.status == SelectionStatus::None {
                    self.draw_phase = EventPhase::new();
                    match code {
                        Code::ArrowDown => self.nudge(room, RoomVector::new(0, 8)),
                        Code::ArrowUp => self.nudge(room, RoomVector::new(0, -8)),
                        Code::ArrowRight => self.nudge(room, RoomVector::new(8, 0)),
                        Code::ArrowLeft => self.nudge(room, RoomVector::new(-8, 0)),
                        Code::KeyA if cx.modifiers == Modifiers::CTRL => {
                            self.current_selection = self.selectables_in(
                                app,
                                room,
                                app.current_layer,
                                RoomRect::new(
                                    RoomPoint::new(-1000000, -1000000),
                                    RoomSize::new(2000000, 2000000),
                                ),
                            );
                            AppEventStaging::new()
                        }
                        Code::KeyC if cx.modifiers == Modifiers::CTRL => {
                            self.clipboard_copy(app, room)
                        }
                        Code::KeyX if cx.modifiers == Modifiers::CTRL => {
                            let mut result = self.clipboard_copy(app, room);
                            result.accumulate(self.delete_all(app, room));
                            result
                        }
                        Code::KeyV if cx.modifiers == Modifiers::CTRL => {
                            if let Ok(s) = cx.clipboard.get_contents() {
                                let app = cx.data().unwrap();
                                self.clipboard_paste(app, s)
                            } else {
                                AppEventStaging::new()
                            }
                        }
                        Code::Backspace | Code::Delete => self.delete_all(app, room),
                        _ => AppEventStaging::new(),
                    }
                } else {
                    AppEventStaging::new()
                }
            }
            _ => AppEventStaging::new(),
        }
        .finalize(cx.data::<AppState>().unwrap(), self.draw_phase)
    }

    fn internal_event(&mut self, event: &AppInternalEvent, cx: &mut Context) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let room = match app.current_room_ref() {
            Some(room) => room,
            None => return vec![],
        };
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

    fn switch_off(&mut self, app: &AppState, _cx: &Context) -> Vec<AppEvent> {
        self.clear_selection(app).finalize(app, self.draw_phase)
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {
        let room = if let Some(room) = state.current_room_ref() {
            room
        } else {
            return;
        };
        canvas.save();
        canvas.translate(room.bounds.origin.x as f32, room.bounds.origin.y as f32);
        // no scissor!

        let screen_pos = ScreenPoint::new(cx.mouse().cursorx, cx.mouse().cursory);
        let map_pos_precise = state
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap {
            room_pos_snapped
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
                vg::Paint::color(vg::Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        let mut path = vg::Path::new();
        for selectable in self
            .pending_selection
            .iter()
            .chain(self.current_selection.iter())
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

        if let Some((float_pos, float_dat)) = &self.bg_float {
            let mut tiler = |pt| -> Option<char> { Some(float_dat.get_or_default(pt)) };
            let rect = TileRect::new(*float_pos, float_dat.size());
            for pt in rect_point_iter(rect, 1) {
                let float_pt = pt - float_pos.to_vector();
                let ch = float_dat.get_or_default(float_pt);
                if ch != '\0' {
                    if let Some(tile) = state
                        .current_palette_unwrap()
                        .autotilers
                        .get("bg")
                        .unwrap()
                        .get(&ch)
                        .and_then(|tileset| tileset.tile(float_pt, &mut tiler))
                    {
                        let room_pos = point_tile_to_room(&pt);
                        if let Err(e) = state.current_palette_unwrap().gameplay_atlas.draw_tile(
                            canvas,
                            tile,
                            room_pos.x as f32,
                            room_pos.y as f32,
                            Color::white().into(),
                        ) {
                            log::error!("{}", e);
                        }
                        path.rect(room_pos.x as f32, room_pos.y as f32, 8.0, 8.0);
                    }
                }
            }
        }
        if let Some((float_pos, float_dat)) = &self.fg_float {
            let mut tiler = |pt| -> Option<char> { Some(float_dat.get_or_default(pt)) };
            let rect = TileRect::new(*float_pos, float_dat.size());
            for pt in rect_point_iter(rect, 1) {
                let float_pt = pt - float_pos.to_vector();
                let ch = float_dat.get_or_default(float_pt);
                if ch != '\0' {
                    if let Some(tile) = state
                        .current_palette_unwrap()
                        .autotilers
                        .get("fg")
                        .unwrap()
                        .get(&ch)
                        .and_then(|tileset| tileset.tile(float_pt, &mut tiler))
                    {
                        let room_pos = point_tile_to_room(&pt);
                        if let Err(e) = state.current_palette_unwrap().gameplay_atlas.draw_tile(
                            canvas,
                            tile,
                            room_pos.x as f32,
                            room_pos.y as f32,
                            Color::white().into(),
                        ) {
                            log::error!("{}", e);
                        }
                        path.rect(room_pos.x as f32, room_pos.y as f32, 8.0, 8.0);
                    }
                }
            }
        }
        if let Some((float_pos, float_dat)) = &self.obj_float {
            let rect = TileRect::new(*float_pos, float_dat.size());
            for pt in rect_point_iter(rect, 1) {
                let float_pt = pt - float_pos.to_vector();
                let ch = float_dat.get_or_default(float_pt);
                if ch >= 0 {
                    let tile = TileReference {
                        tile: TextureTile {
                            x: (ch % 32) as u32,
                            y: (ch / 32) as u32,
                        },
                        texture: "tilesets/scenery".into(), // TODO we shouldn't be doing this lookup during draw. cache this string statically?
                    };
                    let room_pos = point_tile_to_room(&pt);
                    if let Err(e) = state.current_palette_unwrap().gameplay_atlas.draw_tile(
                        canvas,
                        tile,
                        room_pos.x as f32,
                        room_pos.y as f32,
                        Color::white().into(),
                    ) {
                        log::error!("{}", e)
                    }
                    path.rect(room_pos.x as f32, room_pos.y as f32, 8.0, 8.0);
                }
            }
        }

        canvas.fill_path(
            &mut path,
            vg::Paint::color(vg::Color::rgba(255, 255, 0, 128)),
        );

        if self.status == SelectionStatus::None {
            if let Some(sel) = self.selectable_at(state, room, state.current_layer, room_pos) {
                if !self.current_selection.contains(&sel) {
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
                        vg::Paint::color(vg::Color::rgba(100, 100, 255, 128)),
                    );
                }
            }
        }

        canvas.restore();
    }

    fn cursor(&self, cx: &mut Context) -> CursorIcon {
        let app = cx.data::<AppState>().unwrap();
        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return CursorIcon::Default;
        };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let map_pos = point_lose_precision(&map_pos_precise);
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
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
    fn notify_selection(&self, app: &AppState) -> AppEventStaging {
        let mut result = AppEventStaging::new();
        if self.current_selection.len() == 1 {
            result.push_ind(AppEvent::SelectObject {
                tab: app.current_tab,
                selection: Some(*self.current_selection.iter().next().unwrap()),
            });
        } else {
            result.push_ind(AppEvent::SelectObject {
                tab: app.current_tab,
                selection: None,
            });
        }
        result
    }

    #[must_use]
    fn confirm_selection(&mut self, app: &AppState) -> AppEventStaging {
        self.current_selection
            .extend(self.pending_selection.drain());

        self.notify_selection(app)
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
        if let Some((offset, data)) = &self.obj_float {
            let relative_pos = tile_pos - offset.to_vector();
            if data.get(relative_pos).cloned().unwrap_or(-2) != -2 {
                return true;
            }
        }
        false
    }

    fn rects_of(
        &self,
        app: &AppState,
        room: &CelesteMapLevel,
        selectable: AppSelection,
    ) -> Vec<RoomRect> {
        match selectable {
            AppSelection::FgTile(pt) => {
                vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))]
            }
            AppSelection::BgTile(pt) => {
                vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))]
            }
            AppSelection::ObjectTile(pt) => {
                vec![RoomRect::new(point_tile_to_room(&pt), RoomSize::new(8, 8))]
            }
            AppSelection::EntityBody(id, trigger) => {
                if let Some(entity) = room.entity(id, trigger) {
                    let config = app
                        .current_palette_unwrap()
                        .get_entity_config(&entity.name, trigger);
                    let env = entity.make_env();
                    config
                        .hitboxes
                        .initial_rects
                        .iter()
                        .filter_map(|r| match r.evaluate_int(&env) {
                            Ok(r) => Some(r),
                            Err(_) => {
                                //println!("{}", s);
                                None
                            }
                        })
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
                    let env = entity.make_node_env(entity.make_env(), node_idx);
                    config
                        .hitboxes
                        .node_rects
                        .iter()
                        .filter_map(|r| match r.evaluate_int(&env) {
                            Ok(r) => Some(r),
                            Err(_) => {
                                //println!("{}", s);
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
            AppSelection::Decal(id, fg) => {
                if let Some(decal) = room.decal(id, fg) {
                    if let Some(dim) = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(decal))
                    {
                        let size = dim
                            .cast()
                            .cast_unit()
                            .to_vector()
                            .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                            .cast()
                            .to_size()
                            .abs();
                        vec![Rect::new(
                            RoomPoint::new(decal.x as i32, decal.y as i32) - size / 2,
                            size,
                        )]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
        }
    }

    fn selectable_at(
        &self,
        app: &AppState,
        room: &CelesteMapLevel,
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
        room: &CelesteMapLevel,
        layer: Layer,
        room_rect: RoomRect,
    ) -> HashSet<AppSelection> {
        let room_rect = rect_normalize(&room_rect);
        let mut result = HashSet::new();
        let room_rect_cropped = room_rect.intersection(&RoomRect::new(
            RoomPoint::zero(),
            room.bounds.size.cast_unit(),
        ));

        if layer == Layer::FgDecals || layer == Layer::All {
            for (idx, decal) in room.fg_decals.iter().enumerate().rev() {
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
                    if room.object_tiles.get(tile_pos).cloned().unwrap_or(-1) != -1 {
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
            for (idx, entity) in room.entities.iter().enumerate().rev() {
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
            for (idx, entity) in room.triggers.iter().enumerate().rev() {
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
            for (idx, decal) in room.bg_decals.iter().enumerate().rev() {
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

        result
    }

    #[must_use]
    fn clear_selection(&mut self, app: &AppState) -> AppEventStaging {
        self.current_selection.clear();
        let mut result = self.notify_selection(app);
        if let Some((offset, data)) = self.fg_float.take() {
            result.push_room(RoomEvent::TileUpdate {
                fg: true,
                offset,
                data,
            });
        }
        if let Some((offset, data)) = self.bg_float.take() {
            result.push_room(RoomEvent::TileUpdate {
                fg: false,
                offset,
                data,
            });
        }
        if let Some((offset, data)) = self.obj_float.take() {
            result.push_room(RoomEvent::ObjectTileUpdate { offset, data })
        }
        result
    }

    /// This function interprets nudge as relative to the reference positions in Dragging mode and
    /// relative to the current position in other modes.
    #[must_use]
    fn nudge(&mut self, room: &CelesteMapLevel, nudge: RoomVector) -> AppEventStaging {
        let mut result = self.float_tiles(room);
        let dragging = if let SelectionStatus::Dragging(dragging) = &self.status {
            Some(dragging)
        } else {
            None
        };
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
        if let Some((cur_pt, float)) = self.obj_float.take() {
            let base = dragging
                .map(|d| d.obj_float_reference_point.unwrap())
                .unwrap_or_else(|| point_tile_to_room(&cur_pt));
            self.obj_float = Some((point_room_to_tile(&(base + nudge)), float));
        }
        let mut entity_updates = HashMap::new();
        let mut trigger_updates = HashMap::new();
        for selected in &self.current_selection {
            match selected {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) | AppSelection::ObjectTile(_) => {
                    unreachable!()
                }
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
                        result.push_room(RoomEvent::DecalUpdate {
                            fg: *fg,
                            decal: Box::new(decal),
                        });
                    }
                }
            }
        }

        for entity in entity_updates.into_values() {
            result.push_room(RoomEvent::EntityUpdate {
                entity: Box::new(entity),
                trigger: false,
            });
        }
        for entity in trigger_updates.into_values() {
            result.push_room(RoomEvent::EntityUpdate {
                entity: Box::new(entity),
                trigger: true,
            });
        }
        result
    }

    #[must_use]
    fn resize(
        &mut self,
        app: &AppState,
        room: &CelesteMapLevel,
        resize: RoomVector,
    ) -> AppEventStaging {
        let mut result = AppEventStaging::new();

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
        for sel in &self.current_selection {
            match sel {
                AppSelection::FgTile(_)
                | AppSelection::BgTile(_)
                | AppSelection::ObjectTile(_)
                | AppSelection::EntityNode(_, _, _) => {
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
                    result.push_room(RoomEvent::EntityUpdate {
                        entity: Box::new(e),
                        trigger: *trigger,
                    });
                }
                AppSelection::Decal(id, fg) => {
                    let mut d = room.decal(*id, *fg).unwrap().clone();
                    if let Some(dim) = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(&d))
                    {
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
                        result.push_room(RoomEvent::DecalUpdate {
                            fg: *fg,
                            decal: Box::new(d),
                        })
                    }
                    // TODO what happens if we try to resize an untextured decal?
                }
            }
        }

        result
    }

    #[must_use]
    fn float_tiles(&mut self, room: &CelesteMapLevel) -> AppEventStaging {
        // TODO: do this in an efficient order to avoid frequent reallocations of the float
        let mut result = AppEventStaging::new();
        for sel in self.current_selection.iter().cloned().collect::<Vec<_>>() {
            match sel {
                AppSelection::FgTile(pt) => {
                    add_to_float(&mut self.fg_float, pt, &room.solids, '\0');
                    result.push_room(RoomEvent::TileUpdate {
                        fg: true,
                        offset: pt,
                        data: TileGrid {
                            tiles: vec!['0'],
                            stride: 1,
                        },
                    });
                    self.current_selection.remove(&sel);
                    continue;
                }
                AppSelection::BgTile(pt) => {
                    add_to_float(&mut self.bg_float, pt, &room.bg, '\0');
                    result.push_room(RoomEvent::TileUpdate {
                        fg: false,
                        offset: pt,
                        data: TileGrid {
                            tiles: vec!['0'],
                            stride: 1,
                        },
                    });
                    self.current_selection.remove(&sel);
                    continue;
                }
                AppSelection::ObjectTile(pt) => {
                    add_to_float(&mut self.obj_float, pt, &room.object_tiles, -2);
                    result.push_room(RoomEvent::ObjectTileUpdate {
                        offset: pt,
                        data: TileGrid {
                            tiles: vec![-1],
                            stride: 1,
                        },
                    });
                    self.current_selection.remove(&sel);
                    continue;
                }
                _ => {}
            }
        }

        result
    }

    fn elaborate_nodes(&mut self, room: &CelesteMapLevel) {
        for sel in self
            .current_selection
            .iter()
            .cloned()
            .collect::<Vec<AppSelection>>()
        {
            if let AppSelection::EntityNode(id, _, trigger) = sel {
                self.current_selection
                    .insert(AppSelection::EntityBody(id, trigger));
                for (idx, _) in room.entity(id, trigger).unwrap().nodes.iter().enumerate() {
                    self.current_selection
                        .insert(AppSelection::EntityNode(id, idx, trigger));
                }
            }
        }
    }

    fn can_resize(&self, app: &AppState, room: &CelesteMapLevel, pointer: RoomPoint) -> ResizeSide {
        // get which side of the rectangle we're on
        let mut side = ResizeSide::None;
        'outer: for sel in self.current_selection.iter() {
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
            if self.fg_float.is_some() || self.bg_float.is_some() {
                side = ResizeSide::None;
            } else {
                for sel in &self.current_selection {
                    side = match sel {
                        AppSelection::FgTile(_)
                        | AppSelection::BgTile(_)
                        | AppSelection::ObjectTile(_) => ResizeSide::None,
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
        }

        side
    }

    #[must_use]
    fn begin_dragging(
        &mut self,
        app: &AppState,
        room: &CelesteMapLevel,
        pointer_reference_point: RoomPoint,
        pointer_reference_point_unsnapped: RoomPoint,
    ) -> AppEventStaging {
        // Offload all fg/bg selections into the floats
        let result = self.float_tiles(room);

        let side = self.can_resize(app, room, pointer_reference_point_unsnapped);
        if side != ResizeSide::None {
            // collect reference sizes
            let selection_reference_sizes = self
                .current_selection
                .iter()
                .filter_map(|sel| match sel {
                    AppSelection::FgTile(_)
                    | AppSelection::BgTile(_)
                    | AppSelection::ObjectTile(_)
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
                            if let Some(dim) = app
                                .current_palette_unwrap()
                                .gameplay_atlas
                                .sprite_dimensions(&decal_texture(decal))
                            {
                                let size = dim
                                    .cast()
                                    .cast_unit()
                                    .to_vector()
                                    .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                                    .cast()
                                    .to_size();
                                Some((
                                    *sel,
                                    RoomRect::new(
                                        RoomPoint::new(decal.x, decal.y) - size / 2,
                                        size,
                                    ),
                                ))
                            } else {
                                None
                            }
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
            let fg_float_reference_point = self.fg_float.as_ref().map(|f| point_tile_to_room(&f.0));
            let bg_float_reference_point = self.bg_float.as_ref().map(|f| point_tile_to_room(&f.0));
            let obj_float_reference_point =
                self.obj_float.as_ref().map(|f| point_tile_to_room(&f.0));
            let selection_reference_points = self
                .current_selection
                .iter()
                .map(|sel| {
                    (
                        *sel,
                        match sel {
                            AppSelection::FgTile(_)
                            | AppSelection::BgTile(_)
                            | AppSelection::ObjectTile(_) => unreachable!(),
                            AppSelection::EntityBody(id, trigger) => {
                                let e = room.entity(*id, *trigger).unwrap();
                                RoomPoint::new(e.x, e.y)
                            }
                            AppSelection::EntityNode(id, node_idx, trigger) => {
                                let e = room.entity(*id, *trigger).unwrap();
                                RoomPoint::new(e.nodes[*node_idx].x, e.nodes[*node_idx].y)
                            }
                            AppSelection::Decal(id, fg) => {
                                let d = room.decal(*id, *fg).unwrap();
                                RoomPoint::new(d.x, d.y)
                            }
                        },
                    )
                })
                .collect::<HashMap<AppSelection, RoomPoint>>();

            // here's your status!
            self.status = SelectionStatus::Dragging(DraggingStatus {
                pointer_reference_point,
                selection_reference_points,
                fg_float_reference_point,
                bg_float_reference_point,
                obj_float_reference_point,
            });
        }

        result
    }

    #[must_use]
    fn delete_all(&mut self, app: &AppState, room: &CelesteMapLevel) -> AppEventStaging {
        let mut result = self.float_tiles(room);
        self.fg_float = None;
        self.bg_float = None;
        self.obj_float = None;

        let mut entity_nodes_removed = HashMap::new();
        let mut trigger_nodes_removed = HashMap::new();
        let mut entities_removed = HashSet::new();
        let mut triggers_removed = HashSet::new();
        for sel in &self.current_selection {
            match sel {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) | AppSelection::ObjectTile(_) => {
                    unreachable!()
                }
                AppSelection::EntityBody(id, trigger) => {
                    result.push_room(RoomEvent::EntityRemove {
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
                    result.push_room(RoomEvent::DecalRemove { id: *id, fg: *fg });
                }
            }
        }

        for (id, indices) in entity_nodes_removed {
            if !entities_removed.contains(&id) {
                if let Some(entity) = room.entity(*id, false) {
                    let mut entity = entity.clone();
                    for idx in (0..entity.nodes.len()).rev() {
                        if indices.contains(&idx) {
                            entity.nodes.remove(idx);
                        }
                    }
                    result.push_room(RoomEvent::EntityUpdate {
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
                    result.push_room(RoomEvent::EntityUpdate {
                        entity: Box::new(entity),
                        trigger: true,
                    });
                }
            }
        }

        self.current_selection.clear();
        result.accumulate(self.notify_selection(app));

        result
    }

    pub fn clipboard_copy(&mut self, app: &AppState, room: &CelesteMapLevel) -> AppEventStaging {
        let mut result = self.float_tiles(room);
        self.elaborate_nodes(room);
        result.accumulate(self.notify_selection(app));

        let mut clipboard_data: Vec<AppInRoomSelectable> = vec![];
        if let Some(float) = &self.fg_float {
            clipboard_data.push(AppInRoomSelectable::FgTiles(float.0, float.1.clone()));
        }
        if let Some(float) = &self.bg_float {
            clipboard_data.push(AppInRoomSelectable::BgTiles(float.0, float.1.clone()));
        }
        if let Some(float) = &self.obj_float {
            clipboard_data.push(AppInRoomSelectable::ObjectTiles(float.0, float.1.clone()));
        }
        for sel in &self.current_selection {
            match sel {
                AppSelection::FgTile(_) | AppSelection::BgTile(_) | AppSelection::ObjectTile(_) => {
                    unreachable!()
                }
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
        let s = serde_yaml::to_string(&AppSelectable::InRoom(clipboard_data))
            .expect("Failed to serialize copied data");
        result.push_ind(AppEvent::SetClipboard { contents: s });
        result
    }

    pub fn clipboard_paste(&mut self, app: &AppState, data: String) -> AppEventStaging {
        let mut result = self.clear_selection(app);
        let deser: AppSelectable = match serde_yaml::from_str(&data) {
            Ok(d) => d,
            Err(_) => return result,
        };
        let room = match app.current_room_ref() {
            Some(room) => room,
            None => return result,
        };
        let clipboard_data = match deser {
            AppSelectable::InRoom(d) => d,
            AppSelectable::Rooms(_) => return result,
        };
        if clipboard_data.is_empty() {
            return result;
        }
        let mut min_tile = TilePoint::new(i32::MAX, i32::MAX);
        let mut max_tile = TilePoint::new(i32::MIN, i32::MIN);
        for obj in clipboard_data.iter() {
            match obj {
                AppInRoomSelectable::FgTiles(point, float) => {
                    min_tile = min_tile.min(*point);
                    max_tile = max_tile.max(*point + float.size());
                }
                AppInRoomSelectable::BgTiles(point, float) => {
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
                    let env = entity.make_env();
                    for hitbox in config.hitboxes.initial_rects.iter().filter_map(|r| {
                        match r.evaluate_int(&env) {
                            Ok(r) => Some(r),
                            Err(_) => {
                                //println!("{}", s);
                                None
                            }
                        }
                    }) {
                        min_tile = min_tile.min(point_room_to_tile(&hitbox.min()));
                        max_tile = max_tile.max(point_room_to_tile(&hitbox.max()));
                    }
                }
                AppInRoomSelectable::Decal(decal, _) => {
                    if let Some(dim) = app
                        .current_palette_unwrap()
                        .gameplay_atlas
                        .sprite_dimensions(&decal_texture(decal))
                    {
                        let size = dim
                            .cast()
                            .cast_unit()
                            .to_vector()
                            .component_mul(Vector2D::new(decal.scale_x, decal.scale_y))
                            .cast()
                            .to_size()
                            .abs();
                        let hitbox = Rect::new(
                            RoomPoint::new(decal.x as i32, decal.y as i32) - size / 2,
                            size,
                        );
                        min_tile = min_tile.min(point_room_to_tile(&hitbox.min()));
                        max_tile = max_tile.max(point_room_to_tile(&hitbox.max()));
                    }
                }
            }
        }
        let center = (min_tile.to_vector() + max_tile.to_vector()) / 2;
        let real_center =
            (size_room_to_tile(&room.bounds.size.cast_unit::<RoomSpace>()) / 2).to_vector();
        let offset = real_center - center;
        for obj in clipboard_data {
            match obj {
                AppInRoomSelectable::FgTiles(point, float) => {
                    self.fg_float = Some((point + offset, float));
                }
                AppInRoomSelectable::BgTiles(point, float) => {
                    self.bg_float = Some((point + offset, float));
                }
                AppInRoomSelectable::ObjectTiles(point, float) => {
                    self.obj_float = Some((point + offset, float));
                }
                AppInRoomSelectable::Entity(mut entity, trigger) => {
                    entity.x += vector_tile_to_room(&offset).x;
                    entity.y += vector_tile_to_room(&offset).y;
                    for node in entity.nodes.iter_mut() {
                        node.x += vector_tile_to_room(&offset).x;
                        node.y += vector_tile_to_room(&offset).y;
                    }
                    result.push_room(RoomEvent::EntityAdd {
                        entity: Box::new(entity),
                        trigger,
                        selectme: true,
                        genid: true,
                    });
                }
                AppInRoomSelectable::Decal(mut decal, fg) => {
                    decal.x += vector_tile_to_room(&offset).x;
                    decal.y += vector_tile_to_room(&offset).y;
                    result.push_room(RoomEvent::DecalAdd {
                        decal: Box::new(decal),
                        fg,
                        selectme: true,
                        genid: true,
                    });
                }
            }
        }
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
fn add_to_float<T: Copy>(
    float: &mut Option<(TilePoint, TileGrid<T>)>,
    pt: TilePoint,
    src: &TileGrid<T>,
    filler: T,
) {
    if let Some(ch) = src.get(pt) {
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
                let src = &old_dat.tiles.as_slice()
                    [(line * old_size.x) as usize..((line + 1) * old_size.x) as usize];
                new_dat.tiles.as_mut_slice()[(dest_start_offset + line * new_size.x) as usize
                    ..(dest_start_offset + line * new_size.x + old_size.x) as usize]
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
