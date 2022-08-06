use crate::data::app::AppState;
use crate::rendering::draw_entity;
use arborio_maploader::map_struct::{CelesteMapEntity, Node};
use arborio_modloader::config::{EntityConfig, TriggerConfig};
use arborio_modloader::selectable::{
    DecalSelectable, EntitySelectable, TileSelectable, TriggerSelectable,
};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::*;
use std::fmt::Debug;
use std::sync::Arc;

pub trait PaletteItem: Copy + Clone + Data + Debug + Send {
    fn search_text(&self) -> String;
    fn display_name(&self, app: &AppState) -> String;
    const CAN_DRAW: bool = true;
    fn draw(&self, app: &AppState, canvas: &mut Canvas);
}

impl PaletteItem for TileSelectable {
    fn search_text(&self) -> String {
        self.name.to_owned()
    }

    fn display_name(&self, _app: &AppState) -> String {
        self.name.to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) {
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
}

impl PaletteItem for EntitySelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, app: &AppState) -> String {
        (*get_entity_config(self, app).templates[self.template].name).to_owned()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) {
        canvas.scale(2.0, 2.0);

        let tmp_entity = instantiate_entity(
            self,
            app,
            16,
            16,
            get_entity_config(self, app).minimum_size_x as i32,
            get_entity_config(self, app).minimum_size_y as i32,
            vec![(48, 16).into()],
        );
        draw_entity(
            app,
            canvas,
            &tmp_entity,
            &TileGrid::empty(),
            false,
            false,
            &TileGrid::empty(),
        )
    }
}

impl PaletteItem for TriggerSelectable {
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, app: &AppState) -> String {
        (*get_trigger_config(self, app).templates[self.template].name).to_owned()
    }

    const CAN_DRAW: bool = false;
    fn draw(&self, _app: &AppState, _canvas: &mut Canvas) {
        panic!("You cannot draw a trigger. don't call me!")
    }
}

pub fn get_entity_config<'a>(this: &EntitySelectable, app: &'a AppState) -> &'a Arc<EntityConfig> {
    app.current_palette_unwrap()
        .get_entity_config(&this.entity, false)
}

pub fn instantiate_entity(
    this: &EntitySelectable,
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
        name: config.entity_name.to_string(),
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

pub fn instantiate_trigger(
    this: &TriggerSelectable,
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
        name: config.trigger_name.to_string(),
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
    fn search_text(&self) -> String {
        todo!()
    }

    fn display_name(&self, _app: &AppState) -> String {
        self.0.to_string()
    }

    fn draw(&self, app: &AppState, canvas: &mut Canvas) {
        if let Err(e) = app.current_palette_unwrap().gameplay_atlas.draw_sprite(
            canvas,
            &format!("decals/{}", self.0),
            Point2D::new(0.0, 0.0),
            None,
            Some(Vector2D::zero()),
            None,
            None,
            0.0,
        ) {
            log::error!("Error drawing decal: {}", e);
        }
    }
}
