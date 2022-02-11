use celeste::binel::*;
use euclid::{Point2D, Size2D};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::default;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use vizia::Data;

use crate::assets::next_uuid;
use crate::from_binel::{
    bin_el_fuzzy_equal, get_nested_child, GetAttrOrChild, TryFromBinEl, TwoWayConverter,
};
use crate::units::*;

#[derive(Eq, PartialEq, Hash, Debug, Clone, Default)]
pub struct MapID {
    pub module: String,
    pub sid: String,
}

impl Data for MapID {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Clone, Debug, TryFromBinEl)]
#[convert_with(MapComponentConverter)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    #[name("w")]
    pub width: u32,
    #[name("h")]
    pub height: u32,
}

#[derive(Debug, TryFromBinEl)]
pub struct CelesteMap {
    #[bin_el_skip]
    pub id: MapID,
    #[bin_el_skip]
    pub dirty: bool,
    #[bin_el_skip]
    pub name: String,

    #[name("Filler")]
    pub filler: Vec<MapRectStrict>,
    #[name("Style/Foregrounds")]
    pub foregrounds: Vec<CelesteMapStyleground>,
    #[name("Style/Backgrounds")]
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
    #[name("meta")]
    #[optional]
    pub meta: Option<CelesteMapMeta>,
}

// this is a fucking mess.
#[derive(Debug, TryFromBinEl)]
pub struct CelesteMapMeta {
    #[name("OverrideASideMeta")]
    #[optional]
    pub override_aside_meta: Option<bool>,
    #[name("ColorGrade")]
    #[optional]
    pub color_grade: Option<String>,
    #[name("Dreaming")]
    #[optional]
    pub dreaming: Option<bool>,
    #[name("ForegroundTiles")]
    #[optional]
    pub fg_tiles: Option<String>,
    #[name("BackgroundTiles")]
    #[optional]
    pub bg_tiles: Option<String>,
    #[name("IntroType")]
    #[optional]
    pub intro_type: Option<String>, // TODO I think this is an enum
    #[name("TitleTextColor")]
    #[optional]
    pub title_text_color: Option<String>, // this too
    #[name("TitleBaseColor")]
    #[optional]
    pub title_base_color: Option<String>, // this too
    #[name("TitleAccentColor")]
    #[optional]
    pub title_accent_color: Option<String>, // this too
    #[name("Icon")]
    #[optional]
    pub icon: Option<String>,
    #[name("Interlude")]
    #[optional]
    pub interlude: Option<bool>,
    #[name("Wipe")]
    #[optional]
    pub wipe: Option<String>,
    #[name("BloomBase")]
    #[optional]
    pub bloom_base: Option<f32>,
    #[name("BloomStrength")]
    #[optional]
    pub bloom_strength: Option<f32>,
    // TODO more fields that ahorn doesn't let you change but everest will read from map.meta.yaml
    #[children]
    pub modes: Vec<CelesteMapMetaMode>, // [Option<_>; 3] perhaps?
}

#[derive(Debug, TryFromBinEl)]
#[name("mode")]
pub struct CelesteMapMetaMode {
    #[name("HeartIsEnd")]
    #[optional]
    pub heart_is_end: Option<bool>,
    #[name("Inventory")]
    #[optional]
    pub inventory: Option<String>,
    #[name("StartLevel")]
    #[optional]
    pub start_level: Option<String>,
    #[name("SeekerSlowdown")]
    #[optional]
    pub seeker_slowdown: Option<bool>,
    #[name("TheoInBubble")]
    #[optional]
    pub theo_in_bubble: Option<bool>,
    #[name("IgnoreLevelAudioLayerData")]
    #[optional]
    pub ignore_level_audio_layer_data: Option<bool>,
    #[name("audiostate")]
    #[optional]
    pub audio_state: Option<CelesteMapMetaAudioState>,
}

#[derive(Debug, TryFromBinEl)]
pub struct CelesteMapMetaAudioState {
    #[name("Ambience")]
    pub ambience: String,
    #[name("Music")]
    pub music: String,
}

