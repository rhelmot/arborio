mod logging;
#[cfg(test)]
mod tests;

use arborio_modloader::discovery::setup_loader_thread;
use std::error::Error;

use crate::logging::setup_logger_thread;
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::{AppConfigSetter, Progress};
use arborio_utils::resources::fonts::{DROID_SANS_MONO, RENOGARE};
use arborio_utils::vizia::prelude::*;
use arborio_widgets::main_widget::main_widget;

fn main() -> Result<(), Box<dyn Error>> {
    let icon_img = image::load_from_memory(include_bytes!("../icon.png")).unwrap();
    let (width, height) = (icon_img.width(), icon_img.height());
    let app = Application::new(|cx| {
        let tx = setup_loader_thread(
            cx,
            |p, s| AppEvent::Progress {
                progress: Progress {
                    progress: (p * 100.) as i32,
                    status: s,
                },
            },
            |modules| AppEvent::SetModules { modules },
            |modules| AppEvent::UpdateModules { modules },
        );
        AppState::new(tx).build(cx);
        setup_logger_thread(cx);
        log::info!("Hello world!");
        if let Some(path) = &cx.data::<AppState>().unwrap().config.celeste_root {
            let path = path.clone();
            cx.emit(AppEvent::EditSettings {
                setter: AppConfigSetter::CelesteRoot(Some(path)),
            });
        }
        #[cfg(not(debug_assertions))]
        cx.add_theme(include_str!("style.css"));
        #[cfg(debug_assertions)]
        cx.add_stylesheet("src/style.css")
            .expect("Could not load stylesheet. Are you running me in the right directory?");

        cx.add_fonts_mem(&[DROID_SANS_MONO, RENOGARE]);

        main_widget(cx);
    })
    .title("Arborio")
    .icon(icon_img.into_bytes(), width, height)
    .ignore_default_theme();

    app.run();
    Ok(())
}
