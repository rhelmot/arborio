mod logging;
#[cfg(test)]
mod tests;

use std::error::Error;

use crate::logging::setup_logger_thread;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::MapEvent;
use arborio_utils::vizia::prelude::*;
use arborio_widgets::main_widget::main_widget;

fn main() -> Result<(), Box<dyn Error>> {
    let icon_img = image::load_from_memory(include_bytes!("../img/icon.png")).unwrap();
    let (width, height) = (icon_img.width(), icon_img.height());
    let app = Application::new(|cx| {
        AppState::new().build(cx);
        cx.add_global_listener(|cx, event| {
            event.map(|window_event, _| match window_event {
                WindowEvent::KeyDown(Code::KeyZ, _) if cx.modifiers == &Modifiers::CTRL => {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Undo,
                    });
                }
                WindowEvent::KeyDown(Code::KeyY, _) if cx.modifiers == &Modifiers::CTRL => {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Redo,
                    });
                }
                WindowEvent::KeyDown(Code::KeyS, _) if cx.modifiers == &Modifiers::CTRL => {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Save,
                    });
                }
                _ => {}
            });
        });
        setup_logger_thread(cx);
        log::info!("Hello world!");
        if let Some(path) = &cx.data::<AppState>().unwrap().config.celeste_root {
            let path = path.clone();
            cx.emit(AppEvent::SetConfigPath { path });
        }
        //cx.add_theme(include_str!("style.css"));
        cx.add_stylesheet("src/style.css")
            .expect("Could not load stylesheet. Are you running me in the right directory?");

        cx.text_context().resize_shaping_run_cache(10000);

        main_widget(cx);
    })
    .title("Arborio")
    .icon(icon_img.into_bytes(), width, height)
    .ignore_default_theme();

    app.run();
    Ok(())
}