#[derive(Debug)]
pub struct CelesteMapLevel {
    pub name: String,
    pub bounds: MapRectStrict,
    pub color: i32,
    pub camera_offset_x: f32,
    pub camera_offset_y: f32,
    pub wind_pattern: String,
    pub space: bool,
    pub underwater: bool,
    pub whisper: bool,
    pub dark: bool,
    pub disable_down_transition: bool,

    pub music: String,
    pub alt_music: String,
    pub ambience: String,
    pub music_layers: [bool; 6],
    pub music_progress: String,
    pub ambience_progress: String,

    pub object_tiles: Vec<i32>,
    pub fg_decals: Vec<CelesteMapDecal>,
    pub bg_decals: Vec<CelesteMapDecal>,
    pub fg_tiles: Vec<char>,
    pub bg_tiles: Vec<char>,
    pub entities: Vec<CelesteMapEntity>,
    pub triggers: Vec<CelesteMapEntity>,

    pub cache: RefCell<CelesteMapLevelCache>,
}

#[derive(Default)]
pub struct CelesteMapLevelCache {
    pub render_cache_valid: bool,
    pub render_cache: Option<femtovg::ImageId>,
    pub last_entity_idx: usize,
    pub last_decal_idx: usize,
}

#[derive(Debug, Clone, PartialEq, TryFromBinEl)]
pub struct CelesteMapEntity {
    pub id: i32,
    #[name]
    pub name: String,
    pub x: i32,
    pub y: i32,
    #[default]
    pub width: u32,
    #[default]
    pub height: u32,
    #[attributes]
    pub attributes: HashMap<String, BinElAttr>,
    #[children]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, TryFromBinEl)]
#[name("node")]
pub struct Node {
    pub x: i32,
    pub y: i32,
}
impl From<(i32, i32)> for Node {
    fn from((x, y): (i32, i32)) -> Self {
        Node { x, y }
    }
}

#[derive(Debug, Clone, TryFromBinEl)]
#[name("decal")]
pub struct CelesteMapDecal {
    #[generate(next_uuid())]
    pub id: u32,
    pub x: i32,
    pub y: i32,
    #[name("scaleX")]
    pub scale_x: f32,
    #[name("scaleY")]
    pub scale_y: f32,
    #[convert_with(ParenFlipper)]
    pub texture: String,
}

#[derive(Debug, TryFromBinEl)]
pub struct CelesteMapStyleground {
    #[optional]
    pub alpha: Option<f32>,
    #[name]
    pub name: String,
    #[optional]
    pub texture: Option<String>,
    #[optional]
    pub x: Option<i32>,
    #[optional]
    pub y: Option<i32>,
    #[optional]
    #[name("loopx")]
    pub loop_x: Option<bool>,
    #[optional]
    #[name("loopy")]
    pub loop_y: Option<bool>,
    #[optional]
    #[name("scrollx")]
    pub scroll_x: Option<f32>,
    #[optional]
    #[name("scrolly")]
    pub scroll_y: Option<f32>,
    #[optional]
    #[name("speedx")]
    pub speed_x: Option<f32>,
    #[optional]
    #[name("speedy")]
    pub speed_y: Option<f32>,
    #[optional]
    pub color: Option<String>,
    #[optional]
    #[name("blendmode")]
    pub blend_mode: Option<String>,
}

#[derive(Debug)]
pub struct CelesteMapError {
    pub kind: CelesteMapErrorType,
    pub description: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CelesteMapErrorType {
    ParseError,
    MissingChild,
    MissingAttribute,
    BadAttrType,
    OutOfRange,
}

impl vizia::Data for CelesteMapEntity {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Debug for CelesteMapLevelCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CelesteMapLevelCache")
            .field("render_cache_valid", &self.render_cache_valid)
            .field("last_entity_idx", &self.last_entity_idx)
            .finish()
    }
}

impl fmt::Display for CelesteMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.description)
    }
}

impl Error for CelesteMapError {
    fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl CelesteMapError {
    pub(crate) fn missing_child(parent: &str, child: &str) -> CelesteMapError {
        CelesteMapError {
            kind: CelesteMapErrorType::MissingChild,
            description: format!("Expected child of {}: {} not found", parent, child),
        }
    }

    fn missing_attribute(parent: &str, attr: &str) -> CelesteMapError {
        CelesteMapError {
            kind: CelesteMapErrorType::MissingAttribute,
            description: format!("Expected attribute of {}: {} not found", parent, attr),
        }
    }
}

struct ParenFlipper;
impl TwoWayConverter<String> for ParenFlipper {
    type BinType = BinElAttr;

