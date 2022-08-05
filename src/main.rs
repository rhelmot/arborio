#![allow(clippy::too_many_arguments)]

mod app_state;
#[macro_use]
mod assets;
mod atlas_img;
mod auto_saver;
mod autotiler;
mod celeste_mod;
#[macro_use]
mod from_binel;
mod lenses;
mod logging;
mod map_struct;
mod tools;
mod units;
mod widgets;

use std::error::Error;
use vizia::prelude::*;

use crate::app_state::{AppEvent, AppState, AppTab, MapEvent};
use crate::lenses::{CurrentTabImplLens, IsFailedLens};
use crate::widgets::tabs::{build_tab_bar, build_tabs};

fn main() -> Result<(), Box<dyn Error>> {
    let icon_img = image::load_from_memory(include_bytes!("../img/icon.png")).unwrap();
    let (width, height) = (icon_img.width(), icon_img.height());
    let app = Application::new(|cx| {
        app_state::AppState::new().build(cx);
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
        log::info!("Hello world!");
        if let Some(path) = &cx.data::<AppState>().unwrap().config.celeste_root {
            let path = path.clone();
            cx.emit(AppEvent::SetConfigPath { path });
        }
        //cx.add_theme(include_str!("style.css"));
        cx.add_stylesheet("src/style.css")
            .expect("Could not load stylesheet. Are you running me in the right directory?");

        cx.text_context().resize_shaping_run_cache(10000);

        VStack::new(cx, move |cx| {
            MenuController::new(cx, false, |cx| {
                MenuStack::new_horizontal(cx, build_menu_bar).id("menu_bar");
            });
            build_tab_bar(cx);
            build_tabs(cx);

            Binding::new(cx, AppState::progress, move |cx, progress| {
                let progress = progress.get(cx);
                if progress.progress != 100 {
                    let status = format!("{}% - {}", progress.progress, progress.status);
                    let progress = progress.progress;
                    ZStack::new(cx, move |cx| {
                        Label::new(cx, &status)
                            .width(Units::Percentage(100.0))
                            .id("progress_bar_bg");
                        Label::new(cx, &status)
                            .width(Units::Percentage(progress as f32))
                            .id("progress_bar");
                    })
                    .id("progress_bar_container");
                }
            })
        })
        .id("main");
    })
    .title("Arborio")
    .icon(icon_img.into_bytes(), width, height)
    .ignore_default_theme();

    app.run();
    Ok(())
}

fn is_map() -> impl Lens<Target = bool> {
    IsFailedLens::new(CurrentTabImplLens {}.then(AppTab::map)).map(|b| !b)
}

fn build_menu_bar(cx: &mut Context) {
    Menu::new(
        cx,
        |cx| Label::new(cx, "File"),
        |cx| {
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Save Map");
                },
                move |cx| {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Save,
                    });
                },
            )
            .display(is_map());
        },
    );
    Menu::new(
        cx,
        |cx| Label::new(cx, "Edit"),
        |cx| {
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Undo");
                },
                move |cx| {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Undo,
                    });
                },
            )
            .display(is_map());
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Redo");
                },
                move |cx| {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::Redo,
                    });
                },
            )
            .display(is_map());
        },
    );
    Menu::new(
        cx,
        |cx| Label::new(cx, "View"),
        |cx| {
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Celeste Installation");
                },
                move |cx| {
                    cx.emit(AppEvent::OpenInstallationTab);
                },
            );
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Config Editor");
                },
                move |cx| {
                    cx.emit(AppEvent::OpenConfigEditorTab);
                },
            );
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Logs");
                },
                move |cx| {
                    cx.emit(AppEvent::OpenLogsTab);
                },
            );
        },
    );
}
