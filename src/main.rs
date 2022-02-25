#![allow(clippy::too_many_arguments)]

mod app_state;
mod assets;
mod atlas_img;
mod auto_saver;
mod autotiler;
mod celeste_mod;
#[macro_use]
mod from_binel;
mod lenses;
mod map_struct;
mod tools;
mod units;
mod widgets;

use std::error::Error;
use vizia::*;

use crate::app_state::{AppEvent, AppState, AppTab, Layer};
use crate::celeste_mod::aggregate::ModuleAggregate;
use crate::lenses::VecIndexWithLens;
use crate::map_struct::MapID;
use crate::widgets::tabs::{build_tab_bar, build_tabs};
use widgets::entity_tweaker::EntityTweakerWidget;
use widgets::list_palette::PaletteWidget;

fn main() -> Result<(), Box<dyn Error>> {
    let icon_img = image::load_from_memory(include_bytes!("../img/icon.png")).unwrap();
    let (width, height) = (icon_img.width(), icon_img.height());
    let app = Application::new(
        WindowDescription::new().with_title("Arborio").with_icon(
            icon_img.into_bytes(),
            width,
            height,
        ),
        |cx| {
            app_state::AppState::new().build(cx);
            if let Some(path) = &cx.data::<AppState>().unwrap().config.celeste_root {
                let path = path.clone();
                cx.emit(AppEvent::SetConfigPath { path });
            }
            //cx.add_theme(include_str!("style.css"));
            cx.add_stylesheet("src/style.css")
                .expect("Could not load stylesheet. Are you running me in the right directory?");

            VStack::new(cx, move |cx| {
                HStack::new(cx, move |cx| {
                    build_menu_bar(cx);
                })
                .class("menu_bar");
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
                                .class("progress_bar_bg");
                            Label::new(cx, &status)
                                .width(Units::Percentage(progress as f32))
                                .class("progress_bar");
                        })
                        .class("progress_bar_container");
                    }
                })
            })
            .class("main");
        },
    );

    app.run();
    Ok(())
}

fn build_menu_bar(cx: &mut Context) {
    let lens = VecIndexWithLens::new(AppState::tabs, AppState::current_tab);
    Button::new(
        cx,
        move |_cx| {
            println!("TODO");
        },
        move |cx| Label::new(cx, "Save"),
    )
    .bind(lens, move |handle, lens| {
        if let Some(tab) = lens.get_fallible(handle.cx) {
            handle.display(matches!(*tab, AppTab::Map(_)));
        } else {
            handle.display(false);
        }
    });
}
