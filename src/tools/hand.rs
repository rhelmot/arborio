use std::cell::RefCell;
use crate::app_state::{AppEvent, AppState};
use crate::tools::{Tool, generic_nav};
use crate::units::*;

use vizia::*;

#[derive(Default)]
pub struct HandTool {
    last_pos: Option<ScreenPoint>,
}

impl Tool for HandTool {
    fn name(&self) -> &'static str {
        "Hand"
    }

    fn new() -> Self {
        Self { last_pos: None }
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let scroll_events = generic_nav(event, state, cx);
        if scroll_events.len() != 0 {
            return scroll_events;
        }

        match event {
            WindowEvent::MouseDown(btn) if btn == &MouseButton::Left => {
                self.last_pos = Some(ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory));
                vec![]
            }
            WindowEvent::MouseUp(btn) if btn == &MouseButton::Left => {
                self.last_pos = None;
                vec![]
            }
            WindowEvent::MouseMove(x, y) if cx.mouse.left.state == MouseButtonState::Pressed => {
                let screen_pt = ScreenPoint::new(*x, *y);
                if self.last_pos.is_some() {
                    let screen_delta = screen_pt - self.last_pos.unwrap();
                    let map_pan = state.transform.inverse().unwrap().transform_vector(screen_delta);
                    self.last_pos = Some(screen_pt);
                    vec![AppEvent::Pan { delta: map_pan }]
                } else {
                    self.last_pos = Some(screen_pt);
                    vec![]
                }
            }
            _ => vec![]
        }
    }
}
