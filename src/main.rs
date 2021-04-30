mod editor_widget;
mod map_struct;

use std::fs;
use std::error::Error;
use celeste;
use celeste::binel::serialize::BinElType;
use fltk;
use fltk::{prelude::*,*};

#[inline(always)]
pub fn center() -> (i32, i32) {
    (
        (app::screen_size().0 / 2.0) as i32,
        (app::screen_size().1 / 2.0) as i32,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let width = 1000;
    let height = 600;
    let button_size = 25;

    let app = app::App::default();
    let mut win = window::DoubleWindow::default()
        .with_size(width, height)
        .with_label("Arborio")
        .center_screen();

    let mut vlayout = group::Pack::new(0, 0, width, height, "");
    let mut toolbar = group::Pack::new(0, 0, width, button_size, "");

    let mut btn1 = button::Button::new(0, 0, button_size, button_size, "");
    btn1.set_image(Some(image::PngImage::from_data(include_bytes!("../img/strawberry.png")).unwrap()));
    let mut btn2 = button::Button::new(0, 0, button_size, button_size, "");
    btn2.set_image(Some(image::PngImage::from_data(include_bytes!("../img/grass.png")).unwrap()));

    toolbar.end();
    toolbar.set_type(group::PackType::Horizontal);
    toolbar.make_resizable(false);

    let mut editor = editor_widget::EditorWidget::new(0, 0, width, height - button_size);
    vlayout.resizable(&editor.widget);

    vlayout.end();
    vlayout.set_type(group::PackType::Vertical);

    win.end();

    btn1.set_callback(move |_| {
        let path = match dialog::file_chooser("Choose a celeste map", "*.bin", "/home/audrey/games/celeste/Content/Maps", false) {
            Some(v) => v,
            None => {return;}
        };
        let buf = match fs::read(path) {
            Ok(v) => v,
            Err(e) => {
                dialog::alert(center().0, center().1, format!("Could not load file: {}", e).as_str());
                return;
            }
        };
        let parsed = match celeste::binel::parser::take_file(buf.as_slice()) {
            Ok((_, v)) => v,
            Err(e) => {
                dialog::alert(center().0, center().1, format!("File is not a celeste map: {}", e).as_str());
                return;
            },
        };
        let map = match map_struct::from_binfile(parsed) {
            Ok(v) => v,
            Err(e) => {
                dialog::alert(center().0, center().1, format!("Data validation error: {}", e).as_str());
                return;
            },
        };

        editor.set_map(map);
        editor.reset_view();
    });

    win.make_resizable(true);
    win.show();
    app.run().unwrap();
    return Ok(());
}
