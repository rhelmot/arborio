use super::{AttributeInfo, EntityTemplate};
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct TriggerConfigV1 {
    pub trigger_name: String,
    #[serde(default)]
    pub nodes: bool,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub templates: Vec<EntityTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct TriggerConfigV2 {
    pub trigger_name: String,
    #[serde(default)]
    pub nodes: bool,
    #[serde(default)]
    pub attribute_info: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub templates: Vec<EntityTemplate>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "version")]
pub enum TriggerConfigStored {
    V1(TriggerConfigV1),
    V2(TriggerConfigV2),
}

pub type TriggerConfig = TriggerConfigV2;
