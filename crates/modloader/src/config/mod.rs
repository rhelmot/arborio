pub mod drawing;
pub mod entity;
pub mod expression;
pub mod styleground;
pub mod trigger;

use core::fmt;
use serde::de::VariantAccess;
use serde::{self, de};
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

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Data)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Lens, Data)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Data)]
pub struct AttributeOption {
    pub name: String,
    pub value: AttributeValue,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Data)]
pub enum AttributeType {
    String,
    Float,
    Int,
    Bool,
}

#[derive(Clone, Debug, PartialEq, Data)]
pub enum AttributeValue {
    String(String),
    Float(f32),
    Int(i32),
    Bool(bool),
}

impl Serialize for AttributeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AttributeValue::Bool(v) => serializer.serialize_bool(*v),
            AttributeValue::Int(v) => serializer.serialize_i32(*v),
            AttributeValue::Float(v) => serializer.serialize_f32(*v),
            AttributeValue::String(v) => serializer.serialize_str(v),
        }
    }
}

impl<'de> Deserialize<'de> for AttributeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AttributeVisitor)
    }
}

macro_rules! visit {
    () => {};
    ($type:tt => |$param:ident| $body:expr $(,$($remaining:tt)*)?) => {
        visit!($type => |self, $param: $type| $body $(,$($remaining)*)?);
    };
    ($name:tt => |$param:ident: $type:ty| $body:expr $(,$($remaining:tt)*)?) => {
        visit!($name => |self, $param: $type| $body $(,$($remaining)*)?);
    };
    ($name:tt => |$self:ident, $param:ident| $body:expr, $($remaining:tt)*) => {
        visit!($name => |$self, $param| $body);
        visit!($($remaining)*);
    };
    ($name:tt => |$self:ident, $param:ident: $type:ty| $body:expr, $($remaining:tt)*) => {
        visit!($name => |$self, $param: $type| $body);
        visit!($($remaining)*);
    };
    (($($type:ident)|+) => |$self:ident, $param:ident| $body:expr) => {
        $(visit!($type => |$self, $param: $type| $body);)+
    };
    ($type:ident => |$self:ident, $param:ident| $body:expr) => {
        visit!($type => |$self, $param: $type| $body);
    };
    ($name:ident => |$self:ident, $param:ident: $type:ty| $body:expr) => {
        concat_idents::concat_idents!(visit_name = visit_, $name {
            fn visit_name<E>($self, $param: $type) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                $body
            }
        });
    };
}

struct AttributeVisitor;

impl<'de> de::Visitor<'de> for AttributeVisitor {
    type Value = AttributeValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a String, Float, Int, or Bool")
    }

    visit! {
        bool => |v| Ok(AttributeValue::Bool(v)),
        i32 => |v| Ok(AttributeValue::Int(v)),
        f32 => |v| Ok(AttributeValue::Float(v)),
        string => |v: String| Ok(AttributeValue::String(v)),
        (i64 | u64) => |self, v| {
            if let Ok(v) = v.try_into() {
                self.visit_i32(v)
            } else {
                self.visit_f32(v as f32)
            }
        },
        f64 => |self, v| self.visit_f32(v as f32),
        str => |self, v: &str| self.visit_string(v.to_owned()),
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        let (value, contents) = data.variant::<AttributeType>()?;
        Ok(match value {
            AttributeType::Bool => AttributeValue::Bool(contents.newtype_variant()?),
            AttributeType::Int => AttributeValue::Int(contents.newtype_variant()?),
            AttributeType::Float => AttributeValue::Float(contents.newtype_variant()?),
            AttributeType::String => AttributeValue::String(contents.newtype_variant()?),
        })
    }
}

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Data)]
pub struct EntityTemplate {
    pub name: Interned,
    #[serde(default, skip_serializing_if = "is_default")]
    pub keywords: Vec<String>,
    pub attributes: HashMap<Interned, AttributeValue>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default, Data)]
pub struct EntityRects {
    #[serde(default)]
    pub initial_rects: Vec<Rect>,
    #[serde(default)]
    pub node_rects: Vec<Rect>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Data)]
pub struct Rect {
    pub topleft: Vec2,
    pub size: Vec2,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Data)]
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
