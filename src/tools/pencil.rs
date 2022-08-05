use vizia::prelude::*;
use vizia::vg::{Color, Paint, Path};

use crate::app_state::{AppEvent, AppState, EventPhase, Layer, RoomAction};
use crate::celeste_mod::config::PencilBehavior;
use crate::map_struct::{CelesteMapDecal, CelesteMapEntity, Node};
use crate::tools::{generic_nav, Tool};
use crate::units::*;
use crate::widgets::editor;
use crate::widgets::list_palette::{EntitySelectable, TriggerSelectable};

pub struct PencilTool {
    reference_point: Option<RoomPoint>,
    draw_phase: EventPhase,
}

impl PencilTool {
    pub fn new() -> Self {
        Self {
            reference_point: None,
            draw_phase: EventPhase::null(),
        }
    }
}

impl Tool for PencilTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let events = generic_nav(event, app, cx, true);
        if !events.is_empty() {
            return events;
        }

        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return vec![];
        };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app
            .map_tab_unwrap()
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

    fn switch_off(&mut self, app: &AppState, cx: &EventContext) -> Vec<AppEvent> {
        let room = if let Some(room) = app.current_room_ref() {
            room
        } else {
            return vec![];
        };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();

        self.do_draw_finish(app, room_pos)
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {
        let room = if let Some(room) = state.current_room_ref() {
            room
        } else {
            return;
        };
        canvas.save();
        canvas.translate(room.bounds.origin.x as f32, room.bounds.origin.y as f32);
        canvas.intersect_scissor(
            0.0,
            0.0,
            room.bounds.size.width as f32,
            room.bounds.size.height as f32,
        );

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.snap {
            room_pos_snapped
        } else {
            room_pos
        };

        match state.current_layer {
            Layer::FgTiles | Layer::BgTiles | Layer::ObjectTiles => {
                let mut path = Path::new();
                path.rect(
                    room_pos_snapped.x as f32,
                    room_pos_snapped.y as f32,
                    8.0,
                    8.0,
                );
                canvas.fill_path(&mut path, Paint::color(Color::rgba(255, 0, 255, 128)));
            }
            Layer::Entities => {
                let tmp_entity = self.get_terminal_entity(state, state.current_entity, room_pos);
                canvas.set_global_alpha(0.5);
                editor::draw_entity(
                    state,
                    canvas,
                    &tmp_entity,
                    &TileGrid::empty(),
                    false,
                    false,
                    &room.object_tiles,
                );
            }
            Layer::Triggers => {
                let tmp_trigger = self.get_terminal_trigger(state, state.current_trigger, room_pos);
                canvas.set_global_alpha(0.5);
                editor::draw_entity(
                    state,
                    canvas,
                    &tmp_trigger,
                    &TileGrid::empty(),
                    false,
                    true,
                    &TileGrid::empty(),
                );
            }
            Layer::FgDecals | Layer::BgDecals => {
                let texture = format!("decals/{}", state.current_decal.0);
                if cx.mouse.left.state == MouseButtonState::Released {
                    canvas.set_global_alpha(0.5);
                }
                if let Err(e) = state.current_palette_unwrap().gameplay_atlas.draw_sprite(
                    canvas,
                    &texture,
                    room_pos.cast().cast_unit(),
                    None,
                    None,
                    None,
                    None,
                    0.0,
                ) {
                    log::error!("{}", e);
                }
            }
            _ => {}
        }

        canvas.restore();
    }
}

impl PencilTool {
    fn do_draw_start(&mut self, app: &AppState, room_pos: RoomPoint) {
        self.draw_phase = EventPhase::new();
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

    // TODO test to see if the diff would do anything before sending an event
    fn do_draw(&mut self, app: &AppState, room_pos: RoomPoint) -> Vec<AppEvent> {
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos = if app.snap {
            point_tile_to_room(&tile_pos)
        } else {
            room_pos
        };

        match app.current_layer {
            Layer::ObjectTiles => {
                vec![app.room_action(
                    RoomAction::ObjectTileUpdate {
                        offset: tile_pos,
                        data: TileGrid {
                            tiles: vec![app.current_objtile as i32],
                            stride: 1,
                        },
                    },
                    self.draw_phase,
                )]
            }
            Layer::FgTiles | Layer::BgTiles => {
                let fg = app.current_layer == Layer::FgTiles;
                let ch = if fg {
                    app.current_fg_tile
                } else {
                    app.current_bg_tile
                };
                vec![app.room_action(
                    RoomAction::TileUpdate {
                        fg,
                        offset: tile_pos,
                        data: TileGrid {
                            tiles: vec![ch.id],
                            stride: 1,
                        },
                    },
                    self.draw_phase,
                )]
            }
            Layer::Entities if app.current_entity.config(app).pencil == PencilBehavior::Line => {
                match self.reference_point {
                    Some(last_draw) => {
                        let diff = (room_pos - last_draw).cast::<f32>().length();
                        if diff > app.draw_interval {
                            self.reference_point = Some(room_pos);
                            vec![app.room_action(
                                RoomAction::EntityAdd {
                                    entity: Box::new(self.get_terminal_entity(
                                        app,
                                        app.current_entity,
                                        room_pos,
                                    )),
                                    trigger: false,
                                    selectme: false,
                                    genid: true,
                                },
                                self.draw_phase,
                            )]
                        } else {
                            vec![]
                        }
                    }
                    None => {
                        self.reference_point = Some(room_pos);
                        vec![app.room_action(
                            RoomAction::EntityAdd {
                                entity: Box::new(self.get_terminal_entity(
                                    app,
                                    app.current_entity,
                                    room_pos,
                                )),
                                trigger: false,
                                selectme: false,
                                genid: true,
                            },
                            self.draw_phase,
                        )]
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
                            vec![app.room_action(
                                RoomAction::EntityAdd {
                                    entity: Box::new(entity),
                                    trigger: app.current_layer == Layer::Triggers,
                                    selectme: false,
                                    genid: true,
                                },
                                self.draw_phase,
                            )]
                        } else {
                            vec![]
                        }
                    }
                }
            }
            Layer::FgDecals | Layer::BgDecals => {
                vec![app.room_action(
                    RoomAction::DecalAdd {
                        fg: app.current_layer == Layer::FgDecals,
                        decal: Box::new(CelesteMapDecal {
                            id: 0,
                            x: room_pos.x,
                            y: room_pos.y,
                            scale_x: 1.0,
                            scale_y: 1.0,
                            texture: app.current_decal.0.to_string(),
                        }),
                        selectme: false,
                        genid: true,
                    },
                    self.draw_phase,
                )]
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
                    vec![Node {
                        x: room_pos.x,
                        y: room_pos.y,
                    }],
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
