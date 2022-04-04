pub mod config_editor;
pub mod editor;
pub mod installation;
pub mod logs;
pub mod project;

use crate::app_state::AppState;
use crate::app_state::AppTab;
use crate::lenses::VecIndexWithLens;
use crate::AppEvent;
use vizia::*;

pub fn build_tabs(cx: &mut Context) {
    let lens = VecIndexWithLens::new(AppState::tabs, AppState::current_tab);
    Binding::new(cx, lens, move |cx, current_tab| {
        if let Some(current_tab) = current_tab.get_fallible(cx) {
            VStack::new(cx, move |cx| match current_tab {
                AppTab::CelesteOverview => {
                    installation::build_installation_tab(cx);
                }
                AppTab::ProjectOverview(project) => project::build_project_tab(cx, project),
                AppTab::Map(_) => {
                    editor::build_editor(cx);
                }
                AppTab::ConfigEditor(_) => {
                    config_editor::build_config_editor(cx);
                }
                AppTab::Logs => {
                    logs::build_logs(cx);
                }
            })
            .class("tab_container");
        }
    });
}

pub fn build_tab_bar(cx: &mut Context) {
    List::new(cx, AppState::tabs, move |cx, tab_index, tab| {
        HStack::new(cx, move |cx| {
            Label::new(cx, &tab.get(cx).to_string());
            Label::new(cx, "x").class("close_btn").on_press(move |cx| {
                cx.emit(AppEvent::CloseTab { idx: tab_index });
            });
        })
        .class("tab")
        .on_press(move |cx| {
            cx.emit(AppEvent::SelectTab { idx: tab_index });
        })
        .bind(AppState::current_tab, move |handle, current_tab| {
            let current_tab = current_tab.get(handle.cx);
            handle.checked(current_tab == tab_index);
        });
    })
    .class("tab_bar");
}
