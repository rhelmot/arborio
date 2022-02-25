use enum_iterator::IntoEnumIterator;
use vizia::*;

use crate::app_state::AppState;
use crate::lenses::{CurrentMapLens, CurrentPaletteLens};
use crate::tools::ToolSpec;
use crate::widgets::editor::EditorWidget;
use crate::widgets::room_tweaker::RoomTweakerWidget;
use crate::widgets::tile_palette::TilePaletteWidget;
use crate::{AppEvent, EntityTweakerWidget, Layer, ModuleAggregate, PaletteWidget};

pub fn build_editor(cx: &mut Context) {
    HStack::new(cx, |cx| {
        VStack::new(cx, |cx| {
            build_tool_picker(cx);
        })
        .width(Stretch(0.0));

        EditorWidget::new(cx)
            .width(Stretch(1.0))
            .height(Stretch(1.0));

        VStack::new(cx, |cx| {
            build_layer_picker(cx);
            build_palette_widgets(cx);
            build_tweaker_widgets(cx);
        })
        .width(Pixels(100.0));
    })
    .height(Stretch(1.0));
}

pub fn build_tool_picker(cx: &mut Context) {
    Binding::new(cx, CurrentMapLens {}, |cx, map| {
        let showme = map.get_fallible(cx).is_some();
        Picker::new(cx, AppState::current_toolspec, |cx, tool_field| {
            let selected = *tool_field.get(cx);
            for toolspec in ToolSpec::into_enum_iter() {
                Button::new(
                    cx,
                    move |cx| cx.emit(AppEvent::SelectTool { spec: toolspec }),
                    move |cx| {
                        RadioButton::new(cx, toolspec == selected);
                        Label::new(cx, toolspec.name())
                    },
                )
                .checked(toolspec == selected)
                .class("btn_item")
                .layout_type(LayoutType::Row);
            }
        })
        .display(showme);
    });
}

pub fn build_layer_picker(cx: &mut Context) {
    Binding::new(cx, AppState::current_toolspec, move |cx, tool_idx| {
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
                    if layer == Layer::All && tool_idx != ToolSpec::Selection {
                        Display::None
                    } else {
                        Display::Flex
                    },
                );
            }
        });
    });
}

pub fn build_palette_widgets(cx: &mut Context) {
    Binding::new(cx, AppState::current_toolspec, move |cx, tool_idx| {
        let tool_idx = *tool_idx.get(cx);
        Binding::new(cx, AppState::current_layer, move |cx, layer_field| {
            let layer = *layer_field.get(cx);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens {}.then(ModuleAggregate::fg_tiles_palette),
                AppState::current_fg_tile,
                |cx, tile| {
                    cx.emit(AppEvent::SelectPaletteTile { fg: true, tile });
                },
            )
            .display(layer == Layer::FgTiles && tool_idx == ToolSpec::Pencil);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens {}.then(ModuleAggregate::bg_tiles_palette),
                AppState::current_bg_tile,
                |cx, tile| cx.emit(AppEvent::SelectPaletteTile { fg: false, tile }),
            )
            .display(layer == Layer::BgTiles && tool_idx == ToolSpec::Pencil);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens {}.then(ModuleAggregate::entities_palette),
                AppState::current_entity,
                |cx, entity| cx.emit(AppEvent::SelectPaletteEntity { entity }),
            )
            .display(layer == Layer::Entities && tool_idx == ToolSpec::Pencil);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens {}.then(ModuleAggregate::triggers_palette),
                AppState::current_trigger,
                |cx, trigger| cx.emit(AppEvent::SelectPaletteTrigger { trigger }),
            )
            .display(layer == Layer::Triggers && tool_idx == ToolSpec::Pencil);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens {}.then(ModuleAggregate::decals_palette),
                AppState::current_decal,
                |cx, decal| cx.emit(AppEvent::SelectPaletteDecal { decal }),
            )
            .display(
                (layer == Layer::FgDecals || layer == Layer::BgDecals)
                    && tool_idx == ToolSpec::Pencil,
            );
            Binding::new(cx, AppState::current_objtile, move |cx, objtile| {
                TilePaletteWidget::new(cx, *objtile.get(cx), |cx, tile| {
                    cx.emit(AppEvent::SelectPaletteObjectTile { tile })
                })
                .display(layer == Layer::ObjectTiles && tool_idx == ToolSpec::Pencil)
                .min_height(Units::Pixels(100.0))
                .min_width(Units::Pixels(100.0))
                .height(Units::Stretch(1.0))
                .width(Units::Stretch(1.0));
            });
        });
    });
}

pub fn build_tweaker_widgets(cx: &mut Context) {
    Binding::new(cx, AppState::current_toolspec, |cx, tool_idx| {
        let tool_idx = *tool_idx.get(cx);
        EntityTweakerWidget::new(cx).display(tool_idx == ToolSpec::Selection);
        RoomTweakerWidget::new(cx).display(tool_idx == ToolSpec::Room);
    });
}
