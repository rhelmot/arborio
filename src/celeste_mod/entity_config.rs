use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::celeste_mod::entity_expression::{Const, Expression};
use crate::units::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityConfig {
    pub entity_name: String, // TODO can this be an interned &'static str?
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub trigger_name: String,
    #[serde(default)]
    pub nodes: bool,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub templates: Vec<EntityTemplate>,
}

fn eight() -> u32 {
    8
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub ty: AttributeType,
    #[serde(default)]
    pub options: Vec<AttributeOption>,
    pub default: AttributeValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    pub value: AttributeValue,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityTemplate {
    pub name: String,
    pub attributes: HashMap<String, AttributeValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityRects {
    #[serde(default)]
    pub initial_rects: Vec<Rect>,
    #[serde(default)]
    pub node_rects: Vec<Rect>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EntityDraw {
    #[serde(default)]
    pub initial_draw: Vec<DrawElement>,
    #[serde(default)]
    pub node_draw: Vec<DrawElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DrawElement {
    DrawRect {
        rect: Rect,
        #[serde(default = "clear")]
        color: Color,
        #[serde(default = "clear")]
        border_color: Color,
        #[serde(default = "one")]
        border_thickness: u32,
    },
    DrawEllipse {
        rect: Rect,
        #[serde(default = "clear")]
        color: Color,
        #[serde(default = "clear")]
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
    DrawRectImage {
        texture: Expression,
        #[serde(default = "repeat")]
        tiler: Expression,
        bounds: Rect,
        #[serde(default = "empty_rect")]
        slice: Rect,
        #[serde(default = "one_one")]
        scale: Vec2,
        #[serde(default)]
        color: Color,
    },
    DrawPointImage {
        texture: Expression,
        point: Vec2,
        #[serde(default = "half")]
        justify_x: f32,
        #[serde(default = "half")]
        justify_y: f32,
        #[serde(default = "one_one")]
        scale: Vec2,
        #[serde(default)]
        color: Color,
        #[serde(default = "expr_zero")]
        rot: Expression,
    },
    DrawRectCustom {
        interval: f32,
        rect: Rect,
        draw: Vec<DrawElement>,
    },
}

fn one() -> u32 {
    1
}
fn one_one() -> Vec2 {
    Vec2 {
        x: Expression::mk_const(1),
        y: Expression::mk_const(1),
    }
}
fn empty_rect() -> Rect {
    Rect {
        topleft: Vec2 {
            x: Expression::mk_const(0),
            y: Expression::mk_const(0),
        },
        size: Vec2 {
            x: Expression::mk_const(0),
            y: Expression::mk_const(0),
        },
    }
}
fn half() -> f32 {
    0.5
}
fn clear() -> Color {
    Color {
        r: Expression::mk_const(0),
        g: Expression::mk_const(0),
        b: Expression::mk_const(0),
        a: Expression::mk_const(0),
    }
}
fn repeat() -> Expression {
    Expression::Const(Const::String("repeat".to_owned()))
}
fn expr_zero() -> Expression {
    Expression::mk_const(0)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rect {
    pub topleft: Vec2,
    pub size: Vec2,
}

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

impl Color {
    pub fn evaluate(&self, env: &HashMap<&str, Const>) -> Result<femtovg::Color, String> {
        let r = self.r.evaluate(env)?.as_number()?.to_int() as u8;
        let g = self.g.evaluate(env)?.as_number()?.to_int() as u8;
        let b = self.b.evaluate(env)?.as_number()?.to_int() as u8;
        let a = self.a.evaluate(env)?.as_number()?.to_int() as u8;
        Ok(femtovg::Color::rgba(r, g, b, a))
    }
}

impl Vec2 {
    pub fn mk_const(con_x: i32, con_y: i32) -> Vec2 {
        Vec2 {
            x: Expression::mk_const(con_x),
            y: Expression::mk_const(con_y),
        }
    }

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
    ) -> Result<euclid::Rect<f32, RoomSpace>, String> {
        let topleft = self.topleft.evaluate_float(env)?.to_point();
        let size = self.size.evaluate_float(env)?.to_size();
        Ok(euclid::Rect::new(topleft, size))
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

impl EntityConfig {
    pub fn default_template(&self) -> EntityTemplate {
        EntityTemplate {
            name: self.entity_name.clone(),
            attributes: HashMap::new(),
        }
    }
}

impl TriggerConfig {
    pub fn default_template(&self) -> EntityTemplate {
        EntityTemplate {
            name: self.trigger_name.clone(),
            attributes: HashMap::new(),
        }
    }
}

impl AttributeValue {
    pub fn to_binel(&self) -> celeste::binel::BinElAttr {
        match self {
            AttributeValue::String(s) => celeste::binel::BinElAttr::Text(s.clone()),
            AttributeValue::Float(f) => celeste::binel::BinElAttr::Float(*f),
            AttributeValue::Int(i) => celeste::binel::BinElAttr::Int(*i),
            AttributeValue::Bool(b) => celeste::binel::BinElAttr::Bool(*b),
        }
    }
}
