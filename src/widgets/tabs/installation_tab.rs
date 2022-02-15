use dialog::DialogBox;
use std::path::Path;
use vizia::*;

use crate::app_state::{AppConfig, AppState};
use crate::assets::Interned;
use crate::celeste_mod::module::CelesteModuleKind;
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
                Label::new(cx, &format!("Current celeste install is {:?}", *root));
                Binding::new(cx, AppState::modules_version, move |cx, _| {
                    let modules = &cx.data::<AppState>().unwrap().modules;
                    let mut modules_list = modules
                        .iter()
                        .map(|(name, module)| {
                            (
                                *name,
                                module.maps.len(),
                                module.everest_metadata.name,
                                module.module_kind(),
                            )
                        })
                        .collect::<Vec<_>>();
                    modules_list.sort_by_key(|(_, _, name, _)| *name);

                    let mut idx = 0usize;
                    let mut first = true;
                    while idx < modules_list.len() {
                        if matches!(modules_list[idx].3, CelesteModuleKind::Directory) {
                            let (sid, num_maps, name, _) = modules_list.remove(idx);
                            if first {
                                first = false;
                                Label::new(cx, "My Mods");
                            }
                            build_project_overview_card(cx, sid, name, num_maps);
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
                                Label::new(cx, "Builtin Modules");
                            }
                            build_project_overview_card(cx, sid, name, num_maps);
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
                                Label::new(cx, "Downloaded Mods");
                            }
                            build_project_overview_card(cx, sid, name, num_maps);
                        } else {
                            idx += 1;
                        }
                    }

                    assert_eq!(modules_list.len(), 0);
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

fn build_project_overview_card(cx: &mut Context, sid: Interned, name: Interned, num_maps: usize) {
    VStack::new(cx, move |cx| {
        Label::new(cx, *name).class("module_title");
        Label::new(
            cx,
            &format!("{} map{}", num_maps, if num_maps == 1 { "" } else { "s" }),
        );
    })
    .class("module_overview_card")
    .on_press(move |cx| cx.emit(AppEvent::OpenModuleOverview { module: sid }));
}
