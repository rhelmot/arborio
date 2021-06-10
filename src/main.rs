#![allow(unused)]
#![allow(clippy::needless_return)]

mod editor_widget;
mod map_struct;
mod atlas_img;
mod autotiler;

use std::fs;
use std::error::Error;
use fltk::{prelude::*,*};
use std::path::Path;
use std::rc::Rc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use try_or::try_opt_or;

#[derive(Serialize, Deserialize, Default)]
struct Config {
    celeste_path: String,
}

#[inline(always)]
pub fn center() -> (i32, i32) {
    (
        (app::screen_size().0 / 2.0) as i32,
        (app::screen_size().1 / 2.0) as i32,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    // if load from file fails, continue with default config
    let cfg: Option<Config> = confy::load("arborio").map_or(Default::default(), |mut cfg: Config| {
        if cfg.celeste_path.is_empty() {
            cfg.celeste_path = dialog::file_chooser("Please choose Celeste.exe", "Celeste.exe", ".", false)?;
            let _ = confy::store("arborio", &cfg); // If storage fails, continue anyway
        }
        Some(cfg)
    });
    let cfg = try_opt_or!(cfg, Ok(())); // If user didn't choose a path, exit cleanly

    let content_root = Path::new(cfg.celeste_path.as_str()).parent().unwrap().join("Content");

    let width = 1000;
    let height = 600;
    let button_size = 25;

    let atlas = Rc::new(atlas_img::Atlas::load(content_root.join("Graphics/Atlases/Gameplay.meta").as_path())?);
    let mut fgtiles: HashMap<char, autotiler::Tileset> = HashMap::new();
    let added = autotiler::Tileset::load(content_root.join("Graphics/ForegroundTiles.xml").as_path(), &atlas, &mut fgtiles)?;
    let fgtiles = Rc::new(fgtiles);

    let app = app::App::default();
    app::set_visual(enums::Mode::Rgb).unwrap();

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

    let mut editor = editor_widget::EditorWidget::new(0, 0, width, height - button_size, atlas, fgtiles);
    vlayout.resizable(&editor.widget);

    vlayout.end();
    vlayout.set_type(group::PackType::Vertical);

    win.end();

    btn1.set_callback(move |_| {
        // TODO store last used dir in config
        let path = match dialog::file_chooser("Choose a celeste map", "*.bin", content_root.to_str().unwrap(), false) {
            Some(v) => v,
            None => return,
        };

        let result: Result<_, String> = (|| {
            let buf = fs::read(path).map_err(|e| format!("Could not load file: {}", e))?;
            let (_, parsed) = celeste::binel::parser::take_file(buf.as_slice())
                .map_err(|e| format!("File is not a celeste map: {}", e))?;
            let map = map_struct::from_binfile(parsed)
                .map_err(|e| format!("Data validation error: {}", e))?;
            Ok(map)
        })();

        match result {
            Ok(map) => {
                editor.set_map(map);
                editor.reset_view();
            }
            Err(err) => {
                dialog::alert(center().0, center().1, &err);
            }
        }
    });

    win.make_resizable(true);
    win.show();
    app.run().unwrap();
    return Ok(());
}
