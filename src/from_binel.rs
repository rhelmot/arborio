use crate::map_struct::{get_optional_child, CelesteMap, CelesteMapError};
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

pub trait TryFromBinEl<Err>: Sized {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, Err>;
}

impl<T, E> TryFromBinEl<E> for Vec<T>
where
    T: TryFromBinEl<E>,
{
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, E> {
        elem.children()
            .map(|child| T::try_from_bin_el(child))
            .collect()
    }
}

pub trait GetAttrOrChild: Sized {
    fn attr_or_child<'a>(elem: &'a BinEl, key: &str) -> Option<&'a Self>;
    fn apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self);
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
    fn apply_attr_or_child(elem: &mut BinEl, key: &str, value: Self) {}
}

pub trait TwoWayConverter<T> {
    type BinType: GetAttrOrChild;

    fn try_parse(elem: &Self::BinType) -> Result<T, CelesteMapError>;
    fn serialize(val: T) -> Self::BinType;

    fn from_bin_el(elem: &BinEl, key: &str) -> Result<T, CelesteMapError> {
        Self::try_parse(
            GetAttrOrChild::attr_or_child(elem, key)
                .ok_or_else(|| CelesteMapError::missing_child(&elem.name, key))?,
        )
    }
    fn from_bin_el_optional(elem: &BinEl, key: &str) -> Result<Option<T>, CelesteMapError> {
        let got = GetAttrOrChild::attr_or_child(elem, key);
        if let Some(got) = got {
            Ok(Some(Self::try_parse(got)?))
        } else {
            Ok(None)
        }
    }
}
