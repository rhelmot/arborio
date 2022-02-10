#![allow(unused)]
#![feature(type_alias_impl_trait)]

mod app_state;
mod assets;
mod atlas_img;
mod auto_saver;
mod autotiler;
mod celeste_mod;
mod map_struct;
mod tools;
mod units;
mod widgets;
mod lenses;

use dialog::{DialogBox, FileSelectionMode};
use enum_iterator::IntoEnumIterator;
use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use vizia::*;
use widgets::editor_widget;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::TOOLS;
use widgets::palette_widget::PaletteWidget;
use widgets::tweaker_widget::EntityTweakerWidget;
use crate::assets::next_uuid;
use crate::celeste_mod::aggregate::ModuleAggregate;
use crate::celeste_mod::walker::ConfigSource;
use crate::map_struct::MapID;
use crate::widgets::tabs::{build_tabs, build_tab_bar};

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Application::new(WindowDescription::new().with_title("Arborio"), |cx| {
        app_state::AppState::new().build(cx);
        if let Some(path) = &cx.data::<AppState>().unwrap().config.celeste_root {
            let path = path.clone();
            cx.emit(AppEvent::SetConfigPath { path });
        }
        //cx.add_theme(include_str!("style.css"));
        cx.add_stylesheet("src/style.css");

        VStack::new(cx, move |cx| {
            HStack::new(cx, move |cx| {
                // menu bar
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
                            .class("progress_bar_bg");
                        Label::new(cx, &status)
                            .width(Units::Percentage(progress as f32))
                            .class("progress_bar");
                    });
                }
            })
        });
    });

    app.run();
    Ok(())
}
