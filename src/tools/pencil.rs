use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::entity_config::PencilBehavior;
use crate::map_struct::CelesteMapEntity;
use crate::palette_widget::EntitySelectable;
use crate::tools::{Tool, generic_nav};
use crate::units::*;
use crate::editor_widget;

#[derive(Default)]
pub struct PencilTool {
    reference_point: Option<RoomPoint>,
}

impl Tool for PencilTool {
    fn name(&self) -> &'static str {
        "Pencil"
    }

    fn new() -> Self {
        Self {
            reference_point: None,
        }
    }

    fn event(&mut self, event: &WindowEvent, state: &AppState, cx: &Context) -> Vec<AppEvent> {
        let mut events = generic_nav(event, state, cx);
        if events.len() > 0 { return events }

        let room = if let Some(room) = state.current_room_ref() { room} else { return events };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        match event {
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
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &Context) {
        canvas.save();
        let room = if let Some(room) = state.current_room_ref() { room } else { return };
        canvas.translate(room.bounds.origin.x as f32, room.bounds.origin.y as f32);
        canvas.intersect_scissor(0.0, 0.0, room.bounds.size.width as f32, room.bounds.size.height as f32);

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state.transform.inverse().unwrap().transform_point(screen_pos).cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap { room_pos_snapped } else { room_pos };

        match state.current_layer {
            Layer::FgTiles | Layer::BgTiles => {
                let mut path = femtovg::Path::new();
                path.rect(room_pos_snapped.x as f32, room_pos_snapped.y as f32, 8.0, 8.0);
                canvas.fill_path(&mut path, femtovg::Paint::color(femtovg::Color::rgba(255, 0, 255, 128)));
            }
            Layer::Entities => {
                let tmp_entity = self.get_terminal_entity(state.current_entity, room_pos);
                canvas.set_global_alpha(0.5);
                editor_widget::draw_entity(canvas, &tmp_entity, &TileGrid::empty());
            }
            _ => {}
        }

        canvas.restore();
    }
}

impl PencilTool {
    fn do_draw_start(&mut self, state: &AppState, room_pos: RoomPoint) {
        match state.current_entity.config.pencil {
            PencilBehavior::Line => {}
            PencilBehavior::Node | PencilBehavior::Rect => {
                let room_pos = if state.snap {
                    let tile_pos = point_room_to_tile(&room_pos);
                    point_tile_to_room(&tile_pos)
                } else {
                    room_pos
                };
                self.reference_point = Some(room_pos);
            }
        }
    }

    fn do_draw(&mut self, state: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos = if state.snap {
            point_tile_to_room(&tile_pos)
        } else {
            room_pos
        };

        match state.current_layer {
            Layer::FgTiles | Layer::BgTiles => {
                let fg = state.current_layer == Layer::FgTiles;
                let ch = if fg { state.current_fg_tile } else {state.current_bg_tile };
                vec![AppEvent::TileUpdate { fg, offset: tile_pos, data: TileGrid {
                    tiles: vec![ch.id],
                    stride: 1,
                } }]
            }
            Layer::Entities if state.current_entity.config.pencil == PencilBehavior::Line => {
                match self.reference_point {
                    Some(last_draw) => {
                        let diff = (room_pos - last_draw).cast::<f32>().length();
                        if diff > state.draw_interval {
                            self.reference_point = Some(room_pos);
                            vec![AppEvent::EntityAdd {
                                entity: self.get_terminal_entity(state.current_entity, room_pos)
                            }]
                        } else {
                            vec![]
                        }
                    }
                    None => {
                        self.reference_point = Some(room_pos);
                        vec![AppEvent::EntityAdd {
                            entity: self.get_terminal_entity(state.current_entity, room_pos)
                        }]
                    }
                }
            }
            _ => vec![]
        }
    }

    fn do_draw_finish(&mut self, state: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let room_pos = if state.snap {
            point_tile_to_room(&point_room_to_tile(&room_pos))
        } else {
            room_pos
        };
        let result = match state.current_layer {
            Layer::Entities => {
                match state.current_entity.config.pencil {
                    PencilBehavior::Line => vec![],
                    PencilBehavior::Node | PencilBehavior::Rect => {
                        if self.reference_point.is_some() {
                            vec![AppEvent::EntityAdd {
                                entity: self.get_terminal_entity(state.current_entity, room_pos)
                            }]
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

    fn get_terminal_entity(&self, selectable: EntitySelectable, room_pos: RoomPoint) -> CelesteMapEntity {
        match selectable.config.pencil {
            PencilBehavior::Line => selectable.instantiate(
                room_pos.x, room_pos.y,
                selectable.config.minimum_size_x as i32,
                selectable.config.minimum_size_y as i32,
                vec![]
            ),
            PencilBehavior::Node => {
                let ref_pos = self.reference_point.unwrap_or(room_pos);
                selectable.instantiate(
                    ref_pos.x, ref_pos.y,
                    selectable.config.minimum_size_x as i32,
                    selectable.config.minimum_size_y as i32,
                    vec![(room_pos.x, room_pos.y)]
                )
            },
            PencilBehavior::Rect => {
                let ref_pos = self.reference_point.unwrap_or(room_pos);
                let diff = room_pos - ref_pos;
                selectable.instantiate(
                    ref_pos.x, ref_pos.y,
                    diff.x, diff.y,
                    vec![]
                )
            },
        }
    }
}
