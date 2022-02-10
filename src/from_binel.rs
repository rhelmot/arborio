use std::collections::HashMap;

use crate::map_struct::{get_child_mut, get_optional_child, CelesteMap, CelesteMapError};
pub use arborio_derive::TryFromBinEl;
use celeste::binel::{BinEl, BinElAttr};
use itertools::Itertools;

macro_rules! expect_elem {
    ($elem:expr, $name:expr) => {
        if ($elem.name != $name) {
            return Err(CelesteMapError {
                kind: CelesteMapErrorType::ParseError,
                description: format!("Expected {} element, found {}", $name, $elem.name),
            });
        }
    };
}

pub fn get_nested_child<'a>(elem: &'a BinEl, name: &str) -> Option<&'a BinEl> {
    if let Some((first, remaining)) = name.split_once('/') {
        get_nested_child(get_optional_child(elem, first)?, remaining)
    } else {
        get_optional_child(elem, name)
    }
}

pub trait TryFromBinEl: Sized {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError>;
    fn to_binel(&self) -> BinEl;
}

impl<T> TryFromBinEl for Vec<T>
where
    T: TryFromBinEl,
{
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
        elem.children()
            .map(|child| T::try_from_bin_el(child))
            .collect()
    }
    fn to_binel(&self) -> BinEl {
        let mut b = BinEl::new("");
        for child in self {
            b.insert(child.to_binel())
        }
        b
    }
}

pub trait GetAttrOrChild: Sized {
    fn attr_or_child<'a>(elem: &'a BinEl, key: &str) -> Option<&'a Self>;
    fn nested_attr_or_child<'a>(elem: &'a BinEl, key: &str) -> Option<&'a Self> {
        if let Some((first, remaining)) = key.split_once('/') {
            Self::nested_attr_or_child(get_optional_child(elem, first)?, remaining)
        } else {
            Self::attr_or_child(elem, key)
        }
    }
    fn apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self);
    fn nested_apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self) {
        if let Some((first, remaining)) = key.split_once('/') {
            Self::nested_apply_attr_or_child(get_child_mut(elem, first), remaining, value)
        } else {
            Self::apply_attr_or_child(elem, key, value)
        }
    }
}

impl GetAttrOrChild for BinEl {
    fn attr_or_child<'a>(elem: &'a BinEl, key: &str) -> Option<&'a Self> {
        if let Some((x,)) = elem.get(key).iter().collect_tuple() {
            Some(x)
        } else {
            None
        }
    }

    fn apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self) {
        std::mem::replace(elem.get_mut(key), vec![value]);
    }
}

impl GetAttrOrChild for BinElAttr {
    fn attr_or_child<'a>(elem: &'a BinEl, key: &str) -> Option<&'a Self> {
        elem.attributes.get(key)
    }
    fn apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self) {
        elem.attributes.insert(key.to_owned(), value);
    }
}

pub trait TwoWayConverter<T> {
    type BinType: GetAttrOrChild;

    fn try_parse(elem: &Self::BinType) -> Result<T, CelesteMapError>;
    fn serialize(val: &T) -> Self::BinType;

    fn from_bin_el(elem: &BinEl, key: &str) -> Result<T, CelesteMapError> {
        Self::try_parse(
            GetAttrOrChild::nested_attr_or_child(elem, key)
                .ok_or_else(|| CelesteMapError::missing_child(&elem.name, key))?,
        )
    }
    fn from_bin_el_optional(elem: &BinEl, key: &str) -> Result<Option<T>, CelesteMapError> {
        let got = GetAttrOrChild::nested_attr_or_child(elem, key);
        got.map(Self::try_parse).transpose()
    }
}

pub(crate) fn bin_el_fuzzy_equal(first: &BinEl, second: &BinEl) -> bool {
    dbg!(first, second);
    if first.name != second.name {
        return false;
    }
    dbg!("");
    if first.attributes.len() != second.attributes.len() {
        return false;
    }
    dbg!("");
    for (key, value) in &first.attributes {
        if let Some(value2) = second.attributes.get(key) {
            if !bin_el_attr_fuzzy_equal(value, value2) {
                return false;
            }
        } else {
            return false;
        }
    }
    dbg!("");
    let mut first_children = HashMap::new();
    let mut second_children = HashMap::new();

    for child in first.children() {
        first_children
            .entry(child.name.clone())
            .or_insert_with(Vec::new)
            .push(child);
    }
    for child in second.children() {
        second_children
            .entry(child.name.clone())
            .or_insert_with(Vec::new)
            .push(child);
    }

    if first_children.len() != second_children.len() {
        return false;
    }
    dbg!("comparing children");

    for (key, value) in first_children {
        if let Some(value2) = second_children.get(&key) {
            for (one, &two) in value.into_iter().zip_eq(value2) {
                if !bin_el_fuzzy_equal(one, two) {
                    return false;
                }
            }
        } else {
            return false;
        }
    }
    true
}

fn bin_el_attr_fuzzy_equal(first: &BinElAttr, second: &BinElAttr) -> bool {
    match (first, second) {
        (BinElAttr::Bool(_), BinElAttr::Int(_)) => todo!(),
        (BinElAttr::Bool(_), BinElAttr::Float(_)) => todo!(),
        (BinElAttr::Bool(_), BinElAttr::Text(_)) => todo!(),
        (BinElAttr::Int(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Int(i), BinElAttr::Float(f)) => *i as f32 == *f && *f as i32 == *i,
        (BinElAttr::Int(_), BinElAttr::Text(_)) => todo!(),
        (BinElAttr::Float(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Float(f), BinElAttr::Int(i)) => *f as i32 == *i,
        (BinElAttr::Float(_), BinElAttr::Text(_)) => todo!(),
        (BinElAttr::Text(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Text(_), BinElAttr::Int(_)) => todo!(),
        (BinElAttr::Text(_), BinElAttr::Float(_)) => todo!(),
        _ => first == second,
    }
}
