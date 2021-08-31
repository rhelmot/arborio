#![allow(unused)]

mod assets;
mod atlas_img;
mod auto_saver;
mod autotiler;
mod editor_widget;
mod entity_config;
mod entity_expression;
mod from_binel;
mod image_view;
mod map_struct;

use fltk::{prelude::*, *};
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
    assets::load();

    let app = app::App::default();
    app::set_visual(enums::Mode::Rgb).unwrap();

    let mut win = build_main_window();
    win.show();

    app.run().unwrap();
    Ok(())
}

fn build_main_window() -> window::DoubleWindow {
    let width = 1000;
    let height = 600;
    let button_size = 25;

    let mut win = window::DoubleWindow::default()
        .with_size(width, height)
        .with_label("Arborio")
        .center_screen();

    let mut vlayout = group::Pack::new(0, 0, width, height, "");
    let mut toolbar = group::Pack::new(0, 0, width, button_size, "");

    let mut btn1 = button::Button::new(0, 0, button_size, button_size, "");
    btn1.set_image(Some(
        image::PngImage::from_data(include_bytes!("../img/strawberry.png")).unwrap(),
    ));
    let mut btn2 = button::Button::new(0, 0, button_size, button_size, "");
    btn2.set_image(Some(
        image::PngImage::from_data(include_bytes!("../img/grass.png")).unwrap(),
    ));

    toolbar.end();
    toolbar.set_type(group::PackType::Horizontal);
    toolbar.make_resizable(false);

    let mut editor = editor_widget::EditorWidget::new(0, 0, width, height - button_size);
    vlayout.resizable(&editor.widget);

    vlayout.end();
    vlayout.set_type(group::PackType::Vertical);

    win.end();

    btn1.set_callback(move |_| {
        // TODO store last used dir in config
        let path = match dialog::file_chooser(
            "Choose a celeste map",
            "*.bin",
            assets::CONFIG
                .lock()
                .unwrap()
                .celeste_root
                .to_str()
                .unwrap(),
            false,
        ) {
            Some(v) => v,
            None => {
                return;
            }
        };
        let buf = match fs::read(path) {
            Ok(v) => v,
            Err(e) => {
                dialog::alert(
                    center().0,
                    center().1,
                    format!("Could not load file: {}", e).as_str(),
                );
                return;
            }
        };
        let parsed = match celeste::binel::parser::take_file(buf.as_slice()) {
            Ok((_, v)) => v,
            Err(e) => {
                dialog::alert(
                    center().0,
                    center().1,
                    format!("File is not a celeste map: {}", e).as_str(),
                );
                return;
            }
        };
        println!("{:#?}", &parsed);
        let map = match map_struct::from_binfile(parsed) {
            Ok(v) => v,
            Err(e) => {
                dialog::alert(
                    center().0,
                    center().1,
                    format!("Data validation error: {}", e).as_str(),
                );
                return;
            }
        };

        editor.set_map(map);
        editor.reset_view();
    });

    win.make_resizable(true);
    win
}

#[inline(always)]
pub fn center() -> (i32, i32) {
    (
        (app::screen_size().0 / 2.0) as i32,
        (app::screen_size().1 / 2.0) as i32,
    )
}
