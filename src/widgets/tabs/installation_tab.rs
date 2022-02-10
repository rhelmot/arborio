use dialog::DialogBox;
use itertools::Itertools;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use vizia::*;

use crate::app_state::{AppConfig, AppState};
use crate::lenses::{AutoSaverLens, UnwrapLens};
use crate::AppEvent;

pub fn build_installation_tab(cx: &mut Context) {
    Binding::new_fallible(
        cx,
        AppState::config
            .then(AutoSaverLens::new())
            .then(AppConfig::celeste_root)
            .then(UnwrapLens::new()),
        |cx, root| {
            Label::new(
                cx,
                &format!("Current celeste install is {:?}", root.get(cx)),
            );
            Binding::new(cx, AppState::modules_version, move |cx, _| {
                let modules = &cx.data::<AppState>().unwrap().modules;
                // TODO: sort dirs first
                let mut modules_list = modules
                    .iter()
                    .map(|(name, module)| {
                        (
                            name.clone(),
                            module.maps.len(),
                            module.everest_metadata.name.clone(),
                        )
                    })
                    .collect::<Vec<_>>();
                modules_list.sort_by_key(|(_, _, name)| name.clone());

                for (sid, num_maps, name) in modules_list.into_iter() {
                    VStack::new(cx, move |cx| {
                        Label::new(cx, &name).class("module_title");
                        Label::new(
                            cx,
                            &format!("{} map{}", num_maps, if num_maps == 1 { "" } else { "s" }),
                        );
                    })
                    .class("module_overview_card")
                    .on_press(move |cx| {
                        cx.emit(AppEvent::OpenModuleOverview {
                            module: sid.clone(),
                        })
                    });
                }
            });
        },
        |cx| {
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
        },
    )
}
