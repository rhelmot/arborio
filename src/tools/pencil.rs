use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer, TileFloat};
use crate::entity_config::PencilBehavior;
use crate::tools::{Tool, generic_nav};
use crate::units::*;

#[derive(Default)]
pub struct PencilTool {
    pub interval: f32,
    pub snap: bool,

    reference_point: Option<RoomPoint>,
}

impl Tool for PencilTool {
    fn name(&self) -> &'static str {
        "Pencil"
    }

    fn new() -> Self {
        Self {
            interval: 4.0,
            snap: true,
            reference_point: None,
        }
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let mut events = generic_nav(event, state, cx);

        let room = if let Some(room) = state.current_room_ref() { room} else { return events };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        events.extend(match event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.do_draw_start(state, room_pos);
                self.do_draw(state, room_pos)
            }
            WindowEvent::MouseMove(..) if cx.mouse.left.state == MouseButtonState::Pressed => {
                self.do_draw(state, room_pos)
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                self.do_draw_finish(state, room_pos)
            }
            _ => vec![]
        });
        events
    }
}

impl PencilTool {
    fn do_draw_start(&mut self, state: &AppState, room_pos: RoomPoint) {
        match state.current_entity.config.pencil {
            PencilBehavior::Line => {}
            PencilBehavior::Node | PencilBehavior::Rect => {
                self.reference_point = Some(room_pos);
            }
        }
    }

    fn do_draw(&mut self, state: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let tile_pos = point_room_to_tile(&room_pos);
        match state.current_layer {
            Layer::Tiles(fg) => {
                let ch = if fg { state.current_fg_tile } else {state.current_bg_tile };
                vec![AppEvent::TileUpdate { fg, offset: tile_pos, data: TileFloat {
                    tiles: vec![ch.id],
                    stride: 1,
                } }]
            }
            Layer::Entities if state.current_entity.config.pencil == PencilBehavior::Line => {
                let room_pos = if self.snap {
                    point_tile_to_room(&tile_pos)
                } else {
                    room_pos
                };
                match self.reference_point {
                    Some(last_draw) => {
                        let diff = (room_pos - last_draw).cast::<f32>().length();
                        if diff > self.interval {
                            self.reference_point = Some(room_pos);
                            vec![AppEvent::EntityAdd { entity: state.current_entity.instantiate(
                                room_pos.x, room_pos.y,
                                state.current_entity.config.minimum_size_x as i32,
                                state.current_entity.config.minimum_size_y as i32,
                                vec![]
                            )}]
                        } else {
                            vec![]
                        }
                    }
                    None => {
                        self.reference_point = Some(room_pos);
                        vec![AppEvent::EntityAdd { entity: state.current_entity.instantiate(
                            room_pos.x, room_pos.y,
                            state.current_entity.config.minimum_size_x as i32,
                            state.current_entity.config.minimum_size_y as i32,
                            vec![]
                        )}]
                    }
                }
            }
            _ => vec![]
        }
    }

    fn do_draw_finish(&mut self, state: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let room_pos = if self.snap {
            point_tile_to_room(&point_room_to_tile(&room_pos))
        } else {
            room_pos
        };
        let result = match state.current_layer {
            Layer::Entities => {
                match state.current_entity.config.pencil {
                    PencilBehavior::Line => vec![],
                    PencilBehavior::Node => {
                        if let Some(ref_pos) = self.reference_point {
                            vec![AppEvent::EntityAdd { entity: state.current_entity.instantiate(
                                ref_pos.x, ref_pos.y,
                                state.current_entity.config.minimum_size_x as i32,
                                state.current_entity.config.minimum_size_y as i32,
                                vec![(room_pos.x, room_pos.y)]
                            )}]
                        } else {
                            vec![]
                        }
                    }
                    PencilBehavior::Rect => {
                        if let Some(ref_pos) = self.reference_point {
                            let diff = dbg!(room_pos - ref_pos);
                            vec![AppEvent::EntityAdd { entity: dbg!(state.current_entity.instantiate(
                                ref_pos.x, ref_pos.y,
                                diff.x, diff.y,
                                vec![]
                            ))}]
                        } else {
                            vec![]
                        }

                    }
                }
            }
            _ => vec![]
        };
        self.reference_point = None;
        result
    }
}
