use vizia::*;

use crate::tools::{generic_nav, Tool};
use crate::units::*;
use crate::{AppEvent, AppState, Context, WindowEvent};

pub struct StyleTool {
    status: Option<(MapPointStrict, MapPointStrict)>,
}

impl StyleTool {
    pub fn new(_app: &AppState) -> Self {
        Self { status: None }
    }
}

impl Tool for StyleTool {
    fn event(&mut self, event: &WindowEvent, cx: &mut Context) -> Vec<AppEvent> {
        let app = cx.data::<AppState>().unwrap();
        let events = generic_nav(event, app, cx, true);
        if !events.is_empty() {
            return events;
        }

        let screen_pos = ScreenPoint::new(cx.mouse.cursorx, cx.mouse.cursory);
        let map_pos_precise = app
            .map_tab_unwrap()
            .transform
            .inverse()
            .unwrap()
            .transform_point(screen_pos);
        let map_pos_unsnapped = point_lose_precision(&map_pos_precise);
        //let map_pos = (map_pos_unsnapped / 8) * 8;
        let map_tab = app.map_tab_unwrap();

        match event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.status = Some((map_pos_unsnapped, map_tab.preview_pos));
                vec![]
            }
            WindowEvent::MouseMove(_, _) => {
                if let Some((ptr_ref_pos, preview_ref_pos)) = self.status {
                    vec![AppEvent::MovePreview {
                        tab: app.current_tab,
                        pos: preview_ref_pos
                            + (map_pos_unsnapped.to_vector() - ptr_ref_pos.to_vector()),
                    }]
                } else {
                    vec![]
                }
            }
            WindowEvent::MouseUp(_) => {
                self.status = None;
                vec![]
            }
            _ => vec![],
        }
    }
}
