use std::cell::RefCell;
use crate::app_state::{AppEvent, AppState};
use crate::tools::Tool;
use crate::units::*;

use vizia::*;

#[derive(Default)]
pub struct HandTool {
    state: RefCell<HandToolState>,
}

// hack
#[derive(Default)]
struct HandToolState {
    last_pos: Option<ScreenPoint>,
}

impl Tool for HandTool {
    fn translate_event(&self, event: &WindowEvent, state: &AppState, cx: &Context) -> Option<AppEvent> {
        let mut tool_state = self.state.borrow_mut();
        match event {
            WindowEvent::MouseDown(btn) if btn == &MouseButton::Left => {
                tool_state.last_pos = Some(ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory));
                None
            }
            WindowEvent::MouseUp(btn) if btn == &MouseButton::Left => {
                tool_state.last_pos = None;
                None
            }
            WindowEvent::MouseMove(x, y) if cx.mouse.left.state == MouseButtonState::Pressed => {
                let screen_pt = ScreenPoint::new(*x, *y);
                if tool_state.last_pos.is_some() {
                    let screen_delta = screen_pt - tool_state.last_pos.unwrap();
                    let map_pan = state.transform.inverse().unwrap().transform_vector(screen_delta);
                    tool_state.last_pos = Some(screen_pt);
                    Some(AppEvent::Pan { delta: map_pan })
                } else {
                    tool_state.last_pos = Some(screen_pt);
                    None
                }
            }
            WindowEvent::MouseScroll(x, y) if cx.modifiers.contains(Modifiers::CTRL) => {
                let screen_pt = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
                Some(AppEvent::Zoom { delta: y.exp(), focus: state.transform.inverse().unwrap().transform_point(screen_pt) })
            }
            _ => None
        }
    }
}