    fn try_parse(elem: &Self::BinType) -> Result<String, CelesteMapError> {
        DefaultConverter::try_parse(elem).map(|s: String| {
            //assert!(!s.contains('/'));  // this fails on custom maps
            s.replace('\\', "/")
        })
    }

    fn serialize(val: &String) -> Self::BinType {
        assert!(!val.contains('\\'));
        BinElAttr::Text(val.replace('/', "\\"))
    }
}

impl CelesteMapLevel {
    pub fn tile(&self, pt: TilePoint, foreground: bool) -> Option<char> {
        let w = self.bounds.width() as i32 / 8;
        if pt.x < 0 || pt.x >= w {
            return None;
        }
        let tiles = if foreground {
            &self.fg_tiles
        } else {
            &self.bg_tiles
        };

        tiles.get((pt.x + pt.y * w) as usize).copied()
    }

    pub fn tile_mut(&mut self, pt: TilePoint, foreground: bool) -> Option<&mut char> {
        let w = self.bounds.width() as i32 / 8;
        if pt.x < 0 || pt.x >= w {
            return None;
        }
        let tiles = if foreground {
            &mut self.fg_tiles
        } else {
            &mut self.bg_tiles
        };

        tiles.get_mut((pt.x + pt.y * w) as usize)
    }

