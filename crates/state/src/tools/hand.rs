use crate::tools::{generic_nav, Tool};
use arborio_utils::units::*;

use crate::data::app::{AppEvent, AppState};
use arborio_utils::vizia::prelude::*;

#[derive(Default)]
pub struct HandTool {
    last_pos: Option<ScreenPoint>,
}

impl HandTool {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for HandTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let state = cx.data::<AppState>().unwrap();
        let scroll_events = generic_nav(event, state, cx, true);
        if !scroll_events.is_empty() {
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
                    let map_pan = state
                        .map_tab_unwrap()
                        .transform
                        .inverse()
                        .unwrap()
                        .transform_vector(screen_delta);
                    self.last_pos = Some(screen_pt);
                    vec![AppEvent::Pan {
                        tab: state.current_tab,
                        delta: map_pan,
                    }]
                } else {
                    self.last_pos = Some(screen_pt);
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
