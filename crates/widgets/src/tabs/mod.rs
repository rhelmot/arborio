pub mod config_editor;
pub mod editor;
pub mod installation;
pub mod logs;
pub mod map_meta;
pub mod project;

use arborio_state::data::app::AppEvent;
use arborio_state::data::app::AppState;
use arborio_state::data::tabs::AppTab;
use arborio_state::lenses::{TabTextLens, VecIndexWithLens};
use arborio_utils::vizia::prelude::*;

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
                AppTab::MapMeta(id) => {
                    map_meta::build_map_meta_tab(cx, id);
                }
            })
            .class("tab_container");
        }
    });
}

pub fn build_tab_bar(cx: &mut Context) {
    List::new(cx, AppState::tabs, move |cx, tab_index, _tab| {
        HStack::new(cx, move |cx| {
            Label::new(cx, TabTextLens(tab_index));
            Label::new(cx, "\u{e5cd}")
                .class("icon")
                .class("close_btn")
                .on_press(move |cx| {
                    cx.emit(AppEvent::CloseTab { idx: tab_index });
                });
            Element::new(cx).class("tab_highlight");
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