    pub fn entity(&self, id: i32, trigger: bool) -> Option<&CelesteMapEntity> {
        let entities = if trigger {
            &self.triggers
        } else {
            &self.entities
        };
        if let Some(e) = entities.get(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                return Some(e);
            }
        }
        for (idx, e) in entities.iter().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_entity_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn entity_mut(&mut self, id: i32, trigger: bool) -> Option<&mut CelesteMapEntity> {
        let entities = if trigger {
            &mut self.triggers
        } else {
            &mut self.entities
        };
        if let Some(e) = entities.get_mut(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                // hack around borrow checker
                let entities = if trigger {
                    &mut self.triggers
                } else {
                    &mut self.entities
                };
                return entities.get_mut(self.cache.borrow().last_entity_idx);
            }
        }
        let entities = if trigger {
            &mut self.triggers
        } else {
            &mut self.entities
        };
        for (idx, e) in entities.iter_mut().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_entity_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn decal(&self, id: u32, fg: bool) -> Option<&CelesteMapDecal> {
        let decals = if fg { &self.fg_decals } else { &self.bg_decals };
        if let Some(e) = decals.get(self.cache.borrow().last_entity_idx) {
            if e.id == id {
                return Some(e);
            }
        }
        for (idx, e) in decals.iter().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_decal_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn decal_mut(&mut self, id: u32, fg: bool) -> Option<&mut CelesteMapDecal> {
        let decals = if fg {
            &mut self.fg_decals
        } else {
            &mut self.bg_decals
        };
        if let Some(e) = decals.get_mut(self.cache.borrow().last_decal_idx) {
            if e.id == id {
                // hack around borrow checker
                let decals = if fg {
                    &mut self.fg_decals
                } else {
                    &mut self.bg_decals
                };
                return decals.get_mut(self.cache.borrow().last_decal_idx);
            }
        }
        let decals = if fg {
            &mut self.fg_decals
        } else {
            &mut self.bg_decals
        };
        for (idx, e) in decals.iter_mut().enumerate() {
            if e.id == id {
                self.cache.borrow_mut().last_decal_idx = idx;
                return Some(e);
            }
        }
        None
    }

    pub fn cache_entity_idx(&self, idx: usize) {
        self.cache.borrow_mut().last_entity_idx = idx;
    }

    pub fn cache_decal_idx(&self, idx: usize) {
        self.cache.borrow_mut().last_decal_idx = idx;
    }

    pub fn next_id(&self) -> i32 {
        let mut highest_entity = -1;
        for e in &self.entities {
            highest_entity = highest_entity.max(e.id);
        }
        for e in &self.triggers {
            highest_entity = highest_entity.max(e.id);
        }
        highest_entity + 1
    }

    pub fn occupancy_field(&self) -> TileGrid<FieldEntry> {
        let mut result = TileGrid::new_default(size_room_to_tile(&self.bounds.size.cast_unit()));
        for entity in &self.entities {
            if entity.width != 0 && entity.height != 0 {
                let rect = TileRect::new(
                    TilePoint::new(entity.x / 8, entity.y / 8),
                    TileSize::new(entity.width as i32 / 8, entity.height as i32 / 8),
                );
                for pt in rect_point_iter(rect, 1) {
                    if let Some(spot) = result.get_mut(pt) {
                        *spot = FieldEntry::Entity(entity);
                    }
                }
            }
        }
        for pt in rect_point_iter(rect_room_to_tile(&self.room_bounds()), 1) {
            if let Some('0') | None = self.tile(pt, true) {
                *result.get_mut(pt).unwrap() = FieldEntry::Fg;
            }
        }
        result
    }

    pub fn room_bounds(&self) -> RoomRect {
        RoomRect::new(RoomPoint::zero(), self.bounds.size.cast_unit())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FieldEntry<'a> {
    None,
    Fg,
    Entity(&'a CelesteMapEntity),
}

impl Default for FieldEntry<'_> {
    fn default() -> Self {
        FieldEntry::None
    }
}

impl CelesteMap {
    pub fn level_at(&self, pt: MapPointStrict) -> Option<usize> {
        for (idx, room) in self.levels.iter().enumerate() {
            if room.bounds.contains(pt) {
                return Some(idx);
            }
        }
        None
    }
}

use crate::celeste_mod::entity_expression::Const;
impl CelesteMapEntity {
    pub fn make_env(&self) -> HashMap<&str, Const> {
        let mut env: HashMap<&str, Const> = HashMap::new();
        env.insert("x", Const::from_num(self.x));
        env.insert("y", Const::from_num(self.y));
        env.insert("width", Const::from_num(self.width));
        env.insert("height", Const::from_num(self.height));
        for (key, val) in &self.attributes {
            env.insert(key.as_str(), Const::from_attr(val));
        }
        if let Some(Node { x, y }) = self.nodes.first() {
            env.insert("firstnodex", Const::from_num(*x));
            env.insert("firstnodey", Const::from_num(*y));
        }
        if let Some(Node { x, y }) = self.nodes.last() {
            env.insert("lastnodex", Const::from_num(*x));
            env.insert("lastnodey", Const::from_num(*y));
        }

        env
    }

    pub fn make_node_env<'a>(
        &self,
        mut env: HashMap<&'a str, Const>,
        node_idx: usize,
    ) -> HashMap<&'a str, Const> {
        env.insert("nodeidx", Const::from_num(node_idx as f64));
        if let Some(Node { x, y }) = self.nodes.get(node_idx) {
            env.insert("nodex", Const::from_num(*x));
            env.insert("nodey", Const::from_num(*y));
        }
        if let Some(Node { x, y }) = self.nodes.get(node_idx + 1) {
            env.insert("nextnodex", Const::from_num(*x));
            env.insert("nextnodey", Const::from_num(*y));
            env.insert("nextnodexorbase", Const::from_num(*x));
            env.insert("nextnodeyorbase", Const::from_num(*y));
        } else {
            env.insert("nextnodexorbase", Const::from_num(self.x));
            env.insert("nextnodeyorbase", Const::from_num(self.y));
        }
        if let Some(Node { x, y }) = self.nodes.get(node_idx.wrapping_sub(1)) {
            env.insert("prevnodex", Const::from_num(*x));
            env.insert("prevnodey", Const::from_num(*y));
            env.insert("prevnodexorbase", Const::from_num(*x));
            env.insert("prevnodeyorbase", Const::from_num(*y));
        } else {
            env.insert("prevnodexorbase", Const::from_num(self.x));
            env.insert("prevnodeyorbase", Const::from_num(self.y));
        }

        env
    }
}

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

pub fn from_binfile(id: MapID, binfile: BinFile) -> Result<CelesteMap, CelesteMapError> {
    expect_elem!(binfile.root, "Map");

    let mut result = CelesteMap::try_from_bin_el(&binfile.root);
    if let Ok(ref mut r) = result {
        r.id = id;
    }
    result
}

impl TryFromBinEl for CelesteMapLevel {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
        expect_elem!(elem, "level");

