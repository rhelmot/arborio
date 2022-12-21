use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Color, Paint, Path};
use std::collections::{HashMap, HashSet};

use crate::data::action::{MapAction, RoomAction};
use crate::data::app::{AppEvent, AppInternalEvent, AppState};
use crate::data::project_map::MapState;
use crate::data::selection::AppSelectable;
use crate::data::{EventPhase, MapID};
use crate::tools::selection::ResizeSide;
use crate::tools::{generic_nav, Tool};
use arborio_maploader::map_struct::CelesteMapLevel;
use arborio_utils::units::*;

pub struct RoomTool {
    pending_selection: HashSet<usize>,
    current_selection: HashSet<usize>,
    status: SelectionStatus,
    draw_phase: EventPhase,
}

#[derive(Eq, PartialEq, Debug)]
enum SelectionStatus {
    None,
    Selecting(MapPointStrict),
    CouldStartDragging(MapPointStrict, MapPointStrict),
    Dragging(DraggingStatus),
    Resizing(ResizingStatus),
}

#[derive(Eq, PartialEq, Debug)]
struct DraggingStatus {
    pointer_reference_point: MapPointStrict,
    selection_reference_points: HashMap<usize, MapPointStrict>,
}

#[derive(Eq, PartialEq, Debug)]
struct ResizingStatus {
    pointer_reference_point: MapPointStrict,
    selection_reference_sizes: HashMap<usize, MapRectStrict>,
    side: ResizeSide,
}

impl RoomTool {
    pub fn new(app: &AppState) -> Self {
        RoomTool {
            current_selection: HashSet::from([app.map_tab_unwrap().current_room]),
            pending_selection: HashSet::new(),
            status: SelectionStatus::None,
            draw_phase: EventPhase::null(),
        }
    }
}

