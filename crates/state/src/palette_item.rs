use crate::data::app::AppState;
use crate::rendering::draw_entity;
use arborio_maploader::map_struct::{CelesteMapDecal, CelesteMapEntity, Node};
use arborio_modloader::config::{EntityConfig, TriggerConfig};
use arborio_modloader::selectable::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use itertools::Itertools;
use std::fmt::Debug;
use std::sync::Arc;

pub trait PaletteItem: 'static + Copy + Data + Debug + Send {
    fn search_text(&self, app: &AppState) -> String;
    fn display_name(&self, app: &AppState) -> String;
    const CAN_DRAW: bool = true;
    fn draw(&self, app: &AppState, canvas: &mut Canvas, other_name: &str);
    fn other() -> Self;
}

impl PaletteItem for TileSelectable {
    fn search_text(&self, _app: &AppState) -> String {
        self.name.to_owned()
    }

    fn display_name(&self, _app: &AppState) -> String {
        self.name.to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas, _other_name: &str) {
        if let Some(texture) = self.texture {
            if !app.map_tab_check() {
                println!("SOMETHING IS WRONG (list)");
            } else {
                canvas.scale(3.0, 3.0);
                if let Err(e) = app.current_palette_unwrap().gameplay_atlas.draw_sprite(
                    canvas,
                    texture,
                    Point2D::zero(),
                    None,
                    Some(Vector2D::zero()),
                    None,
                    None,
                    0.0,
                ) {
                    log::error!("Error drawing tileset: {}", e);
                }
            }
        }
    }

    fn other() -> Self {
        Self {
            id: '\0',
            name: "other",
            texture: None,
        }
    }
}

impl PaletteItem for EntitySelectable {
    fn search_text(&self, app: &AppState) -> String {
        let config = get_entity_config(self, app);
        let template = &config.templates[self.template];
        config
            .keywords
            .iter()
            .chain(config.templates[self.template].keywords.iter())
            .chain([&config.entity_name.to_string()].into_iter())
            .chain([&template.name.to_string()].into_iter())
            .join(" ")
    }

    fn display_name(&self, app: &AppState) -> String {
        (*get_entity_config(self, app).templates[self.template].name).to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas, other_name: &str) {
        canvas.scale(2.0, 2.0);

        let tmp_entity = instantiate_entity(
            self,
            other_name,
            app,
            16,
            16,
            get_entity_config(self, app).minimum_size_x as i32,
            get_entity_config(self, app).minimum_size_y as i32,
            vec![(48, 16).into()],
        );
        draw_entity(
            app.current_palette_unwrap()
                .get_entity_config(&tmp_entity.name, false),
            app.current_palette_unwrap(),
            canvas,
            &tmp_entity,
            &TileGrid::empty(),
            false,
            &TileGrid::empty(),
        )
    }

    fn other() -> Self {
        Self {
            entity: "arborio/other".into(),
            template: 0,
        }
    }
}

impl PaletteItem for TriggerSelectable {
    fn search_text(&self, app: &AppState) -> String {
        let config = get_trigger_config(self, app);
        let template = &config.templates[self.template];
        config
            .keywords
            .iter()
            .chain(config.templates[self.template].keywords.iter())
            .chain([&config.trigger_name.to_string()].into_iter())
            .chain([&template.name.to_string()].into_iter())
            .join(" ")
    }

    fn display_name(&self, app: &AppState) -> String {
        (*get_trigger_config(self, app).templates[self.template].name).to_owned()
    }

    const CAN_DRAW: bool = false;
    fn draw(&self, _app: &AppState, _canvas: &mut Canvas, _other_name: &str) {
        panic!("You cannot draw a trigger. don't call me!")
    }

    fn other() -> Self {
        Self {
            trigger: "arborio/other".into(),
            template: 0,
        }
    }
}

pub fn get_entity_config<'a>(this: &EntitySelectable, app: &'a AppState) -> &'a Arc<EntityConfig> {
    app.current_palette_unwrap()
        .get_entity_config(&this.entity, false)
}

