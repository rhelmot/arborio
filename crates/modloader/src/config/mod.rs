pub mod drawing;
pub mod entity;
pub mod expression;
pub mod styleground;
pub mod trigger;

use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::str::FromStr;

use arborio_maploader::map_struct::Attribute;
use arborio_utils;
use arborio_utils::default::is_default;
use arborio_utils::interned::{intern_str, Interned};
use arborio_utils::units::{Rect as CRect, *};
use arborio_utils::vizia::prelude::*;

pub use drawing::*;
pub use entity::*;
pub use expression::*;
pub use styleground::*;
pub use trigger::*;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum PencilBehavior {
    // TODO: Place
    Line,
    Node,
    Rect,
}

impl Default for PencilBehavior {
    fn default() -> Self {
        PencilBehavior::Line
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Lens)]
pub struct AttributeInfo {
    #[serde(default, skip_serializing_if = "is_default")]
    pub display_name: Option<String>,
    pub ty: AttributeType,
    pub default: AttributeValue,
    #[serde(default, skip_serializing_if = "is_default")]
    pub options: Vec<AttributeOption>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub ignore: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct AttributeOption {
    pub name: String,
    pub value: AttributeValue,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum AttributeType {
    String,
    Float,
    Int,
    Bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Float(f32),
    Int(i32),
    Bool(bool),
}

impl PartialEq for AttributeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(s1), Self::String(s2)) => s1 == s2,
            (Self::Float(s1), Self::Float(s2)) => s1.to_bits() == s2.to_bits(),
            (Self::Int(s1), Self::Int(s2)) => s1 == s2,
            (Self::Bool(s1), Self::Bool(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl Eq for AttributeValue {}

impl AttributeValue {
    pub fn ty(&self) -> AttributeType {
        match self {
            AttributeValue::String(_) => AttributeType::String,
            AttributeValue::Float(_) => AttributeType::Float,
            AttributeValue::Int(_) => AttributeType::Int,
            AttributeValue::Bool(_) => AttributeType::Bool,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct EntityTemplate {
    pub name: Interned,
    #[serde(default, skip_serializing_if = "is_default")]
    pub keywords: Vec<String>,
    pub attributes: HashMap<Interned, AttributeValue>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Default)]
pub struct EntityRects {
    #[serde(default)]
    pub initial_rects: Vec<Rect>,
    #[serde(default)]
    pub node_rects: Vec<Rect>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Rect {
    pub topleft: Vec2,
    pub size: Vec2,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Vec2 {
    pub x: Expression,
    pub y: Expression,
}

impl Vec2 {
    pub fn evaluate_int(&self, env: &HashMap<&str, Const>) -> Result<RoomVector, String> {
        let x = self.x.evaluate(env)?.as_number()?.to_int();
        let y = self.y.evaluate(env)?.as_number()?.to_int();
        Ok(RoomVector::new(x, y))
    }

    pub fn evaluate_float(
        &self,
        env: &HashMap<&str, Const>,
    ) -> Result<Vector2D<f32, RoomSpace>, String> {
        let x = self.x.evaluate(env)?.as_number()?.to_float();
        let y = self.y.evaluate(env)?.as_number()?.to_float();
        Ok(Vector2D::new(x, y))
    }
}

impl Rect {
    pub fn evaluate_int(&self, env: &HashMap<&str, Const>) -> Result<RoomRect, String> {
        let topleft = self.topleft.evaluate_int(env)?.to_point();
        let size = self.size.evaluate_int(env)?.to_size();
        Ok(RoomRect::new(topleft, size))
    }

    pub fn evaluate_float(
        &self,
        env: &HashMap<&str, Const>,
    ) -> Result<CRect<f32, RoomSpace>, String> {
        let topleft = self.topleft.evaluate_float(env)?.to_point();
        let size = self.size.evaluate_float(env)?.to_size();
        Ok(CRect::new(topleft, size))
    }
}

impl EntityConfig {
    pub fn default_template(&self) -> EntityTemplate {
        EntityTemplate {
            name: intern_str(&self.entity_name),
            attributes: HashMap::new(),
            keywords: vec![],
        }
    }
}

impl TriggerConfig {
    pub fn default_template(&self) -> EntityTemplate {
        EntityTemplate {
            name: intern_str(&self.trigger_name),
            attributes: HashMap::new(),
            keywords: vec![],
        }
    }
}

impl AttributeValue {
    pub fn to_binel(&self) -> Attribute {
        match self {
            AttributeValue::String(s) => Attribute::Text(s.clone()),
            AttributeValue::Float(f) => Attribute::Float(*f),
            AttributeValue::Int(i) => Attribute::Int(*i),
            AttributeValue::Bool(b) => Attribute::Bool(*b),
        }
    }
}

// TODO there has GOTTA be a way to generalize this
impl std::fmt::Display for EntityConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        serde_yaml::to_string(self).unwrap().fmt(f)
    }
}

impl std::fmt::Display for TriggerConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        serde_yaml::to_string(self).unwrap().fmt(f)
    }
}

impl std::fmt::Display for StylegroundConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        serde_yaml::to_string(self).unwrap().fmt(f)
    }
}

impl FromStr for EntityConfig {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl FromStr for TriggerConfig {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl FromStr for StylegroundConfig {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl EntityConfig {
    pub fn new(name: &str) -> Self {
        Self {
            entity_name: name.to_owned(),
            ..Self::default()
        }
    }
}

impl TriggerConfig {
    pub fn new(name: &str) -> Self {
        Self {
            trigger_name: name.to_owned(),
            ..Self::default()
        }
    }
}

impl StylegroundConfig {
    pub fn new(name: &str) -> Self {
        Self {
            styleground_name: name.to_owned(),
            ..Self::default()
        }
    }
}
