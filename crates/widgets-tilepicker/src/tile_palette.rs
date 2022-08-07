use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::tools::SCROLL_SENSITIVITY;
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use arborio_utils::vizia::vg::{Color, Paint, Path};

pub struct TilePaletteWidget {
    selected: u32,
    callback: Box<dyn Fn(&mut EventContext, u32)>,
}

impl TilePaletteWidget {
    pub fn new<F>(cx: &mut Context, selected: u32, callback: F) -> Handle<'_, Self>
    where
        F: 'static + Fn(&mut EventContext, u32),
    {
        Self {
            selected,
            callback: Box::new(callback),
        }
        .build(cx, move |_| {})
        .width(Units::Stretch(1.0))
        .height(Units::Stretch(1.0))
    }
}

impl View for TilePaletteWidget {
    fn element(&self) -> Option<&'static str> {
        Some("tile_palette")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        // TODO nuance
        cx.needs_redraw();

        let app = cx.data::<AppState>().unwrap();
        let entity = cx.current();
        let bounds = cx.cache.get_bounds(entity);
        let t = app.objtiles_transform;
        let t = t.then_translate(ScreenVector::new(bounds.x as f32, bounds.y as f32));
        let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let tinv = t.inverse().unwrap();
        let map_hovered = tinv.transform_point(screen_hovered);

        event.map(|msg, _| {
            match msg {
                WindowEvent::MouseScroll(x, y) => {
                    // TODO should we really be emitting these events directly...?
                    if cx.modifiers.contains(Modifiers::CTRL) {
                        cx.emit(AppEvent::ZoomObjectTiles {
                            delta: y.exp(),
                            focus: map_hovered,
                        });
                    } else {
                        let (x, y) = if cx.modifiers.contains(Modifiers::SHIFT) {
                            (y, x)
                        } else {
                            (x, y)
                        };
                        let screen_vec = ScreenVector::new(-*x, *y) * SCROLL_SENSITIVITY;
                        let map_vec = t.inverse().unwrap().transform_vector(screen_vec);
                        cx.emit(AppEvent::PanObjectTiles { delta: map_vec })
                    }
                }
                WindowEvent::MouseDown(MouseButton::Left) => {
                    let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
                    let map_hovered = t.inverse().unwrap().transform_point(screen_hovered);
                    let tile_hovered =
                        point_room_to_tile(&point_lose_precision(&map_hovered).cast_unit());
                    if tile_hovered.x < 0
                        || tile_hovered.x >= 32
                        || tile_hovered.y < 0
                        || tile_hovered.y >= 32
                    {
                        return;
                    }
                    let tile = (tile_hovered.x + tile_hovered.y * 32) as u32;
                    (self.callback)(cx, tile);
                }
                _ => {}
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let app = cx.data::<AppState>().unwrap();

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        //canvas.scissor(0.0, 0.0, bounds.w, bounds.h);

        let mut path = Path::new();
        path.rect(0.0, 0.0, bounds.w, bounds.h);
        canvas.fill_path(
            &mut path,
            Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                100.0,
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                },
            ),
        );

        let t = app.objtiles_transform;
        canvas.set_transform(t.m11, t.m12, t.m21, t.m22, t.m31.round(), t.m32.round());

        let palette = app.current_palette_unwrap();
        if let Err(e) = palette.gameplay_atlas.draw_sprite(
            canvas,
            "tilesets/scenery",
            Point2D::new(0.0, 0.0),
            None,
            Some(Vector2D::new(0.0, 0.0)),
            None,
            None,
            0.0,
        ) {
            log::error!("{}", e);
        }

        let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let screen_hovered = screen_hovered - ScreenVector::new(bounds.x, bounds.y);
        let map_hovered = t.inverse().unwrap().transform_point(screen_hovered);
        let tile_hovered = point_room_to_tile(&point_lose_precision(&map_hovered).cast_unit());
        let map_hovered_snapped = point_tile_to_room(&tile_hovered);

        let mut path = Path::new();
        path.rect(
            map_hovered_snapped.x as f32,
            map_hovered_snapped.y as f32,
            8.0,
            8.0,
        );
        canvas.fill_path(&mut path, Paint::color(Color::rgba(255, 255, 0, 128)));

        let map_selected_snapped = RoomPoint::new(
            ((self.selected % 32) * 8) as i32,
            ((self.selected / 32) * 8) as i32,
        );
        let mut path = Path::new();
        path.rect(
            map_selected_snapped.x as f32,
            map_selected_snapped.y as f32,
            8.0,
            8.0,
        );
        canvas.fill_path(&mut path, Paint::color(Color::rgba(100, 100, 255, 128)));

        canvas.restore();
    }
}