        let x = get_attr(elem, "x")?;
        let y = get_attr(elem, "y")?;
        let width = get_attr(elem, "width")?;
        let height = get_attr(elem, "height")?;
        let object_tiles = get_optional_child(elem, "fgtiles");
        let fg_decals = get_optional_child(elem, "fgdecals");
        let bg_decals = get_optional_child(elem, "bgdecals");

        Ok(CelesteMapLevel {
            bounds: MapRectStrict {
                origin: Point2D::new(x, y),
                size: Size2D::new(width, height),
            },
            name: get_attr(elem, "name")?,
            color: get_optional_attr(elem, "c")?.unwrap_or_default(),
            camera_offset_x: get_optional_attr(elem, "cameraOffsetX")?.unwrap_or_default(),
            camera_offset_y: get_optional_attr(elem, "cameraOffsetY")?.unwrap_or_default(),
            wind_pattern: get_optional_attr(elem, "windPattern")?.unwrap_or_default(),
            space: get_optional_attr(elem, "space")?.unwrap_or_default(),
            underwater: get_optional_attr(elem, "underwater")?.unwrap_or_default(),
            whisper: get_optional_attr(elem, "whisper")?.unwrap_or_default(),
            dark: get_optional_attr(elem, "dark")?.unwrap_or_default(),
            disable_down_transition: get_optional_attr(elem, "disableDownTransition")?
                .unwrap_or_default(),

            music: get_optional_attr(elem, "music")?.unwrap_or_default(),
            alt_music: get_optional_attr(elem, "alt_music")?.unwrap_or_default(),
            ambience: get_optional_attr(elem, "ambience")?.unwrap_or_default(),
            music_layers: [
                get_optional_attr(elem, "musicLayer1")?.unwrap_or_default(),
                get_optional_attr(elem, "musicLayer2")?.unwrap_or_default(),
                get_optional_attr(elem, "musicLayer3")?.unwrap_or_default(),
                get_optional_attr(elem, "musicLayer4")?.unwrap_or_default(),
                get_optional_attr(elem, "musicLayer5")?.unwrap_or_default(),
                get_optional_attr(elem, "musicLayer6")?.unwrap_or_default(),
            ],
            music_progress: get_optional_attr(elem, "musicProgress")?.unwrap_or_default(),
            ambience_progress: get_optional_attr(elem, "ambienceProgress")?.unwrap_or_default(),

            fg_tiles: parse_fgbg_tiles(get_child(elem, "solids")?, width / 8, height / 8)?,
            bg_tiles: parse_fgbg_tiles(get_child(elem, "bg")?, width / 8, height / 8)?,
            object_tiles: match object_tiles {
                Some(v) => parse_object_tiles(v, width, height),
                None => Ok(vec![-1; (width / 8 * height / 8) as usize]),
            }?,
            entities: DefaultConverter::from_bin_el(elem, "entities")?,
            triggers: TryFromBinEl::try_from_bin_el(get_child(elem, "triggers")?)?,
            fg_decals: fg_decals.map_or(Ok(Vec::new()), TryFromBinEl::try_from_bin_el)?,
            bg_decals: bg_decals.map_or(Ok(Vec::new()), TryFromBinEl::try_from_bin_el)?,

            cache: default::Default::default(),
        })
    }

    fn to_binel(&self) -> BinEl {
        todo!()
    }
}

fn parse_fgbg_tiles(elem: &BinEl, width: i32, height: i32) -> Result<Vec<char>, CelesteMapError> {
    let offset_x: i32 = get_optional_attr(elem, "offsetX")?.unwrap_or_default();
    let offset_y: i32 = get_optional_attr(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError {
        kind: CelesteMapErrorType::OutOfRange,
        description: format!("{} contains out-of-range data", elem.name),
    });
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }

    let mut data: Vec<char> = vec!['0'; (width * height) as usize];
    let mut x = offset_x;
    let mut y = offset_y;
    for ch in get_optional_attr::<String>(elem, "innerText")?
        .unwrap_or_default()
        .chars()
    {
        if ch == '\n' {
            x = offset_x;
            y += 1;
        } else if ch == '\r' {
        } else if x >= width || y >= height {
            // TODO remove this
            println!("{:?}", elem);
        } else {
            data[(x + y * width) as usize] = ch;
            x += 1;
        }
    }

    Ok(data)
}

