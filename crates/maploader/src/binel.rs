pub mod parser;
pub mod writer;

use std::collections::HashMap;

/// This module is a moderately modified copy of much of the source of the now unmaintained celeste crate, by leo60228

/// A value stored in an attribute inside a `BinEl`. Unlike XML, attributes are strongly typed.
#[derive(Debug, PartialEq, Clone)]
pub enum BinElAttr {
    Bool(bool),
    Int(i32),
    Float(f32),
    Text(String),
}

/// An element stored in a `BinFile`. Based on XML.
#[derive(PartialEq, Debug, Clone, Default)]
pub struct BinEl {
    /// The name of the `BinEl`.
    pub name: String,
    /// All attributes of the `BinEl`. Unlike XML, these are strongly typed.
    pub attributes: HashMap<String, BinElAttr>,
    children: HashMap<String, Vec<BinEl>>,
}

impl BinEl {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            attributes: HashMap::new(),
            children: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, child: Self) {
        self.children
            .entry(child.name.clone())
            .or_default()
            .push(child);
    }

    pub(crate) fn children(&self) -> impl Iterator<Item = &BinEl> {
        self.children.values().flatten()
    }

    pub(crate) fn get(&self, key: &str) -> &[Self] {
        self.children
            .get(key)
            .map(AsRef::as_ref)
            .unwrap_or_default()
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> &mut Vec<Self> {
        self.children.entry(key.to_owned()).or_default()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = BinEl> + '_ {
        self.children.drain().flat_map(|(_, v)| v)
    }

    pub fn text(&self) -> Option<&str> {
        let BinElAttr::Text(t) = self.attributes.get("innerText")? else {
            return None
        };
        Some(t)
    }
}

/// Holds `BinaryElement` files.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct BinFile {
    pub package: String,
    pub root: BinEl,
}
