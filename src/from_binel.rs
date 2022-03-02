#![allow(unused)]

use std::collections::{HashMap, HashSet};

use crate::map_struct::{get_child_mut, get_optional_child, CelesteMapError};
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

impl TryFromBinEl for BinEl {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
        Ok(elem.clone())
    }

    fn to_binel(&self) -> BinEl {
        self.clone()
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

    fn apply_attr_or_child(elem: &mut BinEl, key: &str, mut value: Self) {
        if value.name.is_empty() {
            value.name = key.to_owned();
        }
        *elem.get_mut(key) = vec![value];
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
    fn set_bin_el(elem: &mut BinEl, key: &str, value: &T) {
        GetAttrOrChild::nested_apply_attr_or_child(elem, key, Self::serialize(value));
    }
    fn set_bin_el_optional(elem: &mut BinEl, key: &str, value: &Option<T>) {
        if let Some(value) = value {
            Self::set_bin_el(elem, key, value);
        }
    }
    fn set_bin_el_default(elem: &mut BinEl, key: &str, value: &T)
    where
        T: Default + PartialEq,
    {
        if *value != T::default() {
            Self::set_bin_el(elem, key, value);
        }
    }
}

lazy_static::lazy_static! {
    // entires in this list are not parsed by celeste.exe
    static ref ATTRS_IGNORE: HashSet<(&'static str, &'static str)> = HashSet::from([
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
    ]);

    // entries in this list are made default when missing by celeste.exe
    static ref ATTRS_OPTIONAL: HashSet<(&'static str, &'static str)> = HashSet::from([
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
    ]);

    // entries in this list have arbitrary child names and should thusly be looked up by their
    // parent name
    static ref PARENT_OVERRIDES: HashSet<&'static str> = HashSet::from([
        "entities",
        "triggers",
        "Backgrounds",
        "Foregrounds",
    ]);
}

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
    if ["Foregrounds", "Backgrounds"].contains(&elem_name) && attr_name == "alpha" {
        return compare_alpha;
    }
    if ATTRS_IGNORE.contains(&(elem_name, attr_name)) {
        return fuzzy_ignore;
    }
    if ATTRS_OPTIONAL.contains(&(elem_name, attr_name)) {
        return bin_el_attr_fuzzy_equal_optional;
    }
    bin_el_attr_fuzzy_equal_required
}

fn get_elem_comparator(parent_name: &str, child_name: &str) -> fn(&str, &BinEl, &BinEl) -> bool {
    bin_el_fuzzy_equal
}

