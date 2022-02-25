use std::collections::{HashMap, HashSet};
use vizia::*;

use crate::map_struct::{CelesteMap, CelesteMapLevel};
use crate::tools::selection::ResizeSide;
use crate::tools::{generic_nav, Tool};
use crate::units::*;
use crate::{AppEvent, AppState, Context, WindowEvent};

pub struct RoomTool {
    pending_selection: HashSet<usize>,
    current_selection: HashSet<usize>,
    status: SelectionStatus,
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
        }
    }
}

impl Tool for RoomTool {
    fn event(&mut self, event: &WindowEvent, app: &AppState, cx: &Context) -> Vec<AppEvent> {
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

        match event {
            WindowEvent::MouseUp(MouseButton::Left) => {
                let events = match self.status {
                    SelectionStatus::Selecting(_) => self.confirm_selection(app),
                    SelectionStatus::Resizing(ResizingStatus {
                        pointer_reference_point,
                        ..
                    }) => self.resize(map, map_pos - pointer_reference_point),
                    _ => vec![],
                };
                self.status = SelectionStatus::None;
                events
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
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
                    }) => self.nudge(map, map_pos - pointer_reference_point),
                    SelectionStatus::Resizing(_) => vec![],
                }
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                let mut result = CelesteMapLevel::default();
                result.bounds.origin = map_pos;
                self.current_selection = HashSet::from([map.levels.len()]);
                vec![AppEvent::AddRoom {
                    map: map.id.clone(),
                    idx: None,
                    room: Box::new(result),
                }]
            }
            WindowEvent::KeyDown(code, _) if self.status == SelectionStatus::None => match code {
                Code::ArrowDown => self.nudge(map, MapVectorStrict::new(0, 8)),
                Code::ArrowUp => self.nudge(map, MapVectorStrict::new(0, -8)),
                Code::ArrowRight => self.nudge(map, MapVectorStrict::new(8, 0)),
                Code::ArrowLeft => self.nudge(map, MapVectorStrict::new(-8, 0)),
                Code::KeyA if cx.modifiers.contains(Modifiers::CTRL) => {
                    self.current_selection = rooms_in(
                        map,
                        MapRectStrict::new(
                            MapPointStrict::new(-1000000, -1000000),
                            MapSizeStrict::new(2000000, 2000000),
                        ),
                    );
                    vec![]
                }
                Code::Backspace | Code::Delete => self.delete_all(map),
                _ => vec![],
            },
            _ => vec![],
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, app: &AppState, cx: &Context) {
        canvas.save();
        let map = if let Some(map) = app.current_map_ref() {
            map
        } else {
            return;
        };

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos_unsnapped = point_lose_precision(&map_pos_precise);
        let map_pos = (map_pos_unsnapped / 8) * 8;

        if let SelectionStatus::Selecting(ref_pos) = &self.status {
            let selection = rect_normalize(&MapRectStrict::new(
                *ref_pos,
                (map_pos - *ref_pos).to_size(),
            ));
            let mut path = femtovg::Path::new();
            path.rect(
                selection.min_x() as f32,
                selection.min_y() as f32,
                selection.width() as f32,
                selection.height() as f32,
            );
            canvas.stroke_path(
                &mut path,
                femtovg::Paint::color(femtovg::Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        let mut path = femtovg::Path::new();
        for room in self
            .pending_selection
            .iter()
            .chain(self.current_selection.iter())
        {
            if let Some(room) = map.levels.get(*room) {
                let rect = &room.bounds;
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
            femtovg::Paint::color(femtovg::Color::rgba(255, 255, 0, 128)),
        );

        if self.status == SelectionStatus::None {
            if let Some(room) = room_at(map, map_pos_unsnapped) {
                if !self.current_selection.contains(&room) {
                    let mut path = femtovg::Path::new();
                    if let Some(room) = map.levels.get(room) {
                        let rect = &room.bounds;
                        path.rect(
                            rect.min_x() as f32,
                            rect.min_y() as f32,
                            rect.width() as f32,
                            rect.height() as f32,
                        );
                    }
                    canvas.fill_path(
                        &mut path,
                        femtovg::Paint::color(femtovg::Color::rgba(100, 100, 255, 128)),
                    );
                }
            }
        }

        if let SelectionStatus::Resizing(ResizingStatus {
            pointer_reference_point,
            ..
        }) = self.status
        {
            let mut path = femtovg::Path::new();
            for fake_event in self.resize(map, map_pos - pointer_reference_point) {
                if let AppEvent::MoveRoom { bounds, .. } = fake_event {
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
                femtovg::Paint::color(femtovg::Color::rgb(0, 0, 0)).with_line_width(1.5),
            );
        }

        canvas.restore();
    }

    fn cursor(&self, cx: &Context, app: &AppState) -> CursorIcon {
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

    fn nudge(&self, map: &CelesteMap, nudge: MapVectorStrict) -> Vec<AppEvent> {
        let dragging = if let SelectionStatus::Dragging(dragging) = &self.status {
            Some(dragging)
        } else {
            None
        };

        let mut events = vec![];

        for room in self.current_selection.iter() {
            let base = dragging
                .map(|d| d.selection_reference_points[room])
                .unwrap_or_else(|| map.levels[*room].bounds.origin);
            events.push(AppEvent::MoveRoom {
                map: map.id.clone(),
                room: *room,
                bounds: MapRectStrict::new(base + nudge, map.levels[*room].bounds.size),
            });
        }

        events
    }

    fn resize(&self, map: &CelesteMap, resize: MapVectorStrict) -> Vec<AppEvent> {
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
        let pos_vec = MapVectorStrict::new(
            if side.is_left() { resize.x } else { 0 },
            if side.is_top() { resize.y } else { 0 },
        );
        let size_vec = MapVectorStrict::new(
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

        let mut events = vec![];

        for room in self.current_selection.iter() {
            let start_rect = dragging
                .map(|d| d.selection_reference_sizes[room])
                .unwrap_or_else(|| map.levels[*room].bounds);
            let mut new_rect = MapRectStrict::new(
                start_rect.origin + pos_vec,
                start_rect.size + size_vec.to_size(),
            );
            new_rect.size.width = new_rect.size.width.max(8);
            new_rect.size.height = new_rect.size.height.max(8);
            events.push(AppEvent::MoveRoom {
                map: map.id.clone(),
                room: *room,
                bounds: new_rect,
            });
        }

        events
    }

    fn begin_dragging(
        &mut self,
        map: &CelesteMap,
        pt: MapPointStrict,
        pt_unsnapped: MapPointStrict,
    ) {
        let side = self.can_resize(map, pt_unsnapped);
        if side != ResizeSide::None {
            let selection_reference_sizes = self
                .current_selection
                .iter()
                .filter_map(|idx| map.levels.get(*idx).map(|room| (*idx, room.bounds)))
                .collect::<HashMap<_, _>>();

            self.status = SelectionStatus::Resizing(ResizingStatus {
                pointer_reference_point: pt,
                selection_reference_sizes,
                side,
            });
        } else {
            let selection_reference_points = self
                .current_selection
                .iter()
                .filter_map(|idx| map.levels.get(*idx).map(|room| (*idx, room.bounds.origin)))
                .collect::<HashMap<_, _>>();

            self.status = SelectionStatus::Dragging(DraggingStatus {
                pointer_reference_point: pt,
                selection_reference_points,
            });
        }
    }

    fn can_resize(&self, map: &CelesteMap, pointer: MapPointStrict) -> ResizeSide {
        for idx in self.current_selection.iter() {
            if let Some(room) = map.levels.get(*idx) {
                let rect = &room.bounds;
                if rect.contains(pointer) {
                    let smaller_rect = rect.inflate(-2, -2);
                    let at_top = pointer.y < smaller_rect.min_y();
                    let at_bottom = pointer.y >= smaller_rect.max_y();
                    let at_left = pointer.x < smaller_rect.min_x();
                    let at_right = pointer.x >= smaller_rect.max_x();

                    return ResizeSide::from_sides(at_top, at_bottom, at_left, at_right);
                }
            }
        }

        ResizeSide::None
    }

    fn delete_all(&self, map: &CelesteMap) -> Vec<AppEvent> {
        self.current_selection
            .iter()
            .map(|idx| AppEvent::DeleteRoom {
                map: map.id.clone(),
                idx: *idx,
            })
            .collect()
    }
}

fn room_at(map: &CelesteMap, pos: MapPointStrict) -> Option<usize> {
    rooms_in(map, MapRectStrict::new(pos, MapSizeStrict::new(1, 1)))
        .iter()
        .next()
        .cloned()
}

fn rooms_in(map: &CelesteMap, rect: MapRectStrict) -> HashSet<usize> {
    let rect = rect_normalize(&rect);
    let mut result = HashSet::new();
    for (idx, room) in map.levels.iter().enumerate() {
        if room.bounds.intersects(&rect) {
            result.insert(idx);
        }
    }
    result
}
