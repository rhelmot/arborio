pub mod hand;

use crate::app_state::{AppState, AppEvent};

use vizia::*;

pub trait Tool {
    fn translate_event(&self, event: &WindowEvent, state: &AppState, cx: &Context) -> Option<AppEvent>;

    fn switch_on(&self, state: &mut AppState) { }
}