pub(crate) fn bin_el_fuzzy_equal(parent: &str, first: &BinEl, second: &BinEl) -> bool {
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
        if first.name == "Map" && ["audiostate", "mode"].contains(&key.as_str()) {
            // cruor what the HELL are you doing
            continue;
        }
        if first.name == "level"
            && ["bg", "solids"].contains(&key.as_ref())
            && [
                Some(&BinElAttr::Text("lvl_credits-resort".to_string())),
                Some(&BinElAttr::Text("lvl_14".to_string())),
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
            && ["objtiles", "fgtiles", "bgtiles", "bgdecals", "fgdecals"].contains(&key.as_str())
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

#[allow(clippy::unnecessary_unwrap)]
fn compare_char_tiles(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    if first.is_none() {
        let first = make_like_default(second.unwrap());
        compare_char_tiles_required(Some(&first), second)
    } else if second.is_none() {
        let second = make_like_default(first.unwrap());
        compare_char_tiles_required(first, Some(&second))
    } else {
        compare_char_tiles_required(first, second)
    }
}

fn compare_char_tiles_required(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    match (first, second) {
        (Some(BinElAttr::Text(first)), Some(BinElAttr::Text(second))) => {
            let mut pattern_i = 0;
            let second_chars = second.chars().collect::<Vec<_>>();
            for (target_i, target_ch) in first.chars().enumerate() {
                if target_ch == '\r' {
                    continue;
                }
                let pattern_ch = second_chars.get(pattern_i);
                if Some(target_ch) == pattern_ch.copied() {
                    pattern_i += 1;
                } else if (pattern_ch == None || pattern_ch.copied() == Some('\n'))
                    && (target_ch == '0' || target_ch == '\n')
                {
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

#[allow(clippy::unnecessary_unwrap)]
fn compare_int_tiles(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    if first.is_none() {
        let first = make_like_default(second.unwrap());
        compare_int_tiles_required(Some(&first), second)
    } else if second.is_none() {
        let second = make_like_default(first.unwrap());
        compare_int_tiles_required(first, Some(&second))
    } else {
        compare_int_tiles_required(first, second)
    }
}

fn compare_int_tiles_required(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    match (first, second) {
        (Some(BinElAttr::Text(first)), Some(BinElAttr::Text(second))) => {
            let mut pattern_i = 0;
            let second_chars = second.chars().collect::<Vec<_>>();
            for (target_i, target_ch) in first.chars().enumerate() {
                if target_ch == '\r' {
                    continue;
                }
                let pattern_ch = second_chars.get(pattern_i);
                if Some(target_ch) == pattern_ch.copied() {
                    pattern_i += 1;
                } else if (pattern_ch == None || pattern_ch.copied() == Some('\n'))
                    && target_ch == '\n'
                {
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

#[allow(clippy::unnecessary_unwrap)]
fn bin_el_attr_fuzzy_equal_optional(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    if first.is_none() {
        let first = make_like_default(second.unwrap());
        bin_el_attr_fuzzy_equal_required(Some(&first), second)
    } else if second.is_none() {
        let second = make_like_default(first.unwrap());
        bin_el_attr_fuzzy_equal_required(first, Some(&second))
    } else {
        bin_el_attr_fuzzy_equal_required(first, second)
    }
}

fn compare_alpha(first: Option<&BinElAttr>, second: Option<&BinElAttr>) -> bool {
    let one = BinElAttr::Int(1);
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
    if let (Some(first), Some(second)) = (first, second) {
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
    } else {
        false
    }
}

#[cfg(test)]
mod test {
    use crate::app_state::AppConfig;
    use crate::assets::InternedMap;
    use crate::celeste_mod::discovery;
    use crate::celeste_mod::walker::{ConfigSourceTrait, FolderSource};
    use crate::from_binel::{bin_el_fuzzy_equal, TryFromBinEl};
    use crate::map_struct::from_reader;
    use crate::{CelesteMap, MapID};
    use celeste::binel::BinEl;
    use std::ffi::OsStr;
    use std::path::Path;

    #[test]
    fn test_saving_all_mods() {
        let mut cfg: AppConfig = confy::load("arborio").unwrap_or_default();
        if let Some(root) = &cfg.celeste_root {
            assert!(root.is_dir(), "Arborio is misconfigured");
            let mut config = FolderSource::new(&root.join("Content")).unwrap();
            for path in config.list_all_files(Path::new("Maps")) {
                println!("testing Celeste {:?}", path);

                let mut reader = config.get_file(&path).unwrap();
                let mut file = vec![];
                reader.read_to_end(&mut file).unwrap();
                let (_, binfile) = celeste::binel::parser::take_file(file.as_slice()).unwrap();

                test_saving_one_mod(&binfile.root);
            }
            discovery::for_each_mod(root, |_, _, name, mut config| {
                for path in config.list_all_files(Path::new("Maps")) {
                    if path.extension() == Some(OsStr::new("bin")) {
                        if path == Path::new("Maps/SpringCollab2020/4-Expert/Mun.bin") {
                            println!("skipping whatever the fuck this is");
                            continue;
                        }
                        println!("testing {} {:?}", name, path);

                        let mut reader = config.get_file(&path).unwrap();
                        let mut file = vec![];
                        reader.read_to_end(&mut file).unwrap();
                        let (_, binfile) =
                            celeste::binel::parser::take_file(file.as_slice()).unwrap();

                        test_saving_one_mod(&binfile.root);
                    }
                }
            });
        } else {
            println!("TODO: bundle celeste skeleton for tests")
        }
    }

    fn test_saving_one_mod(bin: &BinEl) {
        let structured = CelesteMap::try_from_bin_el(bin).unwrap();
        let saved = structured.to_binel();
        assert!(bin_el_fuzzy_equal("", bin, &saved));
    }
}
