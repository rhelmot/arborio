use serde;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityConfig {
    pub entity_name: String,
    pub hitboxes: EntityRects,
    pub standard_draw: EntityDraw,
    pub selected_draw: EntityDraw,
    #[serde(default = "eight")]
    pub minimum_size_x: u32,
    #[serde(default = "eight")]
    pub minimum_size_y: u32,
    pub resizable_x: bool,
    pub resizable_y: bool,
    #[serde(default)]
    pub nodes: bool,
    pub attribute_info: HashMap<String, AttributeInfo>,
}

fn eight() -> u32 { 8 }

#[derive(Debug, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub ty: AttributeType,
    #[serde(default)]
    pub options: Vec<AttributeValue>,
    pub default: AttributeValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AttributeType {
    String,
    Float,
    Int,
    Bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Float(f32),
    Int(i32),
    Bool(bool),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityRects {
    #[serde(default)]
    pub initial_rects: Vec<Rect>,
    #[serde(default)]
    pub node_rects: Vec<Rect>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityDraw {
    #[serde(default)]
    initial_draw: Vec<DrawElement>,
    #[serde(default)]
    node_draw: Vec<DrawElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DrawElement {
    DrawRect {
        rect: Rect,
        color: Color,
        border_color: Color,
        #[serde(default = "one")]
        border_thickness: u32,
    },
    DrawLine {
        start: Vec2,
        end: Vec2,
        color: Color,
        #[serde(default)]
        arrowhead: bool,
        #[serde(default = "one")]
        thickness: u32,
    },
    DrawCurve {
        start: Vec2,
        end: Vec2,
        middle: Vec2,
        color: Color,
        #[serde(default = "one")]
        thickness: u32,
    },
    DrawImage {
        texture: Expression,
        bounds: Rect,  // special sematics: default (0,0) bounds size means inherit texture's size times abs(scale)
        #[serde(default = "one_one")]
        scale: Vec2,
        #[serde(default)]
        rot: i32,
        #[serde(default)]
        color: Color,
        #[serde(default = "fg")]
        tiler: AutotilerType,
    },
}

fn one() -> u32 { 1 }
fn one_one() -> Vec2 { Vec2 { x: Expression::mk_const(1), y: Expression::mk_const(1) } }

#[derive(Debug, Serialize, Deserialize)]
pub enum AutotilerType {
    Fg, Bg,
    Cassette,
    NineSlice,
}

fn fg() -> AutotilerType { AutotilerType::Fg }

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub topleft: Vec2,
    #[serde(default = "zero_zero")]
    pub size: Vec2,
}

fn zero_zero() -> Vec2 { Vec2 { x: Expression::mk_const(0), y: Expression::mk_const(0) }}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: Expression,
    pub y: Expression,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Color {
    pub r: Expression,
    pub g: Expression,
    pub b: Expression,
    pub a: Expression,
}

#[derive(Debug)]
pub enum Expression {
    Dummy(String)
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Expression::Dummy(s.to_owned() + " :)"))
    }
}

impl Serialize for Expression {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Expression::Dummy(val) => val.strip_suffix(" :)")
        }.serialize(s)
    }
}

impl Expression {
    pub fn mk_const(con: i32) -> Expression {
        Expression::Dummy(con.to_string() + " :)")
    }
}

impl Vec2 {
    pub fn mk_const(con_x: i32, con_y: i32) -> Vec2 {
        Vec2 {
            x: Expression::mk_const(con_x),
            y: Expression::mk_const(con_y),
        }
    }
}

impl Default for Color {
    fn default() -> Color {
        Color {
            r: Expression::mk_const(255),
            g: Expression::mk_const(255),
            b: Expression::mk_const(255),
            a: Expression::mk_const(255),
        }
    }
}