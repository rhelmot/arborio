use std::time;
use std::cell::RefCell;
use vizia::*;

use crate::editor_widget;
use crate::map_struct;
use crate::map_struct::CelesteMap;
use crate::tools::Tool;
use crate::tools;
use crate::units::*;

#[derive(Lens)]
pub struct AppState {
    tools: Vec<Box<dyn Tool>>,
    pub current_tool: usize,
    pub map: Option<map_struct::CelesteMap>,
    pub transform: MapToScreen,
    pub last_draw: time::Instant,
}

#[derive(Debug)]
pub enum AppEvent {
    Load { map: RefCell<Option<CelesteMap>> },
    Pan { delta: MapVectorPrecise },
    Zoom { delta: f32, focus: MapPointPrecise },
}


impl Model for AppState {
    fn event(&mut self, cx: &mut Context, event: &mut Event) {
        if let Some(app_event) = event.message.downcast() {
            self.apply(app_event);
        }
    }
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            tools: vec![Box::new(tools::hand::HandTool::default())],
            current_tool: 0,
            map: None,
            transform: MapToScreen::identity(),
            last_draw: time::Instant::now(),
        }
    }

    pub fn apply(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Pan { delta } => {
                self.transform = self.transform.pre_translate(*delta);
            }
            AppEvent::Zoom { delta, focus } => {
                // TODO scale stepping
                self.transform = self.transform
                    .pre_translate(focus.to_vector())
                    .pre_scale(*delta, *delta)
                    .pre_translate(-focus.to_vector());
            }
            AppEvent::Load { map } => {
                let mut swapped: Option<CelesteMap> = None;
                std::mem::swap(&mut *map.borrow_mut(), &mut swapped);

                if swapped.is_some() {
                    self.map = swapped;
                }
            }
        }
    }

    pub fn tool(&self) -> &dyn Tool {
        self.tools[self.current_tool].as_ref()
    }

    pub fn tool_mut(&mut self) -> &mut dyn Tool {
        self.tools[self.current_tool].as_mut()
    }
}