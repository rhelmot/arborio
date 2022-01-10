use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer, TileFloat};
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
                self.draw(state, &tile_pos)
            }
            WindowEvent::MouseMove(..) if cx.mouse.left.state == MouseButtonState::Pressed => {
                self.draw(state, &tile_pos)
            }
            _ => vec![]
        }
    }
}

impl PencilTool {
    fn draw(&self, state: &AppState, tile_pos: &TilePoint) -> Vec<AppEvent> {
        match state.current_layer {
            Layer::Tiles(fg) => {
                let ch = if fg { state.current_fg_tile } else {state.current_bg_tile };
                vec![AppEvent::TileUpdate { fg, offset: *tile_pos, data: TileFloat {
                    tiles: vec![ch.id],
                    stride: 1,
                } }]
            }
            _ => vec![]
        }
    }
}
