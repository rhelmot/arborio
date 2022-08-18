use crate::tabs::{build_tab_bar, build_tabs};
use arborio_state::data::app::{AppEvent, AppState};
use arborio_state::data::project_map::MapEvent;
use arborio_state::data::tabs::AppTab;
use arborio_state::lenses::{CurrentTabImplLens, IsFailedLens};
use arborio_utils::vizia::prelude::*;

pub fn main_widget(cx: &mut Context) {
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
            WindowEvent::KeyDown(Code::KeyM, _) if cx.modifiers == &Modifiers::ALT => {
                cx.emit(AppEvent::MapEvent {
                    map: None,
                    event: MapEvent::OpenMeta,
                });
            }
            _ => {}
        });
    });
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
        });
        Binding::new(cx, AppState::error_message, move |cx, error_message| {
            let error_message = error_message.get(cx);
            if !error_message.is_empty() {
                Label::new(cx, &error_message)
                    .id("error_message_bar")
                    .on_press(|cx| {
                        cx.emit(AppEvent::OpenLogsTab);
                    });
            }
        });
    })
    .id("main");
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
            MenuButton::new(
                cx,
                move |cx| {
                    Label::new(cx, "Map Metadata");
                },
                move |cx| {
                    cx.emit(AppEvent::MapEvent {
                        map: None,
                        event: MapEvent::OpenMeta,
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
