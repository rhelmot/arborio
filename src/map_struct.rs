use celeste::binel::*;
use euclid::{Point2D, Size2D};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;
use vizia::Data;

use crate::units::*;
use crate::assets::next_uuid;

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct MapID {
    pub module: String,
    pub sid: String,
}

impl Data for MapID {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug)]
pub struct CelesteMap {
    pub id: MapID,
    pub dirty: bool,

    pub name: String,
    pub filler: Vec<MapRectStrict>,
    pub foregrounds: Vec<CelesteMapStyleground>,
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct CelesteMapEntity {
    pub id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub attributes: HashMap<String, BinElAttr>,
    pub nodes: Vec<(i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct CelesteMapDecal {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub texture: String,
}

#[derive(Debug)]
pub struct CelesteMapStyleground {
    pub name: String,
    pub texture: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub loop_x: Option<bool>,
    pub loop_y: Option<bool>,
    pub scroll_x: Option<f32>,
    pub scroll_y: Option<f32>,
    pub speed_x: Option<f32>,
    pub speed_y: Option<f32>,
    pub color: Option<String>,
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
    fn missing_child(parent: &str, child: &str) -> CelesteMapError {
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
            if self.tile(pt, true).unwrap_or('0') != '0' {
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

use crate::config::entity_expression::Const;
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
        if let Some((x, y)) = self.nodes.first() {
            env.insert("firstnodex", Const::from_num(*x));
            env.insert("firstnodey", Const::from_num(*y));
        }
        if let Some((x, y)) = self.nodes.last() {
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
        if let Some((x, y)) = self.nodes.get(node_idx) {
            env.insert("nodex", Const::from_num(*x));
            env.insert("nodey", Const::from_num(*y));
        }
        if let Some((x, y)) = self.nodes.get(node_idx + 1) {
            env.insert("nextnodex", Const::from_num(*x));
            env.insert("nextnodey", Const::from_num(*y));
            env.insert("nextnodexorbase", Const::from_num(*x));
            env.insert("nextnodeyorbase", Const::from_num(*y));
        } else {
            env.insert("nextnodexorbase", Const::from_num(self.x));
            env.insert("nextnodeyorbase", Const::from_num(self.y));
        }
        if let Some((x, y)) = self.nodes.get(node_idx.wrapping_sub(1)) {
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

    let filler = get_child(&binfile.root, "Filler")?;
    let levels = get_child(&binfile.root, "levels")?;
    let style = get_child(&binfile.root, "Style")?;
    let style_fg = get_child(style, "Foregrounds")?;
    let style_bg = get_child(style, "Backgrounds")?;

    let filler_parsed = filler
        .children()
        .map(parse_filler_rect)
        .collect::<Result<_, CelesteMapError>>()?;
    let style_fg_parsed = style_fg
        .children()
        .map(parse_styleground)
        .collect::<Result<_, CelesteMapError>>()?;
    let style_bg_parsed = style_bg
        .children()
        .map(parse_styleground)
        .collect::<Result<_, CelesteMapError>>()?;
    let levels_parsed = levels
        .children()
        .map(parse_level)
        .collect::<Result<_, CelesteMapError>>()?;

    Ok(CelesteMap {
        id,
        dirty: false,
        name: binfile.package,
        filler: filler_parsed,
        foregrounds: style_fg_parsed,
        backgrounds: style_bg_parsed,
        levels: levels_parsed,
    })
}

fn parse_level(elem: &BinEl) -> Result<CelesteMapLevel, CelesteMapError> {
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
        entities: get_child(elem, "entities")?
            .children()
            .map(parse_entity_trigger)
            .collect::<Result<_, CelesteMapError>>()?,
        triggers: get_child(elem, "triggers")?
            .children()
            .map(parse_entity_trigger)
            .collect::<Result<_, CelesteMapError>>()?,
        fg_decals: match fg_decals {
            Some(v) => v
                .children()
                .map(parse_decal)
                .collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },
        bg_decals: match bg_decals {
            Some(v) => v
                .children()
                .map(parse_decal)
                .collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },

        cache: default::Default::default(),
    })
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

fn parse_entity_trigger(elem: &BinEl) -> Result<CelesteMapEntity, CelesteMapError> {
    let basic_attrs: Vec<String> = vec![
        "id".to_string(),
        "x".to_string(),
        "y".to_string(),
        "width".to_string(),
        "height".to_string(),
    ];
    Ok(CelesteMapEntity {
        id: get_attr(elem, "id")?,
        name: elem.name.clone(),
        x: get_attr(elem, "x")?,
        y: get_attr(elem, "y")?,
        width: get_optional_attr(elem, "width")?.unwrap_or(0) as u32,
        height: get_optional_attr(elem, "height")?.unwrap_or(0) as u32,
        attributes: elem
            .attributes
            .iter()
            .map(|kv| (kv.0.clone(), kv.1.clone()))
            .filter(|kv| !basic_attrs.contains(kv.0.borrow()))
            .collect(),
        nodes: elem
            .children()
            .map(|child| -> Result<(i32, i32), CelesteMapError> {
                Ok((get_attr(child, "x")?, get_attr(child, "y")?))
            })
            .collect::<Result<_, CelesteMapError>>()?,
    })
}

fn parse_decal(elem: &BinEl) -> Result<CelesteMapDecal, CelesteMapError> {
    Ok(CelesteMapDecal {
        id: next_uuid(),
        x: get_attr(elem, "x")?,
        y: get_attr(elem, "y")?,
        scale_x: get_attr(elem, "scaleX")?,
        scale_y: get_attr(elem, "scaleY")?,
        texture: get_attr::<String>(elem, "texture")?.replace('\\', "/"),
    })
}

fn parse_filler_rect(elem: &BinEl) -> Result<MapRectStrict, CelesteMapError> {
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

fn parse_styleground(elem: &BinEl) -> Result<CelesteMapStyleground, CelesteMapError> {
    Ok(CelesteMapStyleground {
        name: elem.name.clone(),
        texture: get_optional_attr(elem, "texture")?,
        x: get_optional_attr(elem, "x")?,
        y: get_optional_attr(elem, "y")?,
        loop_x: get_optional_attr(elem, "loopx")?,
        loop_y: get_optional_attr(elem, "loopy")?,
        scroll_x: get_optional_attr(elem, "scrollx")?,
        scroll_y: get_optional_attr(elem, "scrolly")?,
        speed_x: get_optional_attr(elem, "speedx")?,
        speed_y: get_optional_attr(elem, "speedy")?,
        color: get_optional_attr(elem, "color")?,
        blend_mode: get_optional_attr(elem, "blendmode")?,
    })
}

fn get_optional_child<'a>(elem: &'a BinEl, name: &str) -> Option<&'a BinEl> {
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
}
