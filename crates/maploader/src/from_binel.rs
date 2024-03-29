use std::collections::{HashMap, HashSet};

use crate::{
    binel::{BinEl, BinElAttr},
    map_struct::{get_child_mut, get_optional_child, CelesteMapError},
};
pub use arborio_derive::TryFromBinEl;
use itertools::Itertools;
use once_cell::sync::Lazy;

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

impl TryFromBinEl for BinEl {
    fn try_from_bin_el(elem: &Self) -> Result<Self, CelesteMapError> {
        Ok(elem.clone())
    }

    fn to_binel(&self) -> BinEl {
        self.clone()
    }
}

pub trait GetAttrOrChild: Sized {
    fn attr_or_child<'b>(elem: &'b BinEl, key: &str) -> Option<&'b Self>;
    fn nested_attr_or_child<'b>(elem: &'b BinEl, key: &str) -> Option<&'b Self> {
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
    fn attr_or_child<'b>(elem: &'b BinEl, key: &str) -> Option<&'b Self> {
        let (x,) = elem.get(key).iter().collect_tuple()?;
        Some(x)
    }

    fn apply_attr_or_child(elem: &mut BinEl, key: &str, mut value: Self) {
        if value.name.is_empty() {
            value.name = key.to_owned();
        }
        *elem.get_mut(key) = vec![value];
    }
}

impl GetAttrOrChild for BinElAttr {
    fn attr_or_child<'b>(elem: &'b BinEl, key: &str) -> Option<&'b Self> {
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
    fn set_bin_el<'a>(elem: &mut BinEl, key: &'a str, value: &'a T) {
        GetAttrOrChild::nested_apply_attr_or_child(elem, key, Self::serialize(value));
    }
    fn set_bin_el_optional<'a>(elem: &mut BinEl, key: &'a str, value: &'a Option<T>) {
        if let Some(value) = value {
            Self::set_bin_el(elem, key, value);
        }
    }
    fn set_bin_el_default<'a>(elem: &mut BinEl, key: &'a str, value: &'a T)
    where
        T: Default + PartialEq,
    {
        if *value != T::default() {
            Self::set_bin_el(elem, key, value);
        }
    }
}

// entires in this list are not parsed by celeste.exe
static ATTRS_IGNORE: Lazy<HashSet<(&'static str, &'static str)>> = Lazy::new(|| {
    HashSet::from([
        ("meta", "Name"),
        ("meta", "SID"),
        ("meta", "CompleteScreenName"),
        ("meta", "FixRotateSpinnerAngles"),
        ("level", "altMusic"),
        ("bgdecals", "tileset"),
        ("fgdecals", "tileset"),
        ("bgtiles", "tileset"),
        ("fgtiles", "tileset"),
        ("objtiles", "tileset"),
    ])
});
// entries in this list are made default when missing by celeste.exe
static ATTRS_OPTIONAL: Lazy<HashSet<(&'static str, &'static str)>> = Lazy::new(|| {
    HashSet::from([
        ("level", "music"),
        ("level", "alt_music"),
        ("level", "ambience"),
        ("level", "musicProgress"),
        ("level", "ambienceProgress"),
        ("level", "delayAltMusicFade"),
        ("level", "musicLayer1"),
        ("level", "musicLayer2"),
        ("level", "musicLayer3"),
        ("level", "musicLayer4"),
        ("level", "cameraOffsetX"),
        ("level", "cameraOffsetY"),
        ("level", "dark"),
        ("level", "space"),
        ("level", "underwater"),
        ("level", "whisper"),
        ("level", "disableDownTransition"),
        ("level", "enforceDashNumber"),
        ("entities", "width"),
        ("entities", "height"),
        ("triggers", "width"),
        ("triggers", "height"),
        ("bgtiles", "exportMode"),
        ("bgtiles", "offsetX"),
        ("bgtiles", "offsetY"),
        ("fgtiles", "exportMode"),
        ("fgtiles", "offsetX"),
        ("fgtiles", "offsetY"),
        ("objtiles", "exportMode"),
        ("objtiles", "offsetX"),
        ("objtiles", "offsetY"),
        ("bg", "offsetX"),
        ("bg", "offsetY"),
        ("solids", "offsetX"),
        ("solids", "offsetY"),
        ("bgdecals", "offsetX"),
        ("bgdecals", "offsetY"),
        ("fgdecals", "offsetX"),
        ("fgdecals", "offsetY"),
        ("entities", "offsetX"),
        ("entities", "offsetY"),
        ("triggers", "offsetX"),
        ("triggers", "offsetY"),
        ("objtiles", "innerText"),
        ("fgtiles", "innerText"),
        ("bgtiles", "innerText"),
        ("Foregrounds", "tag"),
        ("Foregrounds", "x"),
        ("Foregrounds", "y"),
        ("Foregrounds", "scrollx"),
        ("Foregrounds", "scrolly"),
        ("Foregrounds", "speedx"),
        ("Foregrounds", "speedy"),
        ("Foregrounds", "color"),
        ("Foregrounds", "alpha"),
        ("Foregrounds", "flipx"),
        ("Foregrounds", "flipy"),
        ("Foregrounds", "loopx"),
        ("Foregrounds", "loopy"),
        ("Foregrounds", "wind"),
        ("Foregrounds", "instantIn"),
        ("Foregrounds", "instantOut"),
        ("Foregrounds", "fadex"),
        ("Foregrounds", "fadey"),
        ("Backgrounds", "tag"),
        ("Backgrounds", "x"),
        ("Backgrounds", "y"),
        ("Backgrounds", "scrollx"),
        ("Backgrounds", "scrolly"),
        ("Backgrounds", "speedx"),
        ("Backgrounds", "speedy"),
        ("Backgrounds", "color"),
        ("Backgrounds", "alpha"),
        ("Backgrounds", "flipx"),
        ("Backgrounds", "flipy"),
        ("Backgrounds", "loopx"),
        ("Backgrounds", "loopy"),
        ("Backgrounds", "wind"),
        ("Backgrounds", "instantIn"),
        ("Backgrounds", "instantOut"),
        ("Backgrounds", "fadex"),
        ("Backgrounds", "fadey"),
    ])
});

