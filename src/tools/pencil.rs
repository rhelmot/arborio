use vizia::*;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::assets;
use crate::config::entity_config::PencilBehavior;
use crate::map_struct::{CelesteMapDecal, CelesteMapEntity};
use crate::tools::{generic_nav, Tool};
use crate::units::*;
use crate::widgets::editor_widget;
use crate::widgets::palette_widget::{EntitySelectable, TriggerSelectable};

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

    fn event(&mut self, event: &WindowEvent, app: &AppState, cx: &Context) -> Vec<AppEvent> {
        let mut events = generic_nav(event, app, cx);
        if !events.is_empty() {
            return events;
        }

        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return events;
        };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        match event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.do_draw_start(app, room_pos);
                self.do_draw(app, room_pos)
            }
            WindowEvent::MouseMove(..) if cx.mouse.left.state == MouseButtonState::Pressed => {
                self.do_draw(app, room_pos)
            }
            WindowEvent::MouseUp(MouseButton::Left) => self.do_draw_finish(app, room_pos),
            _ => vec![],
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, app: &AppState, cx: &Context) {
        canvas.save();
        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return;
        };
        canvas.translate(room.bounds.origin.x as f32, room.bounds.origin.y as f32);
        canvas.intersect_scissor(
            0.0,
            0.0,
            room.bounds.size.width as f32,
            room.bounds.size.height as f32,
        );

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app.transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if app.snap {
            room_pos_snapped
        } else {
            room_pos
        };

        match app.current_layer {
            Layer::FgTiles | Layer::BgTiles => {
                let mut path = femtovg::Path::new();
                path.rect(
                    room_pos_snapped.x as f32,
                    room_pos_snapped.y as f32,
                    8.0,
                    8.0,
                );
                canvas.fill_path(
                    &mut path,
                    femtovg::Paint::color(femtovg::Color::rgba(255, 0, 255, 128)),
                );
            }
            Layer::Entities => {
                let tmp_entity = self.get_terminal_entity(app, app.current_entity, room_pos);
                canvas.set_global_alpha(0.5);
                editor_widget::draw_entity(app, canvas, &tmp_entity, &TileGrid::empty(), false, false);
            }
            Layer::Triggers => {
                let tmp_trigger = self.get_terminal_trigger(app, app.current_trigger, room_pos);
                canvas.set_global_alpha(0.5);
                editor_widget::draw_entity(app, canvas, &tmp_trigger, &TileGrid::empty(), false, true);
            }
            Layer::FgDecals | Layer::BgDecals => {
                let texture = "decals/".to_owned() + app.current_decal.0;
                if cx.mouse.left.state == MouseButtonState::Released {
                    canvas.set_global_alpha(0.5);
                }
                app.palette.gameplay_atlas.draw_sprite(
                    canvas,
                    &texture,
                    room_pos.cast().cast_unit(),
                    None,
                    None,
                    None,
                    None,
                    0.0,
                );
            }
            _ => {}
        }

        canvas.restore();
    }
}

impl PencilTool {
    fn do_draw_start(&mut self, app: &AppState, room_pos: RoomPoint) {
        match app.current_layer {
            Layer::Entities | Layer::Triggers => {
                let pencil = if app.current_layer == Layer::Triggers {
                    PencilBehavior::Rect
                } else {
                    app.current_entity.config(app).pencil
                };
                match pencil {
                    PencilBehavior::Line => {}
                    PencilBehavior::Node | PencilBehavior::Rect => {
                        let room_pos = if app.snap {
                            let tile_pos = point_room_to_tile(&room_pos);
                            point_tile_to_room(&tile_pos)
                        } else {
                            room_pos
                        };
                        self.reference_point = Some(room_pos);
                    }
                }
            }
            _ => {}
        }
    }

