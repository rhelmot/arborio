use crate::data::action::RoomAction;
use crate::data::app::{AppEvent, AppState};
use crate::data::project_map::LevelState;
use crate::data::{EventPhase, Layer};
use crate::tools::selection::add_to_float;
use crate::tools::{generic_nav, Tool};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Color, Paint, Path};
use std::collections::{HashSet, VecDeque};

#[derive(Default)]
pub struct BucketTool {}

impl Tool for BucketTool {
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
        let tile_pos = point_room_to_tile(&room_pos);

        if matches!(event, WindowEvent::MouseDown(MouseButton::Left))
            && matches!(app.current_layer, Layer::FgTiles | Layer::BgTiles)
        {
            let fg = matches!(app.current_layer, Layer::FgTiles);
            let tiles = bucket_it(room, fg, tile_pos);
            let mut result_float = None;
            let ch = if fg {
                app.current_fg_tile.id
            } else {
                app.current_bg_tile.id
            };
            for tile in tiles {
                add_to_float(&mut result_float, tile, Some(&ch), '\0');
            }
            if let Some((offset, data)) = result_float {
                vec![app.current_map_id().unwrap().room_action(
                    app.map_tab_unwrap().current_room,
                    EventPhase::new(),
                    RoomAction::TileUpdate { fg, offset, data },
                )]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
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
        if !matches!(state.current_layer, Layer::FgTiles | Layer::BgTiles) {
            canvas.restore();
            return;
        }

        let tiles = bucket_it(
            room,
            matches!(state.current_layer, Layer::FgTiles),
            tile_pos,
        );
        let mut path = Path::new();
        for tile in tiles {
            let room_pt = point_tile_to_room(&tile);
            path.rect(room_pt.x as f32, room_pt.y as f32, 8.0, 8.0);
        }
        canvas.fill_path(&mut path, &Paint::color(Color::rgba(255, 0, 255, 128)));
        canvas.restore();
    }
}

impl BucketTool {
    pub fn new() -> Self {
        Self::default()
    }
}

fn bucket_it(room: &LevelState, fg: bool, tile_pos: TilePoint) -> HashSet<TilePoint> {
    let Some(desired_char) = room.tile(tile_pos, fg) else { return HashSet::new() };

    let mut result = HashSet::new();
    let mut queue = VecDeque::from([tile_pos]);
    while let Some(pt) = queue.pop_front() {
        if room.tile(pt, fg) != Some(desired_char) {
            continue;
        }
        if !result.insert(pt) {
            continue;
        }
        let children = [
            pt + TileVector::new(1, 0),
            pt + TileVector::new(0, 1),
            pt + TileVector::new(-1, 0),
            pt + TileVector::new(0, -1),
        ];
        queue.extend(children.into_iter());
    }
    result
}
