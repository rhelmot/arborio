use enum_iterator::IntoEnumIterator;
use vizia::prelude::*;

use crate::app_state::{AppState, Layer};
use crate::celeste_mod::aggregate::ModuleAggregate;
use crate::lenses::{AnotherLens, CurrentMapLens, CurrentPaletteLens};
use crate::tools::ToolSpec;
use crate::widgets::editor::EditorWidget;
use crate::widgets::entity_tweaker::EntityTweakerWidget;
use crate::widgets::list_palette::PaletteWidget;
use crate::widgets::room_tweaker::RoomTweakerWidget;
use crate::widgets::style_tweaker::{StyleListWidget, StyleTweakerWidget};
use crate::widgets::tile_palette::TilePaletteWidget;
use crate::AppEvent;

pub fn build_editor(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            build_tool_picker(cx);
        })
        .id("left_bar");

        EditorWidget::new(cx)
            .width(Stretch(1.0))
            .height(Stretch(1.0));

        VStack::new(cx, |cx| {
            build_layer_picker(cx);
            build_palette_widgets(cx);
            build_tweaker_widgets(cx);
        })
        .id("right_bar");
    })
    .height(Stretch(1.0));
}

pub fn build_tool_picker(cx: &mut Context) {
    VStack::new(cx, move |cx| {
        Binding::new(cx, CurrentMapLens {}, |cx, map| {
            let showme = map.get_fallible(cx).is_some();
            if showme {
                Binding::new(cx, AppState::current_toolspec, |cx, tool_field| {
                    for toolspec in ToolSpec::into_enum_iter() {
                        let selected = tool_field.map(move |sel| sel == &toolspec);
                        let selected2 = selected.clone();
                        HStack::new(cx, move |cx| {
                            RadioButton::new(cx, selected2.clone());
                            Label::new(cx, toolspec.name());
                        })
                        .on_press(move |cx| cx.emit(AppEvent::SelectTool { spec: toolspec }))
                        .checked(selected)
                        .class("list_highlight");
                    }
                })
            }
        });
    })
    .id("tool_picker");
}

pub fn build_layer_picker(cx: &mut Context) {
    VStack::new(cx, move |cx| {
        for layer in Layer::into_enum_iter() {
            let selected = AppState::current_layer.map(move |sel| sel == &layer);
            let selected2 = selected.clone();
            HStack::new(cx, move |cx| {
                RadioButton::new(cx, selected2.clone());
                Label::new(cx, layer.name());
            })
            .on_press(move |cx| cx.emit(AppEvent::SelectLayer { layer }))
            .checked(selected)
            .class("list_highlight")
            .bind(AppState::current_toolspec, move |handle, toolspec| {
                let toolspec = toolspec.get(handle.cx);
                handle.display(layer != Layer::All || toolspec == ToolSpec::Selection);
            });
        }
    })
    .id("layer_picker")
    .bind(AppState::current_toolspec, move |handle, toolspec| {
        let toolspec = toolspec.get(handle.cx);
        handle.display(toolspec != ToolSpec::Style);
    });
}

pub fn build_palette_widgets(cx: &mut Context) {
    println!("flag 2");
    let pair = AnotherLens::new(AppState::current_toolspec, AppState::current_layer);
    PaletteWidget::new(
        cx,
        CurrentPaletteLens {}.then(ModuleAggregate::fg_tiles_palette),
        AppState::current_fg_tile,
        |cx, tile| {
            cx.emit(AppEvent::SelectPaletteTile { fg: true, tile });
        },
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(handle.cx);
        handle.display(layer == Layer::FgTiles && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        CurrentPaletteLens {}.then(ModuleAggregate::bg_tiles_palette),
        AppState::current_bg_tile,
        |cx, tile| cx.emit(AppEvent::SelectPaletteTile { fg: false, tile }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(handle.cx);
        handle.display(layer == Layer::BgTiles && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        CurrentPaletteLens {}.then(ModuleAggregate::entities_palette),
        AppState::current_entity,
        |cx, entity| cx.emit(AppEvent::SelectPaletteEntity { entity }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(handle.cx);
        handle.display(layer == Layer::Entities && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        CurrentPaletteLens {}.then(ModuleAggregate::triggers_palette),
        AppState::current_trigger,
        |cx, trigger| cx.emit(AppEvent::SelectPaletteTrigger { trigger }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(handle.cx);
        handle.display(layer == Layer::Triggers && toolspec == ToolSpec::Pencil);
    });

    PaletteWidget::new(
        cx,
        CurrentPaletteLens {}.then(ModuleAggregate::decals_palette),
        AppState::current_decal,
        |cx, decal| cx.emit(AppEvent::SelectPaletteDecal { decal }),
    )
    .bind(pair, |handle, pair| {
        let (toolspec, layer) = pair.get(handle.cx);
        handle.display(
            (layer == Layer::FgDecals || layer == Layer::BgDecals) && toolspec == ToolSpec::Pencil,
        );
    });

    Binding::new(cx, AppState::current_objtile, move |cx, objtile| {
        TilePaletteWidget::new(cx, objtile.get(cx), |cx, tile| {
            cx.emit(AppEvent::SelectPaletteObjectTile { tile })
        })
        .bind(pair, |handle, pair| {
            let (toolspec, layer) = pair.get(handle.cx);
            handle.display(layer == Layer::ObjectTiles && toolspec == ToolSpec::Pencil);
        })
        .min_height(Units::Pixels(100.0))
        .min_width(Units::Pixels(100.0))
        .height(Units::Stretch(1.0))
        .width(Units::Stretch(1.0));
    });
}

pub fn build_tweaker_widgets(cx: &mut Context) {
    Binding::new(cx, AppState::current_toolspec, |cx, tool_idx| {
        let tool_idx = tool_idx.get(cx);
        EntityTweakerWidget::new(cx).display(tool_idx == ToolSpec::Selection);
        RoomTweakerWidget::new(cx).display(tool_idx == ToolSpec::Room);
        StyleListWidget::new(cx).display(tool_idx == ToolSpec::Style);
        StyleTweakerWidget::new(cx).display(tool_idx == ToolSpec::Style);
    });
}
