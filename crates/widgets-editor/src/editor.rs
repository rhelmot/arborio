use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Color, ImageFlags, Paint, Path, PixelFormat, RenderTarget};
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::env;
use std::time;

use arborio_state::data::app::AppState;
use arborio_state::rendering;

lazy_static! {
    static ref PERF_MONITOR: bool = env::var("ARBORIO_PERF_MONITOR").is_ok();
}

const BACKDROP_COLOR: Color = Color {
    r: 0.30,
    g: 0.30,
    b: 0.30,
    a: 1.00,
};
const FILLER_COLOR: Color = Color {
    r: 0.40,
    g: 0.40,
    b: 0.40,
    a: 1.00,
};
const ROOM_EMPTY_COLOR: Color = Color {
    r: 0.13,
    g: 0.25,
    b: 0.13,
    a: 1.00,
};
const ROOM_DESELECTED_COLOR: Color = Color {
    r: 0.00,
    g: 0.00,
    b: 0.00,
    a: 0.30,
};

pub struct EditorWidget {}

impl EditorWidget {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {}.build(cx, |cx| {
            cx.focus();
        })
    }
}

impl View for EditorWidget {
    fn element(&self) -> Option<&'static str> {
        Some("arborio_editor")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _| {
            if let WindowEvent::SetCursor(_) = window_event {
                return;
            }

            // TODO: nuance
            cx.needs_redraw();

            if let WindowEvent::MouseDown(..) = &window_event {
                cx.focus();
            }
            let app = cx
                .data::<AppState>()
                .expect("EditorWidget must have an AppState in its ancestry");
            let tool = app.current_tool.borrow_mut().take();
            let (events, cursor) = match tool {
                Some(mut tool) => {
                    let r = (tool.event(window_event, cx), tool.cursor(cx));
                    *cx.data::<AppState>().unwrap().current_tool.borrow_mut() = Some(tool);
                    r
                }
                None => (vec![], CursorIcon::Default),
            };
            for event in events {
                cx.emit(event);
            }
            cx.emit(WindowEvent::SetCursor(cursor));
        });
        event.map(|internal_event, _| {
            let app = cx
                .data::<AppState>()
                .expect("EditorWidget must have an AppState in its ancestry");
            let tool = app.current_tool.borrow_mut().take();
            let events = if let Some(mut tool) = tool {
                let r = tool.internal_event(internal_event, cx);
                *cx.data::<AppState>().unwrap().current_tool.borrow_mut() = Some(tool);
                r
            } else {
                vec![]
            };
            for event in events {
                cx.emit(event);
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let app = cx
            .data::<AppState>()
            .expect("EditorWidget must have an AppState in its ancestry");
        canvas.clear_rect(
            bounds.x as u32,
            bounds.y as u32,
            bounds.w as u32,
            bounds.h as u32,
            BACKDROP_COLOR,
        );
        if !app.map_tab_check() {
            // I am worried there are corner cases in vizia where data may hold stale references
            // for a single frame. if this is the case, I'd like to be able to see that via a single
            // debug print vs many debug prints.
            dbg!(&app.tabs, app.current_tab);
            println!("SOMETHING IS WRONG (editor)");
            return;
        }
        let t = &app.map_tab_unwrap().transform;
        canvas.set_transform(t.m11, t.m12, t.m21, t.m22, t.m31.round(), t.m32.round());

        let map = app.loaded_maps.get(&app.map_tab_unwrap().id).unwrap();
        if *PERF_MONITOR {
            let now = time::Instant::now();
            println!("Drew {}ms ago", (now - *app.last_draw.borrow()).as_millis());
            *app.last_draw.borrow_mut() = now;
        }

        let current_room = app.map_tab_unwrap().current_room;
        let preview = app.map_tab_unwrap().preview_pos;

        let mut path = Path::new();
        for room in &map.data.levels {
            path.rect(
                room.data.bounds.origin.x as f32,
                room.data.bounds.origin.y as f32,
                room.data.bounds.width() as f32,
                room.data.bounds.height() as f32,
            );
        }
        canvas.fill_path(&mut path, &Paint::color(ROOM_EMPTY_COLOR));

        let mut path = Path::new();
        path.rect(preview.x as f32, preview.y as f32, 320.0, 180.0);
        canvas.stroke_path(
            &mut path,
            &Paint::color(Color::black()).with_line_width(2.0),
        );

        canvas.save();
        canvas.intersect_scissor(preview.x as f32, preview.y as f32, 320.0, 180.0);
        rendering::draw_stylegrounds(
            app.current_palette_unwrap(),
            canvas,
            preview,
            map.data.backgrounds.as_slice(),
            map.data
                .levels
                .get(current_room)
                .map_or("", |lvl| lvl.data.name.as_str()),
            &HashSet::new(),
            false,
        );
        canvas.restore();

        let mut path = Path::new();
        for filler in &map.data.filler {
            path.rect(
                filler.origin.x as f32,
                filler.origin.y as f32,
                filler.width() as f32,
                filler.height() as f32,
            );
        }
        canvas.fill_path(&mut path, &Paint::color(FILLER_COLOR));

        for (idx, room) in map.data.levels.iter().enumerate() {
            canvas.save();
            canvas.translate(
                room.data.bounds.min_x() as f32,
                room.data.bounds.min_y() as f32,
            );
            let mut cache = room.cache.borrow_mut();
            let target = cache.render_cache.unwrap_or_else(|| {
                canvas
                    .create_image_empty(
                        room.data.bounds.width() as usize,
                        room.data.bounds.height() as usize,
                        PixelFormat::Rgba8,
                        ImageFlags::NEAREST | ImageFlags::FLIP_Y,
                    )
                    .expect("Failed to allocate ")
            });
            cache.render_cache = Some(target);

            if !cache.render_cache_valid {
                canvas.save();
                canvas.reset();
                canvas.set_render_target(RenderTarget::Image(target));

                canvas.clear_rect(
                    0,
                    0,
                    room.data.bounds.width() as u32,
                    room.data.bounds.height() as u32,
                    Color::rgba(0, 0, 0, 0),
                );
                rendering::draw_tiles(app.current_palette_unwrap(), canvas, room, false);
                rendering::draw_decals(app.current_palette_unwrap(), canvas, &room.data, false);
                rendering::draw_triggers(
                    app.current_palette_unwrap(),
                    canvas,
                    &room.data,
                    if idx == app.map_tab_unwrap().current_room {
                        app.map_tab_unwrap().current_selected
                    } else {
                        None
                    },
                );
                rendering::draw_entities(
                    app.current_palette_unwrap(),
                    canvas,
                    &room.data,
                    if idx == app.map_tab_unwrap().current_room {
                        app.map_tab_unwrap().current_selected
                    } else {
                        None
                    },
                );
                rendering::draw_tiles(app.current_palette_unwrap(), canvas, room, true);
                rendering::draw_decals(app.current_palette_unwrap(), canvas, &room.data, true);
                rendering::draw_objtiles_float(app.current_palette_unwrap(), canvas, room);

                canvas.restore();
                canvas.set_render_target(RenderTarget::Screen);
                cache.render_cache_valid = true;
            }

            let mut path = Path::new();
            path.rect(
                0.0,
                0.0,
                room.data.bounds.width() as f32,
                room.data.bounds.height() as f32,
            );
            let paint = Paint::image(
                target,
                0.0,
                0.0,
                room.data.bounds.width() as f32,
                room.data.bounds.height() as f32,
                0.0,
                1.0,
            );
            canvas.fill_path(&mut path, &paint);
            if idx != current_room {
                canvas.fill_path(&mut path, &Paint::color(ROOM_DESELECTED_COLOR));
            }
            canvas.restore();
        }

        canvas.save();
        canvas.intersect_scissor(preview.x as f32, preview.y as f32, 320.0, 180.0);
        rendering::draw_stylegrounds(
            app.current_palette_unwrap(),
            canvas,
            preview,
            map.data.foregrounds.as_slice(),
            map.data
                .levels
                .get(current_room)
                .map_or("", |lvl| lvl.data.name.as_str()),
            &HashSet::new(),
            false,
        );
        canvas.restore();

        let tool = { app.current_tool.borrow_mut().take() };
        if let Some(mut tool) = tool {
            tool.draw(canvas, app, cx);
            *app.current_tool.borrow_mut() = Some(tool);
        }
    }
}
