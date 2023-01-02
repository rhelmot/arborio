use super::{AttributeInfo, Expression};
use arborio_utils::vizia::prelude::Data;
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, Data)]
pub struct StylegroundConfigV1 {
    pub styleground_name: String,
    #[serde(default)]
    pub preview: Option<Expression>,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "version")]
pub enum StylegroundConfigStored {
    V1(StylegroundConfigV1),
}

pub type StylegroundConfig = StylegroundConfigV1;
