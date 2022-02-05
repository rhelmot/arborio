pub mod editor_tab;

use vizia::*;
use crate::app_state::AppTab;
use crate::app_state::{AppState};
use crate::AppEvent;

pub fn build_tabs(cx: &mut Context) {
    Binding::new(cx, AppState::current_tab, move |cx, current_tab| {
        let current_tab = *current_tab.get(cx);
        Binding::new_fallible(cx, AppState::tabs.index(current_tab), move |cx, current_tab| {
            match current_tab.get(cx) {
                AppTab::CelesteOverview => {}
                AppTab::ProjectOverview(_) => {}
                AppTab::Map(maptab) => {
                    let id = maptab.id.clone(); // ew
                    editor_tab::build_editor(cx, &id);
                },
            }
        }, |cx| {})
    });
}

pub fn build_tab_bar(cx: &mut Context) {
    List::new(cx, AppState::tabs, move |cx, tab| {
        Binding::new(cx, AppState::current_tab, move |cx, current_tab| {
            let current_tab = *current_tab.get(cx);
            HStack::new(cx, move |cx| {
                Label::new(cx, &tab.get(cx).to_string());
                Label::new(cx, "x")
                    .class("close_btn")
                    .on_press(move |cx| {
                        cx.emit(AppEvent::CloseTab { idx: tab.index() });
                    });
            })
                .class("tab")
                .checked(tab.index() == current_tab)
                .on_press(move |cx| {
                    cx.emit(AppEvent::SelectTab { idx: tab.index() })
                });
        });
    })
        .layout_type(LayoutType::Row)
        .height(Units::Auto);
}
