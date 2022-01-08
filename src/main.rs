#![allow(unused)]

mod editor_widget;
mod map_struct;
mod atlas_img;
mod autotiler;
mod assets;
mod auto_saver;
mod entity_config;
mod entity_expression;
mod app_state;
mod tools;
mod units;

use std::fs;
use std::cell::RefCell;
use std::error::Error;
use vizia::*;
use dialog::{DialogBox, FileSelectionMode};
use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::TOOLS;

fn main() -> Result<(), Box<dyn Error>> {
    assets::load();

    let mut app = Application::new(
        WindowDescription::new()
            .with_title("Arborio"),
        |cx| {
            app_state::AppState::new().build(cx);
            cx.add_theme(include_str!("style.css"));

            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Button::new(cx, |cx| {
                            if let Some(map) = load_workflow() {
                                cx.emit(AppEvent::Load { map: RefCell::new(Some(map)) });
                            }
                        },
                        |cx| Label::new(cx, "Load Map")
                    );
                })
                    .height(Pixels(30.0));
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        // tool picker
                        Picker::new(cx, AppState::current_tool, |cx, tool_field| {
                            let selected = *tool_field.get(cx);
                            let count = TOOLS.lock().unwrap().len();
                            for idx in 0..count {
                                Button::new(cx, move |cx| {
                                    cx.emit(AppEvent::SelectTool { idx })
                                }, move |cx| {
                                    RadioButton::new(cx, idx == selected);
                                    Label::new(cx, TOOLS.lock().unwrap()[idx].name())
                                })
                                    .checked(idx == selected)
                                    .class("btn_item")
                                    .layout_type(LayoutType::Row);
                            }
                        });
                    })  .width(Stretch(0.0));

                    editor_widget::EditorWidget::new(cx)
                        .width(Stretch(1.0))
                        .height(Stretch(1.0));
                    VStack::new(cx, |cx| {
                        // tool settings
                        Picker::new(cx, AppState::current_layer, |cx, layer_field| {
                            let selected = *layer_field.get(cx);
                            for layer in Layer::all_layers() {
                                Button::new(cx, move |cx| {
                                    cx.emit(AppEvent::SelectLayer { layer });
                                }, move |cx| {
                                    RadioButton::new(cx, layer == selected);
                                    Label::new(cx, layer.name())
                                })
                                    .checked(layer == selected)
                                    .class("btn_item")
                                    .layout_type(LayoutType::Row);
                            }
                        });
                    })  .width(Pixels(100.0));
                });
            });
        });

    app.run();
    Ok(())
}

fn load_workflow() -> Option<map_struct::CelesteMap>{
    let path = match dialog::FileSelection::new("Select a map")
        .title("Select a map")
        .mode(FileSelectionMode::Open)
        .path(assets::CONFIG.lock().unwrap().celeste_root.to_path_buf())
        .show() {
        Ok(Some(path)) => path,
        _ => return None
    };
    let file = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) => {
            dialog::Message::new(format!("Could not read file: {}", e)).show();
            return None
        }
    };
    let (_, binfile) = match celeste::binel::parser::take_file(file.as_slice()) {
        Ok(binel) => binel,
        _ => {
            dialog::Message::new("Not a Celeste map").show();
            return None
        }
    };
    let map = match map_struct::from_binfile(binfile) {
        Ok(map) => map,
        Err(e) => {
            dialog::Message::new(format!("Data validation error: {}", e));
            return None
        }
    };

    Some(map)
}