#[allow(clippy::too_many_arguments)]
pub fn instantiate_entity(
    this: &EntitySelectable,
    other: &str,
    app: &AppState,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    nodes: Vec<Node>,
) -> CelesteMapEntity {
    let config = get_entity_config(this, app);

    let (x, width) = if width < 0 {
        (x + width, -width as u32)
    } else {
        (x, width as u32)
    };
    let (y, height) = if height < 0 {
        (y + height, -height as u32)
    } else {
        (y, height as u32)
    };
    let width = width.max(config.minimum_size_x);
    let height = height.max(config.minimum_size_y);
    let width = if !config.resizable_x {
        config.minimum_size_x
    } else {
        width
    };
    let height = if !config.resizable_y {
        config.minimum_size_y
    } else {
        height
    };

    let mut entity = CelesteMapEntity {
        id: 0,
        name: if *this.entity == "arborio/other" {
            other.to_owned()
        } else {
            config.entity_name.to_string()
        },
        attributes: config.templates[this.template]
            .attributes
            .iter()
            .map(|attr| (attr.0.to_string(), attr.1.to_binel()))
            .collect(),
        x,
        y,
        width,
        height,
        nodes,
    };
    for (attr, info) in config.attribute_info.iter() {
        if !entity.attributes.contains_key(attr) {
            entity
                .attributes
                .insert(attr.to_string(), info.default.to_binel());
        }
    }

    entity
}

pub fn get_trigger_config<'a>(
    this: &TriggerSelectable,
    app: &'a AppState,
) -> &'a Arc<TriggerConfig> {
    let palette = app.current_palette_unwrap();
    palette
        .trigger_config
        .get(&this.trigger)
        .unwrap_or_else(|| palette.trigger_config.get("default").unwrap())
}

#[allow(clippy::too_many_arguments)]
pub fn instantiate_trigger(
    this: &TriggerSelectable,
    other: &str,
    app: &AppState,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    nodes: Vec<Node>,
) -> CelesteMapEntity {
    let config = get_trigger_config(this, app);

    let (x, width) = if width < 0 {
        (x + width, -width as u32)
    } else {
        (x, width as u32)
    };
    let (y, height) = if height < 0 {
        (y + height, -height as u32)
    } else {
        (y, height as u32)
    };
    let width = width.max(8);
    let height = height.max(8);

    let mut entity = CelesteMapEntity {
        id: 0,
        name: if *this.trigger == "arborio/other" {
            other.to_owned()
        } else {
            config.trigger_name.to_string()
        },
        attributes: config.templates[this.template]
            .attributes
            .iter()
            .map(|attr| (attr.0.to_string(), attr.1.to_binel()))
            .collect(),
        x,
        y,
        width,
        height,
        nodes,
    };
    for (attr, info) in config.attribute_info.iter() {
        if !entity.attributes.contains_key(attr) {
            entity
                .attributes
                .insert(attr.to_string(), info.default.to_binel());
        }
    }

    entity
}

impl PaletteItem for DecalSelectable {
    fn search_text(&self, _app: &AppState) -> String {
        self.0.to_string()
    }

    fn display_name(&self, _app: &AppState) -> String {
        self.0.to_string()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas, other_name: &str) {
        app.current_palette_unwrap()
            .gameplay_atlas
            .draw_sprite(
                canvas,
                &format!(
                    "decals/{}",
                    if *self.0 == "arborio/other" {
                        other_name
                    } else {
                        *self.0
                    }
                ),
                Point2D::new(0.0, 0.0),
                None,
                Some(Vector2D::zero()),
                None,
                None,
                0.0,
            )
            .ok();
    }

    fn other() -> Self {
        Self("arborio/other".into())
    }
}

pub fn instantiate_decal(
    this: &DecalSelectable,
    other: &str,
    x: i32,
    y: i32,
    scale_x: f32,
    scale_y: f32,
) -> CelesteMapDecal {
    CelesteMapDecal {
        id: 0,
        x,
        y,
        scale_x,
        scale_y,
        texture: if *this.0 == "arborio/other" {
            other.to_string()
        } else {
            this.0.to_string()
        },
    }
}
