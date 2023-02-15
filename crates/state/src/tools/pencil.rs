use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Color, Paint, Path};

use crate::data::action::RoomAction;
use crate::data::app::{AppEvent, AppState};
use crate::data::{EventPhase, Layer};
use crate::palette_item::{
    get_entity_config, instantiate_decal, instantiate_entity, instantiate_trigger,
};
use crate::rendering;
use crate::rendering::draw_decal;
use crate::tools::{generic_nav, Tool};
use arborio_maploader::map_struct::{CelesteMapEntity, Node};
use arborio_modloader::config::PencilBehavior;
use arborio_modloader::selectable::{EntitySelectable, TriggerSelectable};
use arborio_utils::units::*;

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

impl Default for PencilTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for PencilTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut EventContext) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let events = generic_nav(event, app, cx, true);
        if !events.is_empty() {
            return events;
        }

        let Some(room) = app.current_room_ref() else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.data.bounds.origin).to_point().cast_unit();
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
        let Some(room) = app.current_room_ref() else { return vec![] };
        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.data.bounds.origin).to_point().cast_unit();

        self.do_draw_finish(app, room_pos)
    }

    fn draw(&mut self, canvas: &mut Canvas, state: &AppState, cx: &DrawContext) {
        let Some(room) = state.current_room_ref() else { return };
        canvas.save();
        canvas.translate(
            room.data.bounds.origin.x as f32,
            room.data.bounds.origin.y as f32,
        );
        canvas.intersect_scissor(
            0.0,
            0.0,
            room.data.bounds.size.width as f32,
            room.data.bounds.size.height as f32,
        );

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos = state
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos)
            .cast();
        let room_pos = (map_pos - room.data.bounds.origin).to_point().cast_unit();
        let tile_pos = point_room_to_tile(&room_pos);
        let room_pos_snapped = point_tile_to_room(&tile_pos);
        let room_pos = if state.config.snap {
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
                canvas.fill_path(&mut path, &Paint::color(Color::rgba(255, 0, 255, 128)));
            }
            Layer::Entities => {
                let tmp_entity = self.get_terminal_entity(state, state.current_entity, room_pos);
                canvas.set_global_alpha(0.5);
                rendering::draw_entity(
                    state
                        .current_palette_unwrap()
                        .get_entity_config(&tmp_entity.name, false),
                    state.current_palette_unwrap(),
                    canvas,
                    &tmp_entity,
                    &TileGrid::empty(),
                    false,
                    &room.data.object_tiles,
                );
            }
            Layer::Triggers => {
                let tmp_trigger = self.get_terminal_trigger(state, state.current_trigger, room_pos);
                canvas.set_global_alpha(0.5);
                rendering::draw_entity(
                    state
                        .current_palette_unwrap()
                        .get_entity_config(&tmp_trigger.name, true),
                    state.current_palette_unwrap(),
                    canvas,
                    &tmp_trigger,
                    &TileGrid::empty(),
                    false,
                    &TileGrid::empty(),
                );
            }
            Layer::FgDecals | Layer::BgDecals => {
                if cx.mouse.left.state == MouseButtonState::Released {
                    canvas.set_global_alpha(0.5);
                }
                let decal = instantiate_decal(
                    &state.current_decal,
                    &state.current_decal_other,
                    room_pos.x,
                    room_pos.y,
                    1.0,
                    1.0,
                );
                draw_decal(state.current_palette_unwrap(), canvas, &decal);
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
                    get_entity_config(&app.current_entity, app).pencil
                };
                match pencil {
                    PencilBehavior::Line => {}
                    PencilBehavior::Node | PencilBehavior::Rect => {
                        let room_pos = if app.config.snap {
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
        let room_pos = if app.config.snap {
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
                let other = if fg {
                    &app.current_fg_tile_other
                } else {
                    &app.current_bg_tile_other
                }
                .chars()
                .next()
                .unwrap_or('0');
                let ch_id = if ch.id == '\0' { other } else { ch.id };
                if let Some(start) = self.reference_point {
                    let mut result = vec![];
                    for step in steps(point_room_to_tile(&start), tile_pos, 1) {
                        result.push(app.room_action(
                            RoomAction::TileUpdate {
                                fg,
                                offset: step,
                                data: TileGrid {
                                    tiles: vec![ch_id],
                                    stride: 1,
                                },
                            },
                            self.draw_phase,
                        ));
                        self.reference_point = Some(point_tile_to_room(&step));
                    }
                    result
                } else {
                    self.reference_point = Some(room_pos);
                    vec![app.room_action(
                        RoomAction::TileUpdate {
                            fg,
                            offset: tile_pos,
                            data: TileGrid {
                                tiles: vec![ch_id],
                                stride: 1,
                            },
                        },
                        self.draw_phase,
                    )]
                }
            }
            Layer::Entities
                if get_entity_config(&app.current_entity, app).pencil == PencilBehavior::Line =>
            {
                match self.reference_point {
                    Some(last_draw) => {
                        let mut result = vec![];
                        let mut last_step = None;
                        for step in steps(last_draw, room_pos, app.config.draw_interval as i32) {
                            let step = if app.config.snap {
                                point_tile_to_room(&point_room_to_tile(&step))
                            } else {
                                step
                            };
                            if last_step == Some(step) {
                                continue;
                            }
                            last_step = Some(step);
                            result.push(app.room_action(
                                RoomAction::EntityAdd {
                                    entity: Box::new(self.get_terminal_entity(
                                        app,
                                        app.current_entity,
                                        step,
                                    )),
                                    trigger: false,
                                    genid: true,
                                },
                                self.draw_phase,
                            ));
                            self.reference_point = Some(step);
                        }
                        result
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
        let room_pos = if app.config.snap {
            point_tile_to_room(&point_room_to_tile(&room_pos))
        } else {
            room_pos
        };
        let result = match app.current_layer {
            Layer::Entities | Layer::Triggers => {
                let pencil = if app.current_layer == Layer::Triggers {
                    PencilBehavior::Rect
                } else {
                    get_entity_config(&app.current_entity, app).pencil
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
                        decal: Box::new(instantiate_decal(
                            &app.current_decal,
                            &app.current_decal_other,
                            room_pos.x,
                            room_pos.y,
                            1.0,
                            1.0,
                        )),
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
        let config = get_entity_config(&selectable, app);
        let other = &app.current_entity_other;
        match config.pencil {
            PencilBehavior::Line => instantiate_entity(
                &selectable,
                other,
                app,
                room_pos.x,
                room_pos.y,
                config.minimum_size_x as i32,
                config.minimum_size_y as i32,
                vec![],
            ),
            PencilBehavior::Node => {
                let ref_pos = self.reference_point.unwrap_or(room_pos);
                instantiate_entity(
                    &selectable,
                    other,
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
                instantiate_entity(
                    &selectable,
                    other,
                    app,
                    ref_pos.x,
                    ref_pos.y,
                    diff.x,
                    diff.y,
                    vec![],
                )
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
        instantiate_trigger(
            &selectable,
            &app.current_trigger_other,
            app,
            ref_pos.x,
            ref_pos.y,
            diff.x,
            diff.y,
            vec![],
        )
    }
}

fn steps<U>(from: Point2D<i32, U>, to: Point2D<i32, U>, step: i32) -> Vec<Point2D<i32, U>> {
    let vec = (to - from).cast::<f32>();
    let step_vec = vec.normalize() * step as f32;
    let length = vec.length();
    let mut result = vec![];
    let mut cursor = from.cast::<f32>();
    let mut accumulated_length = 0.0;

    loop {
        cursor += step_vec;
        accumulated_length += step as f32;
        if accumulated_length > length {
            break;
        }
        result.push(cursor.cast());
    }

    result
}