fn parse_object_tiles(elem: &BinEl, width: i32, height: i32) -> Result<Vec<i32>, CelesteMapError> {
    let offset_x: i32 = get_optional_attr(elem, "offsetX")?.unwrap_or_default();
    let offset_y: i32 = get_optional_attr(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError {
        kind: CelesteMapErrorType::OutOfRange,
        description: format!("{} contains out-of-range data", elem.name),
    });
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }

    let mut data: Vec<i32> = vec![-1; (width * height) as usize];
    let mut y = offset_y;

    for line in get_optional_attr::<String>(elem, "innerText")?
        .unwrap_or_default()
        .split('\n')
    {
        let mut x = offset_x;
        for num in line.split(',') {
            if num.is_empty() {
                continue;
            }
            let ch: i32 = match num.parse() {
                Err(_) => {
                    return Err(CelesteMapError {
                        kind: CelesteMapErrorType::ParseError,
                        description: format!("Could not parse {} as int", num),
                    })
                }
                Ok(v) => v,
            };

            if x >= width || y >= height {
                return exc;
            } else {
                data[(x + y * width) as usize] = ch;
                x += 1;
            }
        }
        y += 1;
    }

    Ok(data)
}

struct DefaultConverter;
impl<T: TryFromBinEl> TwoWayConverter<T> for DefaultConverter {
    type BinType = BinEl;

    fn try_parse(elem: &Self::BinType) -> Result<T, CelesteMapError> {
        TryFromBinEl::try_from_bin_el(elem)
    }

    fn serialize(val: &T) -> Self::BinType {
        val.to_binel()
    }
}
macro_rules! attr_converter_impl {
    ($($types:ty),+) => {

        $(impl TwoWayConverter<$types> for DefaultConverter {
            type BinType = BinElAttr;

            fn try_parse(elem: &Self::BinType) -> Result<$types, CelesteMapError> {
                AttrCoercion::try_coerce(elem).ok_or_else(||
                    CelesteMapError {
                        kind: CelesteMapErrorType::BadAttrType,
                        description: format!("Expected {nice}, found {:?}", elem, nice = <$types>::NICE_NAME),
                    }
                )
            }

            fn serialize(val: &$types) -> Self::BinType {
                val.serialize()
            }
        })+
    }
}
attr_converter_impl!(i32, u32, String, f32, bool);

impl TryFromBinEl for MapRectStrict {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
        expect_elem!(elem, "rect");

        let x: i32 = get_attr(elem, "x")?;
        let y: i32 = get_attr(elem, "y")?;
        let w: i32 = get_attr(elem, "w")?;
        let h: i32 = get_attr(elem, "h")?;

        Ok(MapRectStrict {
            origin: Point2D::new(x * 8, y * 8),
            size: Size2D::new(w * 8, h * 8),
        })
    }

    fn to_binel(&self) -> BinEl {
        todo!()
    }
}

struct MapComponentConverter;
impl<T: Copy + TryFrom<i32> + TryInto<i32>> TwoWayConverter<T> for MapComponentConverter {
    type BinType = BinElAttr;

    fn try_parse(elem: &Self::BinType) -> Result<T, CelesteMapError> {
        if let Some(i) = i32::try_coerce(elem) {
            Ok((i * 8).try_into().ok().unwrap())
        } else {
            Err(CelesteMapError {
                kind: CelesteMapErrorType::BadAttrType,
                description: format!("Expected {nice}, found {:?}", elem, nice = i32::NICE_NAME),
            })
        }
    }

    fn serialize(val: &T) -> Self::BinType {
        BinElAttr::Int((*val).try_into().ok().unwrap() / 8)
    }
}