    fn do_draw(&mut self, app: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos = if app.snap {
            point_tile_to_room(&tile_pos)
        } else {
            room_pos
        };

        match app.current_layer {
            Layer::FgTiles | Layer::BgTiles => {
                let fg = app.current_layer == Layer::FgTiles;
                let ch = if fg {
                    app.current_fg_tile
                } else {
                    app.current_bg_tile
                };
                vec![AppEvent::TileUpdate {
                    fg,
                    offset: tile_pos,
                    data: TileGrid {
                        tiles: vec![ch.id],
                        stride: 1,
                    },
                }]
            }
            Layer::Entities if app.current_entity.config(app).pencil == PencilBehavior::Line => {
                match self.reference_point {
                    Some(last_draw) => {
                        let diff = (room_pos - last_draw).cast::<f32>().length();
                        if diff > app.draw_interval {
                            self.reference_point = Some(room_pos);
                            vec![AppEvent::EntityAdd {
                                entity: self.get_terminal_entity(app, app.current_entity, room_pos),
                                trigger: false,
                            }]
                        } else {
                            vec![]
                        }
                    }
                    None => {
                        self.reference_point = Some(room_pos);
                        vec![AppEvent::EntityAdd {
                            entity: self.get_terminal_entity(app, app.current_entity, room_pos),
                            trigger: false,
                        }]
                    }
                }
            }
            _ => vec![],
        }
    }

    fn do_draw_finish(&mut self, app: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let room_pos = if app.snap {
            point_tile_to_room(&point_room_to_tile(&room_pos))
        } else {
            room_pos
        };
        let result = match app.current_layer {
            Layer::Entities | Layer::Triggers => {
                let pencil = if app.current_layer == Layer::Triggers {
                    PencilBehavior::Rect
                } else {
                    app.current_entity.config(app).pencil
                };
                match pencil {
                    PencilBehavior::Line => vec![],
                    PencilBehavior::Node | PencilBehavior::Rect => {
                        if self.reference_point.is_some() {
                            let entity = if app.current_layer == Layer::Triggers {
                                self.get_terminal_trigger(app, app.current_trigger, room_pos)
                            } else {
                                self.get_terminal_entity(app, app.current_entity, room_pos)
                            };
                            vec![AppEvent::EntityAdd {
                                entity,
                                trigger: app.current_layer == Layer::Triggers,
                            }]
                        } else {
                            vec![]
                        }
                    }
                }
            }
            Layer::FgDecals | Layer::BgDecals => {
                vec![AppEvent::DecalAdd {
                    fg: app.current_layer == Layer::FgDecals,
                    decal: CelesteMapDecal {
                        id: 0,
                        x: room_pos.x,
                        y: room_pos.y,
                        scale_x: 1.0,
                        scale_y: 1.0,
                        texture: app.current_decal.0.to_owned(),
                    },
                }]
            }
            _ => vec![],
        };
        self.reference_point = None;
        result
    }

    fn get_terminal_entity(
        &self,
        app: &AppState,
        selectable: EntitySelectable,
        room_pos: RoomPoint,
    ) -> CelesteMapEntity {
        let config = selectable.config(app);
        match config.pencil {
            PencilBehavior::Line => selectable.instantiate(
                app,
                room_pos.x,
                room_pos.y,
                config.minimum_size_x as i32,
                config.minimum_size_y as i32,
                vec![],
            ),
            PencilBehavior::Node => {
                let ref_pos = self.reference_point.unwrap_or(room_pos);
                selectable.instantiate(
                    app,
                    ref_pos.x,
                    ref_pos.y,
                    config.minimum_size_x as i32,
                    config.minimum_size_y as i32,
                    vec![(room_pos.x, room_pos.y)],
                )
            }
            PencilBehavior::Rect => {
                let ref_pos = self.reference_point.unwrap_or(room_pos);
                let diff = room_pos - ref_pos;
                selectable.instantiate(app, ref_pos.x, ref_pos.y, diff.x, diff.y, vec![])
            }
        }
    }

    fn get_terminal_trigger(
        &self,
        app: &AppState,
        selectable: TriggerSelectable,
        room_pos: RoomPoint,
    ) -> CelesteMapEntity {
        let ref_pos = self.reference_point.unwrap_or(room_pos);
        let diff = room_pos - ref_pos;
        selectable.instantiate(app, ref_pos.x, ref_pos.y, diff.x, diff.y, vec![])
    }
}
