use super::drawing::EntityDraw;
use super::EntityRects;
use crate::config::{AttributeInfo, EntityTemplate, PencilBehavior};
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct EntityConfigV1 {
    pub entity_name: String,
    pub hitboxes: EntityRects,
    #[serde(default)]
    pub standard_draw: EntityDraw,
    #[serde(default)]
    pub selected_draw: EntityDraw,
    #[serde(default = "eight")]
    pub minimum_size_x: u32,
    #[serde(default = "eight")]
    pub minimum_size_y: u32,
    pub resizable_x: bool,
    pub resizable_y: bool,
    #[serde(default)]
    pub nodes: bool,
    #[serde(default)]
    pub pencil: PencilBehavior,
    #[serde(default)]
    pub solid: bool,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub templates: Vec<EntityTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct EntityConfigV2 {
    pub entity_name: String,
    pub hitboxes: EntityRects,
    #[serde(default)]
    pub standard_draw: EntityDraw,
    #[serde(default)]
    pub selected_draw: EntityDraw,
    #[serde(default = "eight")]
    pub minimum_size_x: u32,
    #[serde(default = "eight")]
    pub minimum_size_y: u32,
    pub resizable_x: bool,
    pub resizable_y: bool,
    #[serde(default)]
    pub nodes: bool,
    #[serde(default)]
    pub pencil: PencilBehavior,
    #[serde(default)]
    pub solid: bool,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub templates: Vec<EntityTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "version")]
pub enum EntityConfigStored {
    V1(EntityConfigV1),
    V2(EntityConfigV2),
}

pub type EntityConfig = EntityConfigV2;

fn eight() -> u32 {
    8
}