// impl TryFromBinEl<CelesteMapError> for Rect {
//     fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
//         expect_elem!(elem, "rect");
//
//         let x = MapComponentConverter::from_bin_el(elem, "x")?;
//         let y = MapComponentConverter::from_bin_el(elem, "y")?;
//         let width = MapComponentConverter::from_bin_el(elem, "w")?;
//         let height = MapComponentConverter::from_bin_el(elem, "h")?;
//
//         Ok(Rect {
//             x,
//             y,
//             width,
//             height,
//         })
//     }
// }

pub fn get_optional_child<'a>(elem: &'a BinEl, name: &str) -> Option<&'a BinEl> {
    let children_of_name = elem.get(name);
    if let [ref child] = children_of_name.as_slice() {
        // if there is exactly one child
        Some(child)
    } else {
        None
    }
}

fn get_child<'a>(elem: &'a BinEl, name: &str) -> Result<&'a BinEl, CelesteMapError> {
    get_optional_child(elem, name).ok_or_else(|| CelesteMapError::missing_child(&elem.name, name))
}

pub fn get_child_mut<'a>(elem: &'a mut BinEl, name: &str) -> &'a mut BinEl {
    let children = elem.get_mut(name);
    if children.is_empty() {
        children.push(BinEl::new(name));
    }
    match children.as_mut_slice() {
        [ref mut child] => child,
        [] => {
            unreachable!()
        }
        _ => todo!(),
    }
}

fn get_optional_attr<T>(elem: &BinEl, name: &str) -> Result<Option<T>, CelesteMapError>
where
    T: AttrCoercion,
{
    if let Some(attr) = elem.attributes.get(name) {
        if let Some(coerced) = T::try_coerce(attr) {
            Ok(Some(coerced))
        } else {
            Err(CelesteMapError {
                kind: CelesteMapErrorType::BadAttrType,
                description: format!("Expected {nice}, found {:?}", attr, nice = T::NICE_NAME),
            })
        }
    } else {
        Ok(None)
    }
}

fn get_attr<T>(elem: &BinEl, name: &str) -> Result<T, CelesteMapError>
where
    T: AttrCoercion,
{
    get_optional_attr(elem, name)?
        .ok_or_else(|| CelesteMapError::missing_attribute(&elem.name, name))
}

// Trait for types that a BinElAttr can possibly be coerced to, with the logic to do the coercion
trait AttrCoercion: Sized {
    // Type name to print out when giving BadAttrType errors
    const NICE_NAME: &'static str;
    fn try_coerce(attr: &BinElAttr) -> Option<Self>;
    fn serialize(&self) -> BinElAttr;
}

impl AttrCoercion for i32 {
    const NICE_NAME: &'static str = "integer";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Int(i) => Some(i),
            BinElAttr::Float(f) => Some(f as i32),
            _ => None,
        }
    }
    fn serialize(&self) -> BinElAttr {
        BinElAttr::Int(*self)
    }
}
impl AttrCoercion for u32 {
    const NICE_NAME: &'static str = "integer";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Int(i) => i.try_into().ok(),
            BinElAttr::Float(f) => Some(f as u32),
            _ => None,
        }
    }

    fn serialize(&self) -> BinElAttr {
        BinElAttr::Int(*self as i32)
    }
}
impl AttrCoercion for bool {
    const NICE_NAME: &'static str = "bool";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        if let BinElAttr::Bool(value) = *attr {
            Some(value)
        } else {
            None
        }
    }

    fn serialize(&self) -> BinElAttr {
        BinElAttr::Bool(*self)
    }
}
impl AttrCoercion for f32 {
    const NICE_NAME: &'static str = "float";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Float(f) => Some(f),
            BinElAttr::Int(i) => Some(i as f32),
            _ => None,
        }
    }

    fn serialize(&self) -> BinElAttr {
        BinElAttr::Float(*self)
    }
}
impl AttrCoercion for String {
    const NICE_NAME: &'static str = "text";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Text(ref s) => Some(s.clone()),
            BinElAttr::Int(i) => Some(i.to_string()),
            _ => None,
        }
    }

    fn serialize(&self) -> BinElAttr {
        BinElAttr::Text(self.clone())
    }
}
