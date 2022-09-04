pub mod bucket;
pub mod hand;
pub mod pencil;
pub mod room;
pub mod selection;
pub mod style;

use arborio_utils::vizia::prelude::*;
use enum_iterator::IntoEnumIterator;

use crate::data::app::{AppEvent, AppInternalEvent, AppState};
use arborio_utils::units::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoEnumIterator)]
pub enum ToolSpec {
    Hand,
    Selection,
    Pencil,
    Room,
    Style,
    Bucket,
}

impl Data for ToolSpec {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl ToolSpec {
    pub fn name(&self) -> &'static str {
        match self {
            ToolSpec::Hand => "Hand",
            ToolSpec::Selection => "Select",
            ToolSpec::Pencil => "Pencil",
            ToolSpec::Room => "Rooms",
            ToolSpec::Style => "Style",
            ToolSpec::Bucket => "Bucket",
        }
    }

    pub fn switch_on(&self, app: &AppState) -> Box<dyn Tool> {
        match self {
            ToolSpec::Hand => Box::new(hand::HandTool::new()),
            ToolSpec::Selection => Box::new(selection::SelectionTool::new(app)),
            ToolSpec::Pencil => Box::new(pencil::PencilTool::new()),
            ToolSpec::Room => Box::new(room::RoomTool::new(app)),
            ToolSpec::Style => Box::new(style::StyleTool::new(app)),
            ToolSpec::Bucket => Box::new(bucket::BucketTool::new()),
        }
    }
}

#[allow(unused_variables)]
pub trait Tool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent>;
    fn internal_event(&mut self, event: &AppInternalEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        vec![]
    }

    fn switch_off(&mut self, app: &AppState, cx: &EventContext) -> Vec<AppEvent> {
        vec![]
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {}

    fn cursor(&self, cx: &mut EventContext) -> CursorIcon {
        CursorIcon::Default
    }
}

pub const SCROLL_SENSITIVITY: f32 = 35.0;

pub fn generic_nav(
    event: &WindowEvent,
    state: &AppState,
    cx: &EventContext,
    room: bool,
) -> Vec<AppEvent> {
    let screen_pt = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
    match event {
        WindowEvent::MouseScroll(_, y) if cx.modifiers.contains(Modifiers::CTRL) => {
            vec![AppEvent::Zoom {
                tab: state.current_tab,
                delta: y.exp(),
                focus: state
                    .map_tab_unwrap()
                    .transform
                    .inverse()
                    .unwrap()
                    .transform_point(screen_pt),
            }]
        }
        WindowEvent::MouseScroll(x, y) if !cx.modifiers.contains(Modifiers::CTRL) => {
            let (x, y) = if cx.modifiers.contains(Modifiers::SHIFT) {
                (y, x)
            } else {
                (x, y)
            };
            let screen_vec = ScreenVector::new(*x, *y) * SCROLL_SENSITIVITY;
            let map_vec = state
                .map_tab_unwrap()
                .transform
                .inverse()
                .unwrap()
                .transform_vector(screen_vec);
            vec![AppEvent::Pan {
                tab: state.current_tab,
                delta: map_vec,
            }]
        }
        WindowEvent::MouseDown(btn) if *btn == MouseButton::Left => {
            if room {
                if let Some(map) = state.loaded_maps.get(&state.map_tab_unwrap().id) {
                    let map_pt = state
                        .map_tab_unwrap()
                        .transform
                        .inverse()
                        .unwrap()
                        .transform_point(screen_pt)
                        .cast();
                    if let Some(idx) = map.level_at(map_pt) {
                        if idx != state.map_tab_unwrap().current_room {
                            return vec![AppEvent::SelectRoom {
                                tab: state.current_tab,
                                idx,
                            }];
                        }
                    }
                }
            }
            vec![]
        }
        _ => vec![],
    }
}
