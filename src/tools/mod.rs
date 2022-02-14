pub mod hand;
pub mod pencil;
pub mod selection;

use lazy_static::lazy_static;
use std::sync::Mutex;
use vizia::*;

use crate::app_state::{AppEvent, AppState};
use crate::units::*;

pub trait Tool: Send {
    fn name(&self) -> &'static str;

    fn new() -> Self
    where
        Self: Sized;

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent>;

    fn switch_on(&mut self) {}

    fn draw(&mut self, _canvas: &mut Canvas, _state: &AppState, _cx: &Context) {}

    fn cursor(&self, _cx: &Context, _state: &AppState) -> CursorIcon {
        CursorIcon::Default
    }
}

lazy_static! {
    pub static ref TOOLS: Mutex<[Box<dyn Tool>; 3]> = {
        Mutex::new([
            Box::new(hand::HandTool::new()),
            Box::new(selection::SelectionTool::new()),
            Box::new(pencil::PencilTool::new()),
        ])
    };
}

pub const SCROLL_SENSITIVITY: f32 = 35.0;

pub fn generic_nav(event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
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
            let screen_vec = ScreenVector::new(-*x, *y) * SCROLL_SENSITIVITY;
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
            vec![]
        }
        _ => vec![],
    }
}
