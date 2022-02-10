pub mod editor_tab;
pub mod installation_tab;
pub mod project_tab;

use crate::app_state::AppState;
use crate::app_state::AppTab;
use crate::AppEvent;
use vizia::*;

pub fn build_tabs(cx: &mut Context) {
    Binding::new_fallible(
        cx,
        AppState::tabs.index_with_lens(AppState::current_tab),
        move |cx, current_tab| {
            VStack::new(cx, move |cx| {
                match current_tab.get(cx) {
                    AppTab::CelesteOverview => {
                        installation_tab::build_installation_tab(cx);
                    }
                    AppTab::ProjectOverview(project) => {
                        let project = project.clone();
                        project_tab::build_project_tab(cx, &project)
                    }
                    AppTab::Map(maptab) => {
                        let id = maptab.id.clone(); // ew
                        editor_tab::build_editor(cx, &id);
                    }
                }
            });
        },
        move |cx| {},
    );
}

pub fn build_tab_bar(cx: &mut Context) {
    Binding::new(cx, AppState::current_tab, move |cx, current_tab| {
        List::new(cx, AppState::tabs, move |cx, tab| {
            let current_tab = *current_tab.get(cx);
            HStack::new(cx, move |cx| {
                Label::new(cx, &tab.get(cx).to_string());
                Label::new(cx, "x").class("close_btn").on_press(move |cx| {
                    cx.emit(AppEvent::CloseTab { idx: tab.index() });
                });
            })
            .class("tab")
            .checked(tab.index() == current_tab)
            .on_press(move |cx| {
                cx.emit(AppEvent::SelectTab { idx: tab.index() });
            });
        })
        .layout_type(LayoutType::Row)
        .height(Units::Auto);
    });
}
