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
use vizia::*;
use widgets::editor_widget;

use crate::app_state::{AppEvent, AppState, Layer};
use crate::tools::TOOLS;
use widgets::palette_widget::PaletteWidget;
use widgets::tweaker_widget::EntityTweakerWidget;
use crate::config::aggregate::ModuleAggregate;

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = Application::new(WindowDescription::new().with_title("Arborio"), |cx| {
        app_state::AppState::new().build(cx);
        //cx.add_theme(include_str!("style.css"));
        cx.add_stylesheet("src/style.css");

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Button::new(
                    cx,
                    |cx| {
                        if let Some(map) = load_workflow() {
                            cx.emit(AppEvent::Load {
                                map: RefCell::new(Some(map)),
                            });
                        }
                    },
                    |cx| Label::new(cx, "Load Map"),
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
                            Button::new(
                                cx,
                                move |cx| cx.emit(AppEvent::SelectTool { idx }),
                                move |cx| {
                                    RadioButton::new(cx, idx == selected);
                                    Label::new(cx, TOOLS.lock().unwrap()[idx].name())
                                },
                            )
                            .checked(idx == selected)
                            .class("btn_item")
                            .layout_type(LayoutType::Row);
                        }
                    });
                })
                .width(Stretch(0.0));

                editor_widget::EditorWidget::new(cx)
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                VStack::new(cx, |cx| {
                    // tool settings
                    Binding::new(cx, AppState::current_tool, move |cx, tool_idx| {
                        let tool_idx = *tool_idx.get(cx);
                        Picker::new(cx, AppState::current_layer, move |cx, layer_field| {
                            let selected = *layer_field.get(cx);
                            for layer in Layer::into_enum_iter() {
                                Button::new(
                                    cx,
                                    move |cx| {
                                        cx.emit(AppEvent::SelectLayer { layer });
                                    },
                                    move |cx| {
                                        RadioButton::new(cx, layer == selected);
                                        Label::new(cx, layer.name())
                                    },
                                )
                                .checked(layer == selected)
                                .class("btn_item")
                                .layout_type(LayoutType::Row)
                                .display(
                                    if layer == Layer::All && tool_idx != 1 {
                                        // TODO un-hardcode selection tool
                                        Display::None
                                    } else {
                                        Display::Flex
                                    },
                                );
                            }
                        });
                        Binding::new(cx, AppState::current_layer, move |cx, layer_field| {
                            let layer = *layer_field.get(cx);
                            PaletteWidget::new(
                                cx,
                                AppState::palette.then(ModuleAggregate::fg_tiles_palette),
                                AppState::current_fg_tile,
                                |cx, tile| {
                                    cx.emit(AppEvent::SelectPaletteTile { fg: true, tile });
                                },
                            )
                            .display(
                                if layer == Layer::FgTiles && tool_idx == 2 {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                            );
                            PaletteWidget::new(
                                cx,
                                AppState::palette.then(ModuleAggregate::bg_tiles_palette),
                                AppState::current_bg_tile,
                                |cx, tile| cx.emit(AppEvent::SelectPaletteTile { fg: false, tile }),
                            )
                            .display(
                                if layer == Layer::BgTiles && tool_idx == 2 {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                            );
                            PaletteWidget::new(
                                cx,
                                AppState::palette.then(ModuleAggregate::entities_palette),
                                AppState::current_entity,
                                |cx, entity| cx.emit(AppEvent::SelectPaletteEntity { entity }),
                            )
                            .display(
                                if layer == Layer::Entities && tool_idx == 2 {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                            );
                            PaletteWidget::new(
                                cx,
                                AppState::palette.then(ModuleAggregate::triggers_palette),
                                AppState::current_trigger,
                                |cx, trigger| cx.emit(AppEvent::SelectPaletteTrigger { trigger }),
                            )
                            .display(
                                if layer == Layer::Triggers && tool_idx == 2 {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                            );
                            PaletteWidget::new(
                                cx,
                                AppState::palette.then(ModuleAggregate::decals_palette),
                                AppState::current_decal,
                                |cx, decal| cx.emit(AppEvent::SelectPaletteDecal { decal }),
                            )
                            .display(
                                if (layer == Layer::FgDecals || layer == Layer::BgDecals)
                                    && tool_idx == 2
                                {
                                    Display::Flex
                                } else {
                                    Display::None
                                },
                            );
                        });
                        EntityTweakerWidget::new(cx).display(if tool_idx == 1 {
                            Display::Flex
                        } else {
                            Display::None
                        });
                    });
                })
                .width(Pixels(100.0));
            });
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
    let map = match map_struct::from_binfile(binfile) {
        Ok(map) => map,
        Err(e) => {
            dialog::Message::new(format!("Data validation error: {}", e));
            return None;
        }
    };

    Some(map)
}
