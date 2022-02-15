pub mod editor_tab;
pub mod installation_tab;
pub mod project_tab;

use crate::app_state::AppState;
use crate::app_state::AppTab;
use crate::AppEvent;
use vizia::*;

pub fn build_tabs(cx: &mut Context) {
    Binding::new(cx, AppState::current_tab, move |cx, current_tab_idx| {
        Binding::new(
            cx,
            AppState::tabs.index(*current_tab_idx.get(cx)),
            move |cx, current_tab| {
                if let Some(current_tab) = current_tab.get_fallible(cx) {
                    VStack::new(cx, move |cx| match *current_tab {
                        AppTab::CelesteOverview => {
                            installation_tab::build_installation_tab(cx);
                        }
                        AppTab::ProjectOverview(project) => {
                            project_tab::build_project_tab(cx, project)
                        }
                        AppTab::Map(_) => {
                            editor_tab::build_editor(cx);
                        }
                    });
                }
            },
        );
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
            let current_tab = *current_tab.get(handle.cx);
            handle.checked(current_tab == tab_index);
        });
    })
    .layout_type(LayoutType::Row)
    .height(Units::Auto);
}
