#![allow(unused)]
#![feature(type_alias_impl_trait)]

mod app_state;
mod assets;
mod atlas_img;
mod auto_saver;
mod autotiler;
mod config;
mod map_struct;
mod tools;
mod units;
mod widgets;

use dialog::{DialogBox, FileSelectionMode};
use enum_iterator::IntoEnumIterator;
use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::io::Read;
use vizia::*;
use widgets::editor_widget;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::TOOLS;
use widgets::palette_widget::PaletteWidget;
use widgets::tweaker_widget::EntityTweakerWidget;
use crate::assets::next_uuid;
use crate::config::aggregate::ModuleAggregate;
use crate::config::walker::ConfigSource;
use crate::map_struct::MapID;
use crate::widgets::tabs::{build_tabs, build_tab_bar};

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Application::new(WindowDescription::new().with_title("Arborio"), |cx| {
        app_state::AppState::new().build(cx);
        //cx.add_theme(include_str!("style.css"));
        cx.add_stylesheet("src/style.css");

        VStack::new(cx, move |cx| {
            HStack::new(cx, move |cx| {
                Button::new(cx, move |cx| {
                    if let Some(map) = load_workflow() {
                        cx.emit(AppEvent::Load { map: RefCell::new(Some(map)) })
                    }
                }, move |cx| {
                    Label::new(cx, "Load Map")
                });
            });
            build_tab_bar(cx);
            build_tabs(cx);
        });
    });

    app.run();
    Ok(())
}

fn load_workflow() -> Option<map_struct::CelesteMap> {
    let path = match dialog::FileSelection::new("Select a map")
        .title("Select a map")
        .mode(FileSelectionMode::Open)
        .path(&assets::CONFIG.lock().unwrap().celeste_root)
        .show()
    {
        Ok(Some(path)) => path,
        _ => return None,
    };
    let file = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) => {
            dialog::Message::new(format!("Could not read file: {}", e)).show();
            return None;
        }
    };
    let (_, binfile) = match celeste::binel::parser::take_file(file.as_slice()) {
        Ok(binel) => binel,
        _ => {
            dialog::Message::new("Not a Celeste map").show();
            return None;
        }
    };
    let map = match map_struct::from_binfile(MapID { module: "Arborio".to_string(), sid: format!("weh/{}", next_uuid()) }, binfile) {
        Ok(map) => map,
        Err(e) => {
            dialog::Message::new(format!("Data validation error: {}", e));
            return None;
        }
    };

    Some(map)
}