impl Tool for RoomTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let events = generic_nav(event, app, cx, false);
        if !events.is_empty() {
            return events;
        }

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos_unsnapped = point_lose_precision(&map_pos_precise);
        let map_pos = (map_pos_unsnapped / 8) * 8;

        let map = app.current_map_ref().unwrap();
        let mapid = app.map_tab_unwrap().id;

        match event {
            WindowEvent::MouseUp(_) => {
                let events = match self.status {
                    SelectionStatus::Selecting(_) => self.confirm_selection(app),
                    SelectionStatus::Resizing(ResizingStatus {
                        pointer_reference_point,
                        ..
                    }) => {
                        vec![app.batch_action(
                            self.resize(map, map_pos - pointer_reference_point),
                            self.draw_phase,
                        )]
                    }
                    _ => vec![],
                };
                self.status = SelectionStatus::None;
                events
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.draw_phase = EventPhase::new();
                if self.status == SelectionStatus::None {
                    let got = room_at(map, map_pos_unsnapped);
                    if got.is_some() && self.current_selection.contains(&got.unwrap()) {
                        self.status =
                            SelectionStatus::CouldStartDragging(map_pos, map_pos_unsnapped);
                        vec![]
                    } else {
                        self.status = SelectionStatus::Selecting(map_pos);
                        if let Some(g) = got {
                            self.pending_selection = HashSet::from([g]);
                        }
                        if !cx.modifiers.contains(Modifiers::SHIFT) {
                            self.clear_selection(app)
                        } else {
                            vec![]
                        }
                    }
                } else {
                    vec![]
                }
            }
            WindowEvent::MouseMove(_, _) => {
                if let SelectionStatus::CouldStartDragging(pt, unsn) = self.status {
                    self.begin_dragging(map, pt, unsn) // sets self.status = Dragging | Resizing
                }

                match self.status {
                    SelectionStatus::None => vec![],
                    SelectionStatus::CouldStartDragging(_, _) => unreachable!(),
                    SelectionStatus::Selecting(ref_pos) => {
                        self.pending_selection = rooms_in(
                            map,
                            MapRectStrict::new(ref_pos, (map_pos - ref_pos).to_size()),
                        );
                        vec![]
                    }
                    SelectionStatus::Dragging(DraggingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.nudge(app, map, map_pos - pointer_reference_point),
                    SelectionStatus::Resizing(_) => vec![],
                }
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                self.draw_phase = EventPhase::new();
                if self.status == SelectionStatus::None {
                    let mut result = CelesteMapLevel::default();
                    result.bounds.origin = map_pos;
                    self.current_selection = HashSet::from([map.data.levels.len()]);
                    self.status = SelectionStatus::Dragging(DraggingStatus {
                        pointer_reference_point: map_pos,
                        selection_reference_points: HashMap::from([(
                            map.data.levels.len(),
                            map_pos,
                        )]),
                    });
                    let mut events = self.notify_selection(app);
                    events.push(app.map_action_unique(vec![MapAction::AddRoom {
                        idx: None,
                        room: Box::new(result),
                    }]));
                    events
                } else {
                    vec![]
                }
            }
            WindowEvent::KeyDown(code, _) => {
                if self.status == SelectionStatus::None {
                    self.draw_phase = EventPhase::new();
                    match code {
                        Code::ArrowDown => self.nudge(app, map, MapVectorStrict::new(0, 8)),
                        Code::ArrowUp => self.nudge(app, map, MapVectorStrict::new(0, -8)),
                        Code::ArrowRight => self.nudge(app, map, MapVectorStrict::new(8, 0)),
                        Code::ArrowLeft => self.nudge(app, map, MapVectorStrict::new(-8, 0)),
                        Code::KeyA if cx.modifiers == &Modifiers::CTRL => {
                            self.current_selection = rooms_in(
                                map,
                                MapRectStrict::new(
                                    MapPointStrict::new(-1000000, -1000000),
                                    MapSizeStrict::new(2000000, 2000000),
                                ),
                            );
                            vec![]
                        }
                        Code::KeyC if cx.modifiers == &Modifiers::CTRL => {
                            self.clipboard_copy(app, mapid)
                        }
                        Code::KeyX if cx.modifiers == &Modifiers::CTRL => {
                            let mut result = self.clipboard_copy(app, mapid);
                            result.extend(self.delete_all(app));
                            result
                        }
                        Code::KeyV if cx.modifiers == &Modifiers::CTRL => {
                            if let Ok(s) = cx.get_clipboard() {
                                let app = cx.data().unwrap();
                                self.clipboard_paste(app, s)
                            } else {
                                vec![]
                            }
                        }
                        Code::Backspace | Code::Delete => self.delete_all(app),
                        _ => vec![],
                    }
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn internal_event(
        &mut self,
        event: &AppInternalEvent,
        _cx: &mut EventContext,
    ) -> Vec<AppEvent> {
        if let AppInternalEvent::SelectMeRoom { idx } = event {
            self.current_selection.insert(*idx);
        }
        vec![]
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {
        let Some(map) = state.current_map_ref() else { return };

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = state
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos_unsnapped = point_lose_precision(&map_pos_precise);
        let map_pos = (map_pos_unsnapped / 8) * 8;

        canvas.save();
        if let SelectionStatus::Selecting(ref_pos) = &self.status {
            let selection = rect_normalize(&MapRectStrict::new(
                *ref_pos,
                (map_pos - *ref_pos).to_size(),
            ));
            let mut path = Path::new();
            path.rect(
                selection.min_x() as f32,
                selection.min_y() as f32,
                selection.width() as f32,
                selection.height() as f32,
            );
            canvas.stroke_path(
                &mut path,
                &Paint::color(Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        let mut path = Path::new();
        for room in self
            .pending_selection
            .iter()
            .chain(self.current_selection.iter())
        {
            if let Some(room) = map.data.levels.get(*room) {
                let rect = &room.data.bounds;
                path.rect(
                    rect.min_x() as f32,
                    rect.min_y() as f32,
                    rect.width() as f32,
                    rect.height() as f32,
                )
            }
        }

        canvas.fill_path(&mut path, &Paint::color(Color::rgba(255, 255, 0, 128)));

        if self.status == SelectionStatus::None {
            if let Some(room) = room_at(map, map_pos_unsnapped) {
                if !self.current_selection.contains(&room) {
                    let mut path = Path::new();
                    if let Some(room) = map.data.levels.get(room) {
                        let rect = &room.data.bounds;
                        path.rect(
                            rect.min_x() as f32,
                            rect.min_y() as f32,
                            rect.width() as f32,
                            rect.height() as f32,
                        );
                    }
                    canvas.fill_path(&mut path, &Paint::color(Color::rgba(100, 100, 255, 128)));
                }
            }
        }

        if let SelectionStatus::Resizing(ResizingStatus {
            pointer_reference_point,
            ..
        }) = self.status
        {
            let mut path = Path::new();
            for fake_event in self.resize(map, map_pos - pointer_reference_point) {
                if let MapAction::RoomAction {
                    event: RoomAction::MoveRoom { bounds, .. },
                    ..
                } = fake_event
                {
                    path.rect(
                        bounds.min_x() as f32,
                        bounds.min_y() as f32,
                        bounds.width() as f32,
                        bounds.height() as f32,
                    );
                }
            }

            canvas.stroke_path(
                &mut path,
                &Paint::color(Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        canvas.restore();
    }

    fn cursor(&self, cx: &mut EventContext) -> CursorIcon {
        let app = cx.data::<AppState>().unwrap();
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos_unsnapped = point_lose_precision(&map_pos_precise);

        let map = app.current_map_ref().unwrap();

        match &self.status {
            SelectionStatus::CouldStartDragging(_, _) | SelectionStatus::None => {
                self.can_resize(map, map_pos_unsnapped).to_cursor_icon()
            }
            SelectionStatus::Dragging(_) | SelectionStatus::Selecting(_) => CursorIcon::Default,
            SelectionStatus::Resizing(info) => info.side.to_cursor_icon(),
        }
    }
}

impl RoomTool {
    fn clear_selection(&mut self, app: &AppState) -> Vec<AppEvent> {
        self.current_selection.clear();
        self.notify_selection(app)
    }

    fn notify_selection(&self, app: &AppState) -> Vec<AppEvent> {
        if self.current_selection.len() == 1 {
            vec![AppEvent::SelectRoom {
                tab: app.current_tab,
                idx: *self.current_selection.iter().next().unwrap(),
            }]
        } else {
            vec![]
        }
    }

    fn confirm_selection(&mut self, app: &AppState) -> Vec<AppEvent> {
        self.current_selection
            .extend(self.pending_selection.drain());

        self.notify_selection(app)
    }

    fn nudge(&self, app: &AppState, map: &MapState, nudge: MapVectorStrict) -> Vec<AppEvent> {
        let dragging = if let SelectionStatus::Dragging(dragging) = &self.status {
            Some(dragging)
        } else {
            None
        };

        let mut events = vec![];

        for room in self.current_selection.iter() {
            let base = dragging
                .map(|d| d.selection_reference_points[room])
                .unwrap_or_else(|| map.data.levels[*room].data.bounds.origin);
            events.push(MapAction::RoomAction {
                event: RoomAction::MoveRoom {
                    bounds: MapRectStrict::new(
                        base + nudge,
                        map.data.levels[*room].data.bounds.size,
                    ),
                },
                idx: *room,
            });
        }

        vec![app.batch_action(events, self.draw_phase)]
    }

    fn resize(&self, map: &MapState, resize: MapVectorStrict) -> Vec<MapAction> {
        let pos_vec;
        let size_vec;
        let dragging = if let SelectionStatus::Resizing(dragging) = &self.status {
            let side = dragging.side;
            pos_vec = MapVectorStrict::new(
                if side.is_left() { resize.x } else { 0 },
                if side.is_top() { resize.y } else { 0 },
            );
            size_vec = MapVectorStrict::new(
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
            Some(dragging)
        } else {
            pos_vec = MapVectorStrict::new(resize.x, resize.y);
            size_vec = MapVectorStrict::new(-resize.x, -resize.y);
            None
        };

        let mut events = vec![];

        for room in self.current_selection.iter() {
            let start_rect = dragging
                .map(|d| d.selection_reference_sizes[room])
                .unwrap_or_else(|| map.data.levels[*room].data.bounds);
            let mut new_rect = MapRectStrict::new(
                start_rect.origin + pos_vec,
                start_rect.size + size_vec.to_size(),
            );
            new_rect.size.width = new_rect.size.width.max(8);
            new_rect.size.height = new_rect.size.height.max(8);
            events.push(MapAction::RoomAction {
                event: RoomAction::MoveRoom { bounds: new_rect },
                idx: *room,
            });
        }

        events
    }

    fn begin_dragging(&mut self, map: &MapState, pt: MapPointStrict, pt_unsnapped: MapPointStrict) {
        let selection_reference_sizes = self.current_selection.iter().filter_map(|idx| {
            map.data
                .levels
                .get(*idx)
                .map(|room| (*idx, room.data.bounds))
        });
        self.status = match self.can_resize(map, pt_unsnapped) {
            ResizeSide::None => SelectionStatus::Dragging(DraggingStatus {
                pointer_reference_point: pt,
                selection_reference_points: selection_reference_sizes
                    .map(|(idx, size)| (idx, size.origin))
                    .collect(),
            }),
            side => SelectionStatus::Resizing(ResizingStatus {
                pointer_reference_point: pt,
                selection_reference_sizes: selection_reference_sizes.collect(),
                side,
            }),
        };
    }

    fn can_resize(&self, map: &MapState, pointer: MapPointStrict) -> ResizeSide {
        let Some(rect) = self
            .current_selection
            .iter()
            .filter_map(|idx| {
                Some(map.data.levels.get(*idx)?.data.bounds)
            })
            .find(|rect| rect.contains(pointer))
            else { return ResizeSide::None };

        let smaller_rect = rect.inflate(-2, -2);
        let at_top = pointer.y < smaller_rect.min_y();
        let at_bottom = pointer.y >= smaller_rect.max_y();
        let at_left = pointer.x < smaller_rect.min_x();
        let at_right = pointer.x >= smaller_rect.max_x();

        ResizeSide::from_sides(at_top, at_bottom, at_left, at_right)
    }

    fn delete_all(&self, app: &AppState) -> Vec<AppEvent> {
        let phase = EventPhase::new();
        self.current_selection
            .iter()
            .map(|&idx| app.map_action(vec![MapAction::DeleteRoom { idx }], phase))
            .collect()
    }

    fn clipboard_copy(&self, app: &AppState, mapid: MapID) -> Vec<AppEvent> {
        if self.current_selection.is_empty() {
            return vec![];
        }
        let map = app.loaded_maps.get(&mapid).unwrap();
        vec![AppEvent::SetClipboard {
            contents: serde_yaml::to_string(&AppSelectable::Rooms(
                self.current_selection
                    .iter()
                    .map(|roomid| map.data.levels.get(*roomid).unwrap().data.clone())
                    .collect(),
            ))
            .unwrap(),
        }]
    }

    fn clipboard_paste(&mut self, app: &AppState, data: String) -> Vec<AppEvent> {
        let mut result = self.clear_selection(app);
        let Ok(AppSelectable::Rooms(clipboard_data)) = serde_yaml::from_str(&data) else { return result };
        if clipboard_data.is_empty() {
            return result;
        }
        let mut min_pt = MapPointStrict::new(i32::MAX, i32::MAX);
        let mut max_pt = MapPointStrict::new(i32::MIN, i32::MIN);
        for room in clipboard_data.iter() {
            min_pt = min_pt.min(room.bounds.min());
            max_pt = max_pt.max(room.bounds.max());
        }
        let center = ((min_pt.to_vector() + max_pt.to_vector()) / 2).to_point();
        let real_center = point_lose_precision(
            &app.map_tab_unwrap()
                .transform
                .inverse()
                .unwrap()
                .transform_point(ScreenPoint::new(100., 100.)),
        );
        let real_center =
            point_tile_to_room(&point_room_to_tile(&real_center.cast_unit())).cast_unit();
        let offset = real_center - center;
        result.push(
            app.batch_action_unique(clipboard_data.into_iter().map(|mut room| {
                MapAction::AddRoom {
                    idx: None,
                    room: Box::new({
                        room.bounds.origin += offset;
                        room
                    }),
                }
            })),
        );
        result
    }
}

fn room_at(map: &MapState, pos: MapPointStrict) -> Option<usize> {
    rooms_in(map, MapRectStrict::new(pos, MapSizeStrict::new(1, 1)))
        .iter()
        .next()
        .cloned()
}

fn rooms_in(map: &MapState, rect: MapRectStrict) -> HashSet<usize> {
    let rect = rect_normalize(&rect);
    let mut result = HashSet::new();
    for (idx, room) in map.data.levels.iter().enumerate() {
        if room.data.bounds.intersects(&rect) {
            result.insert(idx);
        }
    }
    result
}
