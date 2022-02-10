use vizia::*;
use enum_iterator::IntoEnumIterator;

use crate::lenses::{CurrentMapLens, CurrentPaletteLens};
use crate::app_state::{AppState};
use crate::map_struct::{CelesteMap, MapID};
use crate::editor_widget::EditorWidget;
use crate::{AppEvent, EntityTweakerWidget, Layer, ModuleAggregate, PaletteWidget, TOOLS};
use crate::units::*;


pub fn build_editor(cx: &mut Context, id: &MapID) {
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
            build_tweaker_widget(cx);
        })
            .width(Pixels(100.0));
    })
        .height(Stretch(1.0));
}

pub fn build_tool_picker(cx: &mut Context) {
    Binding::new(cx, CurrentMapLens{}, |cx, map| {
        let showme = map.get_fallible(cx).is_some();
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
        })
            .display(showme);
    });
}

pub fn build_layer_picker(cx: &mut Context) {
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
    });
}

pub fn build_palette_widgets(cx: &mut Context) {
    Binding::new(cx, AppState::current_tool, move |cx, tool_idx| {
        let tool_idx = *tool_idx.get(cx);
        Binding::new(cx, AppState::current_layer, move |cx, layer_field| {
            let layer = *layer_field.get(cx);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens{}.then(ModuleAggregate::fg_tiles_palette),
                AppState::current_fg_tile,
                |cx, tile| {
                    cx.emit(AppEvent::SelectPaletteTile { fg: true, tile });
                },
            )
                .display(layer == Layer::FgTiles && tool_idx == 2);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens{}.then(ModuleAggregate::bg_tiles_palette),
                AppState::current_bg_tile,
                |cx, tile| cx.emit(AppEvent::SelectPaletteTile { fg: false, tile }),
            )
                .display(layer == Layer::BgTiles && tool_idx == 2);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens{}.then(ModuleAggregate::entities_palette),
                AppState::current_entity,
                |cx, entity| cx.emit(AppEvent::SelectPaletteEntity { entity }),
            )
                .display(layer == Layer::Entities && tool_idx == 2);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens{}.then(ModuleAggregate::triggers_palette),
                AppState::current_trigger,
                |cx, trigger| cx.emit(AppEvent::SelectPaletteTrigger { trigger }),
            )
                .display(layer == Layer::Triggers && tool_idx == 2);
            PaletteWidget::new(
                cx,
                CurrentPaletteLens{}.then(ModuleAggregate::decals_palette),
                AppState::current_decal,
                |cx, decal| cx.emit(AppEvent::SelectPaletteDecal { decal }),
            )
                .display((layer == Layer::FgDecals || layer == Layer::BgDecals) && tool_idx == 2);
        });
    });
}

pub fn build_tweaker_widget(cx: &mut Context) {
    Binding::new(cx, AppState::current_tool, |cx, tool_idx| {
        let tool_idx = *tool_idx.get(cx);
        EntityTweakerWidget::new(cx).display(tool_idx == 1);
    });
}