// entries in this list have arbitrary child names and should thusly be looked up by their
// parent name
static PARENT_OVERRIDES: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["entities", "triggers", "Backgrounds", "Foregrounds"]));

fn fuzzy_ignore(_one: Option<&BinElAttr>, _two: Option<&BinElAttr>) -> bool {
    true
}

fn get_attr_comparator(
    parent_name: &str,
    elem_name: &str,
    attr_name: &str,
) -> fn(Option<&BinElAttr>, Option<&BinElAttr>) -> bool {
    let elem_name = if PARENT_OVERRIDES.contains(parent_name) {
        parent_name
    } else {
        elem_name
    };
    if ["bg", "solids"].contains(&elem_name) && attr_name == "innerText" {
        return compare_char_tiles;
    }
    if ["bgtiles", "fgtiles", "objtiles"].contains(&elem_name) && attr_name == "innerText" {
        return compare_int_tiles;
    }
    if ["Foregrounds", "Backgrounds"].contains(&elem_name)
        && ["alpha", "scrollx", "scrolly"].contains(&attr_name)
    {
        return compare_default_one;
    }
    if ["Foregrounds", "Backgrounds"].contains(&elem_name)
        && ["loopx", "loopy", "instantIn"].contains(&attr_name)
    {
        return compare_default_true;
    }
    if ATTRS_IGNORE.contains(&(elem_name, attr_name)) {
        return fuzzy_ignore;
    }
    if ATTRS_OPTIONAL.contains(&(elem_name, attr_name)) {
        return bin_el_attr_fuzzy_equal_optional;
    }
    bin_el_attr_fuzzy_equal_required
}

fn get_elem_comparator(_parent_name: &str, _child_name: &str) -> fn(&str, &BinEl, &BinEl) -> bool {
    bin_el_fuzzy_equal
}

pub fn bin_el_fuzzy_equal(parent: &str, first: &BinEl, second: &BinEl) -> bool {
    if first.name != second.name {
        panic!("element name: {} != {}", first.name, second.name);
    }
    let attributes = first
        .attributes
        .keys()
        .chain(second.attributes.keys())
        .sorted()
        .dedup()
        .collect::<Vec<_>>();
    for key in attributes {
        let value1 = first.attributes.get(key);
        let value2 = second.attributes.get(key);
        let compare = get_attr_comparator(parent, &first.name, key);
        if !compare(value1, value2) {
            panic!("{}.{}: {:?} != {:?}", &first.name, key, value1, value2);
        }
    }
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
    let children = first_children
        .keys()
        .chain(second_children.keys())
        .sorted()
        .dedup()
        .collect::<Vec<_>>();

    for key in children {
        let key = key.as_str();
        if first.name == "Map" && ["audiostate", "mode"].contains(&key) {
            // cruor what the HELL are you doing
            continue;
        }
        if first.name == "level"
            && ["bg", "solids"].contains(&key)
            && [
                Some(&BinElAttr::Text("lvl_credits-resort".into())),
                Some(&BinElAttr::Text("lvl_14".into())),
            ]
            .contains(&first.attributes.get("name"))
        {
            // fuck you in particular
            continue;
        }
        if let (Some(list1), Some(list2)) = (first_children.get(key), second_children.get(key)) {
            for (one, &two) in list1.iter().zip_eq(list2) {
                let compare = get_elem_comparator(&first.name, key);
                compare(&first.name, one, two);
            }
        } else if first.name == "level"
            && ["objtiles", "fgtiles", "bgtiles", "bgdecals", "fgdecals"].contains(&key)
        {
            // older versions of ahorn do not serialize objtiles, fgtiles, bgtiles
            // older versions of vanilla editor do not serialize bgdecals
            let list2 = second_children.get(key).unwrap();
            assert_eq!(list2.len(), 1);
            let compare = get_elem_comparator(&first.name, key);
            let one = BinEl::new(key);
            let two = list2.get(0).unwrap();
            compare(&first.name, &one, two);
        } else {
            let (there, not) = if first_children.contains_key(key) {
                ("first", "second")
            } else {
                ("second", "first")
            };
            panic!(
                "Child of {} present in {} but not {}: {}",
                &first.name, there, not, key
            );
        }
    }
    true
}

