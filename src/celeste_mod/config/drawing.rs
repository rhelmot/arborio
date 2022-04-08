use super::{Const, Expression, Rect, Vec2};
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vizia::vg;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct EntityDraw {
    #[serde(default)]
    pub initial_draw: Vec<DrawElement>,
    #[serde(default)]
    pub node_draw: Vec<DrawElement>,
}

#[allow(clippy::large_enum_variant, clippy::enum_variant_names)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Color {
    pub r: Expression,
    pub g: Expression,
    pub b: Expression,
    pub a: Expression,
}

impl Color {
    pub fn evaluate(&self, env: &HashMap<&str, Const>) -> Result<vg::Color, String> {
        let r = self.r.evaluate(env)?.as_number()?.to_int() as u8;
        let g = self.g.evaluate(env)?.as_number()?.to_int() as u8;
        let b = self.b.evaluate(env)?.as_number()?.to_int() as u8;
        let a = self.a.evaluate(env)?.as_number()?.to_int() as u8;
        Ok(vg::Color::rgba(r, g, b, a))
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

fn one() -> u32 {
    1
}
fn one_one() -> Vec2 {
    Vec2 {
        x: Expression::mk_const(1),
        y: Expression::mk_const(1),
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
fn repeat() -> Expression {
    Expression::Const(Const::String("repeat".to_owned()))
}
fn expr_zero() -> Expression {
    Expression::mk_const(0)
}
