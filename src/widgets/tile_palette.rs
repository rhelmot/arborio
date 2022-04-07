use vizia::*;

use crate::logging::*;
use crate::tools::SCROLL_SENSITIVITY;
use crate::units::*;
use crate::{AppEvent, AppState};

pub struct TilePaletteWidget {
    selected: u32,
    callback: Box<dyn Fn(&mut Context, u32)>,
}

impl TilePaletteWidget {
    pub fn new<F>(cx: &mut Context, selected: u32, callback: F) -> Handle<'_, Self>
    where
        F: 'static + Fn(&mut Context, u32),
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
    fn element(&self) -> Option<String> {
        Some("tile_palette".to_owned())
    }

    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        cx.style.needs_redraw = true;

        let app = cx.data::<AppState>().unwrap();
        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);
        let t = app.objtiles_transform;
        let t = t.then_translate(ScreenVector::new(bounds.x as f32, bounds.y as f32));
        let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let tinv = t.inverse().unwrap();
        let map_hovered = tinv.transform_point(screen_hovered);

        if let Some(WindowEvent::MouseScroll(x, y)) = event.message.downcast() {
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

        if let Some(WindowEvent::MouseDown(MouseButton::Left)) = event.message.downcast() {
            let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
            let map_hovered = t.inverse().unwrap().transform_point(screen_hovered);
            let tile_hovered = point_room_to_tile(&point_lose_precision(&map_hovered).cast_unit());
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
    }

    fn draw(&self, cx: &mut Context, canvas: &mut Canvas) {
        let app = cx.data::<AppState>().unwrap();
        let entity = cx.current;
        let bounds = cx.cache.get_bounds(entity);

        canvas.save();
        canvas.translate(bounds.x, bounds.y);
        //canvas.scissor(0.0, 0.0, bounds.w, bounds.h);

        let mut path = vg::Path::new();
        path.rect(0.0, 0.0, bounds.w, bounds.h);
        canvas.fill_path(
            &mut path,
            vg::Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                100.0,
                Color::black().into(),
                Color::blue().into(),
            ),
        );

        let t = app.objtiles_transform;
        canvas.set_transform(t.m11, t.m12, t.m21, t.m22, t.m31.round(), t.m32.round());

        let palette = app.current_palette_unwrap();
        palette
            .gameplay_atlas
            .draw_sprite(
                canvas,
                "tilesets/scenery",
                Point2D::new(0.0, 0.0),
                None,
                Some(Vector2D::new(0.0, 0.0)),
                None,
                None,
                0.0,
            )
            .emit(LogLevel::Error, cx);

        let screen_hovered = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let screen_hovered = screen_hovered - ScreenVector::new(bounds.x, bounds.y);
        let map_hovered = t.inverse().unwrap().transform_point(screen_hovered);
        let tile_hovered = point_room_to_tile(&point_lose_precision(&map_hovered).cast_unit());
        let map_hovered_snapped = point_tile_to_room(&tile_hovered);

        let mut path = vg::Path::new();
        path.rect(
            map_hovered_snapped.x as f32,
            map_hovered_snapped.y as f32,
            8.0,
            8.0,
        );
        canvas.fill_path(
            &mut path,
            vg::Paint::color(vg::Color::rgba(255, 255, 0, 128)),
        );

        let map_selected_snapped = RoomPoint::new(
            ((self.selected % 32) * 8) as i32,
            ((self.selected / 32) * 8) as i32,
        );
        let mut path = vg::Path::new();
        path.rect(
            map_selected_snapped.x as f32,
            map_selected_snapped.y as f32,
            8.0,
            8.0,
        );
        canvas.fill_path(
            &mut path,
            vg::Paint::color(vg::Color::rgba(100, 100, 255, 128)),
        );

        canvas.restore();
    }
}