fn compare_char_tiles(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let default;

    let (first, second) = match (first, second) {
        (None, Some(second)) => {
            default = make_like_default(second);
            (&default, second)
        }
        (Some(first), None) => {
            default = make_like_default(first);
            (first, &default)
        }
        (Some(first), Some(second)) => (first, second),
        (None, None) => unimplemented!(),
    };
    compare_char_tiles_required(first, second)
}

fn compare_char_tiles_required(first: &BinElAttr, second: &BinElAttr) -> bool {
    let (BinElAttr::Text(first), BinElAttr::Text(second)) = (first, second) else {
        return false
    };
    let mut pattern_i = 0;
    let second_chars = second.chars().collect::<Vec<_>>();
    for target_ch in first.chars().filter(|ch| ch != &'\r') {
        match second_chars.get(pattern_i) {
            Some(&pattern_ch) if pattern_ch == target_ch => pattern_i += 1,
            None | Some(&'\n') => continue,
            _ => return false,
        }
    }
    true
}

fn compare_int_tiles(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let default;
    let (new_first, new_second) = match (first, second) {
        (Some(first), Some(second)) => (first, second),
        (Some(first), None) => {
            default = make_like_default(first);
            (first, &default)
        }
        (None, Some(second)) => {
            default = make_like_default(second);
            (&default, second)
        }
        (None, None) => unreachable!(),
    };
    compare_int_tiles_required(new_first, new_second)
}

fn compare_int_tiles_required(first: &BinElAttr, second: &BinElAttr) -> bool {
    let (BinElAttr::Text(first), BinElAttr::Text(second)) = (first, second) else { return false };
    let mut pattern_i = 0;
    let second_chars = second.chars().collect::<Vec<_>>();
    for target_ch in first.chars().filter(|ch| ch != &'\r') {
        match second_chars.get(pattern_i) {
            Some(&pattern_ch) if pattern_ch == target_ch => pattern_i += 1,
            None | Some(&'\n') if target_ch == '\n' => {}
            _ => return false,
        }
    }
    true
}

fn bin_el_attr_fuzzy_equal_optional(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let default;
    let (new_first, new_second) = match (first, second) {
        (Some(first), Some(second)) => (first, second),
        (Some(first), None) => {
            default = make_like_default(first);
            (first, &default)
        }
        (None, Some(second)) => {
            default = make_like_default(second);
            (&default, second)
        }
        (None, None) => unreachable!(),
    };
    bin_el_attr_fuzzy_equal_required(Some(new_first), Some(new_second))
}

fn compare_default_one(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let one = BinElAttr::Int(1);
    let first = first.unwrap_or(&one);
    let second = second.unwrap_or(&one);
    bin_el_attr_fuzzy_equal_required(Some(first), Some(second))
}

fn compare_default_true(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let one = BinElAttr::Bool(true);
    let first = first.unwrap_or(&one);
    let second = second.unwrap_or(&one);
    bin_el_attr_fuzzy_equal_required(Some(first), Some(second))
}

fn make_like_default(like: &BinElAttr) -> BinElAttr {
    match like {
        BinElAttr::Bool(_) => BinElAttr::Bool(false),
        BinElAttr::Int(_) => BinElAttr::Int(0),
        BinElAttr::Float(_) => BinElAttr::Float(0.0),
        BinElAttr::Text(_) => BinElAttr::Text("".to_owned()),
    }
}

fn bin_el_attr_fuzzy_equal_required(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let (Some(first), Some(second)) = (first, second) else { return false };
    match (first, second) {
        (BinElAttr::Bool(_), BinElAttr::Int(_)) => todo!(),
        (BinElAttr::Bool(_), BinElAttr::Float(_)) => todo!(),
        (BinElAttr::Bool(_), BinElAttr::Text(_)) => todo!(),
        (BinElAttr::Int(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Int(i), BinElAttr::Float(f)) => *i as f32 == *f && *f as i32 == *i,
        (BinElAttr::Int(i), BinElAttr::Text(t)) => *t == i.to_string(),
        (BinElAttr::Float(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Float(f), BinElAttr::Int(i)) => *f as i32 == *i,
        (BinElAttr::Float(_), BinElAttr::Text(_)) => todo!(),
        (BinElAttr::Text(_), BinElAttr::Bool(_)) => todo!(),
        (BinElAttr::Text(t), BinElAttr::Int(i)) => *t == i.to_string(),
        (BinElAttr::Text(_), BinElAttr::Float(_)) => todo!(),
        _ => first == second,
    }
}
