use vizia::*;

use crate::app_state::{AppEvent, AppState, TileFloat};
use crate::tools::{Tool, generic_nav};
use crate::units::*;

#[derive(Default)]
pub struct PencilTool {

}

impl Tool for PencilTool {
    fn name(&self) -> &'static str {
        "Pencil"
    }

    fn new() -> Self {
        Self {}
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let scroll_events = generic_nav(event, state, cx);
        if scroll_events.len() != 0 {
            return scroll_events;
        }

        let room = if let Some(room) = state.current_room_ref() { room} else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos: MapPointStrict = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let tile_pos = point_room_to_tile(&(map_pos - room.bounds.origin).to_point().cast_unit());
        match event {
            WindowEvent::MouseDown(btn) if btn == &MouseButton::Left => {
                vec![self.fg_draw(&tile_pos)]
            }
            WindowEvent::MouseMove(..) if cx.mouse.left.state == MouseButtonState::Pressed => {
                vec![self.fg_draw(&tile_pos)]
            }
            _ => vec![]
        }
    }
}

impl PencilTool {
    fn fg_draw(&self, tile_pos: &TilePoint) -> AppEvent {
        AppEvent::FgTileUpdate { offset: *tile_pos, data: TileFloat {
            tiles: vec!['0'],
            stride: 1,
        } }
    }
}
