use dialog::DialogBox;
use std::path::Path;
use vizia::prelude::*;
use vizia::state::UnwrapLens;

use crate::app_state::{AppConfig, AppState};
use crate::celeste_mod::module::{CelesteModuleKind, ModuleID};
use crate::lenses::AutoSaverLens;
use crate::AppEvent;

pub fn build_installation_tab(cx: &mut Context) {
    Binding::new(
        cx,
        AppState::config
            .then(AutoSaverLens::new())
            .then(AppConfig::celeste_root)
            .then(UnwrapLens::new()),
        |cx, root| {
            if let Some(root) = root.get_fallible(cx) {
                Label::new(cx, &format!("Current celeste install is {:?}", root));
                ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
                    VStack::new(cx, move |cx| {
                        Binding::new(cx, AppState::modules_version, move |cx, _| {
                            let modules = &cx.data::<AppState>().unwrap().modules;
                            let mut modules_list = modules
                                .iter()
                                .map(|(name, module)| {
                                    (
                                        *name,
                                        module.maps.len(),
                                        module.everest_metadata.name.clone(),
                                        module.module_kind(),
                                    )
                                })
                                .collect::<Vec<_>>();
                            modules_list.sort_by_key(|(_, _, name, _)| name.clone()); // TODO why clone????

                            let mut idx = 0usize;
                            Label::new(cx, "My Mods").class("module_category");
                            HStack::new(cx, move |cx| {
                                Label::new(cx, "+").class("big_plus");
                                Label::new(cx, "New Project").id("new_mod_text");
                            })
                            .class("btn_highlight")
                            .id("new_mod_button")
                            .on_press(|cx| cx.emit(AppEvent::NewMod));
                            while idx < modules_list.len() {
                                if matches!(modules_list[idx].3, CelesteModuleKind::Directory) {
                                    let (sid, num_maps, name, _) = modules_list.remove(idx);
                                    build_project_overview_card(cx, sid, &name, num_maps);
                                } else {
                                    idx += 1;
                                }
                            }

                            let mut idx = 0usize;
                            let mut first = true;
                            while idx < modules_list.len() {
                                if matches!(modules_list[idx].3, CelesteModuleKind::Builtin) {
                                    let (sid, num_maps, name, _) = modules_list.remove(idx);
                                    if first {
                                        first = false;
                                        Label::new(cx, "Builtin Modules").class("module_category");
                                    }
                                    build_project_overview_card(cx, sid, &name, num_maps);
                                } else {
                                    idx += 1;
                                }
                            }

                            let mut idx = 0usize;
                            let mut first = true;
                            while idx < modules_list.len() {
                                if matches!(modules_list[idx].3, CelesteModuleKind::Zip) {
                                    let (sid, num_maps, name, _) = modules_list.remove(idx);
                                    if first {
                                        first = false;
                                        Label::new(cx, "Downloaded Mods").class("module_category");
                                    }
                                    build_project_overview_card(cx, sid, &name, num_maps);
                                } else {
                                    idx += 1;
                                }
                            }

                            assert_eq!(modules_list.len(), 0);
                        });
                    })
                    .id("modules_container");
                });
            } else {
                Button::new(
                    cx,
                    move |cx| {
                        if let Ok(Some(celeste_path)) =
                            dialog::FileSelection::new("Celeste Installation")
                                .title("Please choose Celeste.exe")
                                .path(".")
                                .mode(dialog::FileSelectionMode::Open)
                                .show()
                        {
                            cx.emit(AppEvent::SetConfigPath {
                                path: Path::new(&celeste_path).parent().unwrap().to_path_buf(),
                            });
                        }
                    },
                    move |cx| Label::new(cx, "Select Celeste.exe"),
                );
                Label::new(cx, "Please show me where Celeste is installed.");
            }
        },
    )
}

fn build_project_overview_card(cx: &mut Context, module: ModuleID, name: &str, num_maps: usize) {
    VStack::new(cx, move |cx| {
        Label::new(cx, name).class("module_title");
        Label::new(
            cx,
            &format!("{} map{}", num_maps, if num_maps == 1 { "" } else { "s" }),
        );
    })
    .class("module_overview_card")
    .class("btn_highlight")
    .on_press(move |cx| cx.emit(AppEvent::OpenModuleOverviewTab { module }));
}
