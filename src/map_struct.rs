#![allow(unused_parens)] // TODO: ???

use celeste::binel::*;
use euclid::{Point2D, Size2D};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::mem::swap;
use std::str::FromStr;
use vizia::{vg, Data, Lens};

use crate::assets::{next_uuid, Interned};
use crate::from_binel::{GetAttrOrChild, TryFromBinEl, TwoWayConverter};
use crate::units::*;

#[derive(Eq, PartialEq, Hash, Debug, Clone, Default)]
pub struct MapPath {
    pub module: Interned,
    pub sid: String,
}

impl Data for MapPath {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

uuid_cls!(MapID);

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

#[derive(Debug, TryFromBinEl, Lens)]
#[name("Map")]
pub struct CelesteMap {
    #[bin_el_skip]
    pub dirty: bool,

    #[name("Filler")]
    pub filler: Vec<MapRectStrict>,
    #[optional]
    #[name("Style/color")]
    pub background_color: Option<String>,
    #[name("Style/Foregrounds")]
    pub foregrounds: Vec<CelesteMapStyleground>,
    #[name("Style/Backgrounds")]
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
    #[optional]
    #[name("meta")]
    pub meta: Option<CelesteMapMeta>,
}

impl CelesteMap {
    pub fn new() -> Self {
        Self {
            dirty: false,
            filler: vec![],
            background_color: None,
            foregrounds: vec![],
            backgrounds: vec![],
            levels: vec![],
            meta: None,
        }
    }
}

// this is a fucking mess.
#[derive(Debug, TryFromBinEl)]
#[name("meta")]
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
    #[name("AnimatedTiles")]
    #[optional]
    pub animated_tiles: Option<String>,
    #[name("Sprites")]
    #[optional]
    pub sprites: Option<String>,
    #[name("Portraits")]
    #[optional]
    pub portraits: Option<String>,
    #[name("IntroType")]
    #[optional]
    pub intro_type: Option<String>, // TODO I think this is an enum
    #[name("CassetteNoteColor")]
    #[optional]
    pub cassette_note_color: Option<String>,
    #[name("TitleTextColor")]
    #[optional]
    pub title_text_color: Option<String>,
    #[name("TitleBaseColor")]
    #[optional]
    pub title_base_color: Option<String>,
    #[name("TitleAccentColor")]
    #[optional]
    pub title_accent_color: Option<String>,
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
    #[name("DarknessAlpha")]
    #[optional]
    pub darkness_alpha: Option<f32>,
    #[name("CassetteSong")]
    #[optional]
    pub cassette_song: Option<String>,
    #[name("CoreMode")]
    #[optional]
    pub core_mode: Option<String>,
    #[name("PostcardSoundID")]
    #[optional]
    pub postcard_sound_id: Option<String>,
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
#[name("audiostate")]
pub struct CelesteMapMetaAudioState {
    #[name("Ambience")]
    pub ambience: String,
    #[name("Music")]
    pub music: String,
}

#[derive(Debug, Lens, Serialize, Deserialize)]
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
    pub enforce_dash_number: i32,

    pub music: String,
    pub alt_music: String,
    pub ambience: String,
    pub music_layers: [bool; 4],
    pub music_progress: String,
    pub ambience_progress: String,
    pub delay_alt_music_fade: bool,

    pub solids: TileGrid<char>,
    pub bg: TileGrid<char>,
    pub object_tiles: TileGrid<i32>,
    pub entities: Vec<CelesteMapEntity>,
    pub triggers: Vec<CelesteMapEntity>,
    pub fg_decals: Vec<CelesteMapDecal>,
    pub bg_decals: Vec<CelesteMapDecal>,
    pub fg_tiles: TileGrid<i32>,
    pub bg_tiles: TileGrid<i32>,

    #[serde(skip)]
    pub cache: RefCell<CelesteMapLevelCache>,
}

// totally normal clone except wipe the cache
impl Clone for CelesteMapLevel {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            bounds: self.bounds,
            color: self.color,
            camera_offset_x: self.camera_offset_x,
            camera_offset_y: self.camera_offset_y,
            wind_pattern: self.wind_pattern.clone(),
            space: self.space,
            underwater: self.underwater,
            whisper: self.whisper,
            dark: self.dark,
            disable_down_transition: self.disable_down_transition,
            enforce_dash_number: self.enforce_dash_number,
            music: self.music.clone(),
            alt_music: self.alt_music.clone(),
            ambience: self.ambience.clone(),
            music_layers: self.music_layers,
            music_progress: self.music_progress.clone(),
            ambience_progress: self.ambience_progress.clone(),
            delay_alt_music_fade: self.delay_alt_music_fade,
            solids: self.solids.clone(),
            bg: self.bg.clone(),
            object_tiles: self.object_tiles.clone(),
            entities: self.entities.clone(),
            triggers: self.triggers.clone(),
            fg_decals: self.fg_decals.clone(),
            bg_decals: self.bg_decals.clone(),
            fg_tiles: self.fg_tiles.clone(),
            bg_tiles: self.bg_tiles.clone(),
            cache: RefCell::new(Default::default()),
        }
    }
}

impl Default for CelesteMapLevel {
    fn default() -> Self {
        let tile_size = TileSize::new(40, 23);
        Self {
            name: "".to_string(),
            bounds: MapRectStrict::new(MapPointStrict::new(0, 0), MapSizeStrict::new(320, 184)),
            color: 0,
            camera_offset_x: 0.0,
            camera_offset_y: 0.0,
            wind_pattern: "None".to_string(),
            space: false,
            underwater: false,
            whisper: false,
            dark: false,
            disable_down_transition: false,
            enforce_dash_number: 0,
            music: "".to_string(),
            alt_music: "".to_string(),
            ambience: "".to_string(),
            music_layers: [true, true, true, true],
            music_progress: "".to_string(),
            ambience_progress: "".to_string(),
            delay_alt_music_fade: false,
            solids: TileGrid::new(tile_size, '0'),
            bg: TileGrid::new(tile_size, '0'),
            object_tiles: TileGrid::new(tile_size, -1),
            entities: vec![],
            triggers: vec![],
            fg_decals: vec![],
            bg_decals: vec![],
            fg_tiles: TileGrid::new(tile_size, -1),
            bg_tiles: TileGrid::new(tile_size, -1),
            cache: RefCell::new(Default::default()),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CelesteMapLevelUpdate {
    pub name: Option<String>,
    pub color: Option<i32>,
    pub camera_offset_x: Option<f32>,
    pub camera_offset_y: Option<f32>,
    pub wind_pattern: Option<String>,
    pub space: Option<bool>,
    pub underwater: Option<bool>,
    pub whisper: Option<bool>,
    pub dark: Option<bool>,
    pub disable_down_transition: Option<bool>,
    pub enforce_dash_number: Option<i32>,

    pub music: Option<String>,
    pub alt_music: Option<String>,
    pub ambience: Option<String>,
    pub music_layers: [Option<bool>; 4],
    pub music_progress: Option<String>,
    pub ambience_progress: Option<String>,
    pub delay_alt_music_fade: Option<bool>,
}

impl CelesteMapLevel {
    pub fn apply(&mut self, update: &mut CelesteMapLevelUpdate) {
        if let Some(x) = &mut update.name {
            swap(&mut self.name, x);
        }
        if let Some(x) = &mut update.color {
            swap(&mut self.color, x);
        }
        if let Some(x) = &mut update.camera_offset_x {
            swap(&mut self.camera_offset_x, x);
        };
        if let Some(x) = &mut update.camera_offset_y {
            swap(&mut self.camera_offset_y, x);
        };
        if let Some(x) = &mut update.wind_pattern {
            swap(&mut self.wind_pattern, x);
        };
        if let Some(x) = &mut update.space {
            swap(&mut self.space, x);
        };
        if let Some(x) = &mut update.underwater {
            swap(&mut self.underwater, x);
        };
        if let Some(x) = &mut update.whisper {
            swap(&mut self.whisper, x);
        };
        if let Some(x) = &mut update.dark {
            swap(&mut self.dark, x);
        };
        if let Some(x) = &mut update.disable_down_transition {
            swap(&mut self.disable_down_transition, x);
        };
        if let Some(x) = &mut update.enforce_dash_number {
            swap(&mut self.enforce_dash_number, x);
        };
        if let Some(x) = &mut update.music {
            swap(&mut self.music, x);
        };
        if let Some(x) = &mut update.alt_music {
            swap(&mut self.alt_music, x);
        };
        if let Some(x) = &mut update.ambience {
            swap(&mut self.ambience, x);
        };
        for i in 0..4 {
            if let Some(x) = &mut update.music_layers[i] {
                swap(&mut self.music_layers[i], x);
            };
        }
        if let Some(x) = &mut update.music_progress {
            swap(&mut self.music_progress, x);
        };
        if let Some(x) = &mut update.ambience_progress {
            swap(&mut self.ambience_progress, x);
        };
        if let Some(x) = &mut update.enforce_dash_number {
            swap(&mut self.enforce_dash_number, x);
        };
    }
}

#[derive(Default)]
pub struct CelesteMapLevelCache {
    pub render_cache_valid: bool,
    pub render_cache: Option<vg::ImageId>,
    pub last_entity_idx: usize,
    pub last_decal_idx: usize,
}

#[derive(Debug, Clone, PartialEq, TryFromBinEl, Lens, Serialize, Deserialize)]
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
    pub attributes: HashMap<String, Attribute>,
    #[children]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, TryFromBinEl, Data, Serialize, Deserialize)]
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

#[derive(Debug, Clone, TryFromBinEl, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, TryFromBinEl, Lens, Clone)]
pub struct CelesteMapStyleground {
    #[name]
    pub name: String,
    #[default]
    pub tag: String,
    #[default]
    pub x: f32,
    #[default]
    pub y: f32,
    #[default(1.0)]
    #[name("scrollx")]
    pub scroll_x: f32,
    #[default(1.0)]
    #[name("scrolly")]
    pub scroll_y: f32,
    #[default]
    #[name("speedx")]
    pub speed_x: f32,
    #[default]
    #[name("speedy")]
    pub speed_y: f32,
    #[default]
    pub color: String,
    #[default(1.0)]
    pub alpha: f32,
    #[default]
    #[name("flipx")]
    pub flip_x: bool,
    #[default]
    #[name("flipy")]
    pub flip_y: bool,
    #[default(true)]
    #[name("loopx")]
    pub loop_x: bool,
    #[default(true)]
    #[name("loopy")]
    pub loop_y: bool,
    #[default]
    pub wind: f32,
    #[optional]
    pub exclude: Option<RoomGlob>,
    #[optional]
    pub only: Option<RoomGlob>,
    #[optional]
    pub flag: Option<String>,
    #[optional]
    #[name("notflag")]
    pub not_flag: Option<String>,
    #[optional]
    pub always: Option<String>,
    #[optional]
    pub dreaming: Option<bool>,
    #[name("instantIn")]
    #[default(true)]
    pub instant_in: bool,
    #[name("instantOut")]
    #[default]
    pub instant_out: bool,
    #[name("fadex")]
    #[default]
    pub fade_x: FadeDirectives,
    #[name("fadey")]
    #[default]
    pub fade_y: FadeDirectives,

    #[attributes]
    pub attributes: HashMap<String, Attribute>,
    #[children]
    pub children: Vec<BinEl>,
}

#[derive(Debug, Clone)]
pub struct CelesteMapError {
    pub kind: CelesteMapErrorType,
    pub description: String,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CelesteMapErrorType {
    ParseError,
    MissingChild,
    MissingAttribute,
    BadAttrType,
    OutOfRange,
}

#[derive(Debug, PartialEq, Clone, Lens, Serialize, Deserialize)]
pub enum Attribute {
    Bool(bool),
    Int(i32),
    Float(f32),
    Text(String),
}

impl Default for CelesteMapStyleground {
    fn default() -> Self {
        Self {
            name: "parallax".to_string(),
            tag: "".to_string(),
            x: 0.0,
            y: 0.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            speed_x: 0.0,
            speed_y: 0.0,
            color: "".to_string(),
            alpha: 0.0,
            flip_x: false,
            flip_y: false,
            loop_x: false,
            loop_y: false,
            wind: 0.0,
            exclude: None,
            only: None,
            flag: None,
            not_flag: None,
            always: None,
            dreaming: None,
            instant_in: false,
            instant_out: false,
            fade_x: Default::default(),
            fade_y: Default::default(),
            attributes: Default::default(),
            children: vec![],
        }
    }
}

impl From<BinElAttr> for Attribute {
    fn from(s: BinElAttr) -> Self {
        match s {
            BinElAttr::Bool(b) => Attribute::Bool(b),
            BinElAttr::Int(i) => Attribute::Int(i),
            BinElAttr::Float(f) => Attribute::Float(f),
            BinElAttr::Text(s) => Attribute::Text(s),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<BinElAttr> for Attribute {
    fn into(self) -> BinElAttr {
        match self {
            Attribute::Bool(b) => BinElAttr::Bool(b),
            Attribute::Int(i) => BinElAttr::Int(i),
            Attribute::Float(f) => BinElAttr::Float(f),
            Attribute::Text(s) => BinElAttr::Text(s),
        }
    }
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

impl CelesteMapStyleground {
    pub fn visible(&self, room: &str, flags: &HashSet<String>, dreaming: bool) -> bool {
        /*
        this.ForceVisible
        || ((string.IsNullOrEmpty(this.OnlyIfNotFlag) || !level.Session.GetFlag(this.OnlyIfNotFlag))
            && ((!string.IsNullOrEmpty(this.AlsoIfFlag) && level.Session.GetFlag(this.AlsoIfFlag))
                || (!this.Dreaming.HasValue || this.Dreaming.Value == level.Session.Dreaming)
                    && (string.IsNullOrEmpty(this.OnlyIfFlag) || level.Session.GetFlag(this.OnlyIfFlag))
                    && (this.ExcludeFrom == null || !this.ExcludeFrom.Contains(level.Session.Level))
                    && (this.OnlyIn == null || this.OnlyIn.Contains(level.Session.Level))
            ));
         */
        self.not_flag.as_ref().map_or(true, |not_flag| {
            not_flag.is_empty() || !flags.contains(not_flag)
        }) && ((self
            .always
            .as_ref()
            .map_or(false, |always| flags.contains(always)))
            || (self.dreaming.map_or(true, |dream| dream == dreaming)
                && self
                    .flag
                    .as_ref()
                    .map_or(true, |flag| flag.is_empty() || flags.contains(flag))
                && self
                    .exclude
                    .as_ref()
                    .map_or(true, |exclude| !exclude.matches(room))
                && self.only.as_ref().map_or(true, |only| only.matches(room))))
    }
}

struct ParenFlipper;
impl TwoWayConverter<String> for ParenFlipper {
    type BinType = BinElAttr;

    fn try_parse(elem: &Self::BinType) -> Result<String, CelesteMapError> {
        DefaultConverter::try_parse(elem)
    }

    fn serialize(val: &String) -> Self::BinType {
        //assert!(!val.contains('\\'));
        //BinElAttr::Text(val.replace('/', "\\"))
        BinElAttr::Text(val.clone())
    }
}

impl CelesteMapLevel {
    pub fn tile(&self, pt: TilePoint, foreground: bool) -> Option<char> {
        let w = self.bounds.width() as i32 / 8;
        if pt.x < 0 || pt.x >= w {
            return None;
        }
        let tiles = if foreground { &self.solids } else { &self.bg };

        tiles.get(pt).copied()
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
            if !matches!(self.tile(pt, true), Some('0') | None) {
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
    pub fn styles(&self, fg: bool) -> &Vec<CelesteMapStyleground> {
        if fg {
            &self.foregrounds
        } else {
            &self.backgrounds
        }
    }

    pub fn styles_mut(&mut self, fg: bool) -> &mut Vec<CelesteMapStyleground> {
        if fg {
            &mut self.foregrounds
        } else {
            &mut self.backgrounds
        }
    }

    pub fn level_at(&self, pt: MapPointStrict) -> Option<usize> {
        for (idx, room) in self.levels.iter().enumerate() {
            if room.bounds.contains(pt) {
                return Some(idx);
            }
        }
        None
    }
}

use crate::celeste_mod::config::expression::Const;
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

pub fn from_reader(mut reader: impl std::io::Read) -> Result<CelesteMap, std::io::Error> {
    let mut file = vec![];
    reader.read_to_end(&mut file)?;
    let (_, binfile) = celeste::binel::parser::take_file(file.as_slice())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a Celeste map"))?;
    let map = from_binfile(binfile).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Data validation error: {}", e),
        )
    })?;

    Ok(map)
}

pub fn from_binfile(binfile: BinFile) -> Result<CelesteMap, CelesteMapError> {
    expect_elem!(binfile.root, "Map");
    CelesteMap::try_from_bin_el(&binfile.root)
}

impl TryFromBinEl for CelesteMapLevel {
    fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
        expect_elem!(elem, "level");

        let x = get_attr(elem, "x")?;
        let y = get_attr(elem, "y")?;
        let width = get_attr(elem, "width")?;
        let height = get_attr(elem, "height")?;
        let bounds = MapRectStrict {
            origin: Point2D::new(x, y),
            size: Size2D::new(width, height),
        };
        let fg_decals =
            DefaultConverter::from_bin_el_optional(elem, "fgdecals")?.unwrap_or_default();
        let bg_decals =
            DefaultConverter::from_bin_el_optional(elem, "bgdecals")?.unwrap_or_default();
        let name = DefaultConverter::from_bin_el(elem, "name")?;
        let color = DefaultConverter::from_bin_el_optional(elem, "c")?.unwrap_or_default();
        let camera_offset_x =
            DefaultConverter::from_bin_el_optional(elem, "cameraOffsetX")?.unwrap_or_default();
        let camera_offset_y =
            DefaultConverter::from_bin_el_optional(elem, "cameraOffsetY")?.unwrap_or_default();
        let wind_pattern =
            DefaultConverter::from_bin_el_optional(elem, "windPattern")?.unwrap_or_default();
        let space = DefaultConverter::from_bin_el_optional(elem, "space")?.unwrap_or_default();
        let underwater =
            DefaultConverter::from_bin_el_optional(elem, "underwater")?.unwrap_or_default();
        let whisper = DefaultConverter::from_bin_el_optional(elem, "whisper")?.unwrap_or_default();
        let dark = DefaultConverter::from_bin_el_optional(elem, "dark")?.unwrap_or_default();
        let disable_down_transition =
            DefaultConverter::from_bin_el_optional(elem, "disableDownTransition")?
                .unwrap_or_default();
        let enforce_dash_number =
            DefaultConverter::from_bin_el_optional(elem, "enforceDashNumber")?.unwrap_or_default();

        let music = DefaultConverter::from_bin_el_optional(elem, "music")?.unwrap_or_default();
        let alt_music =
            DefaultConverter::from_bin_el_optional(elem, "alt_music")?.unwrap_or_default();
        let ambience =
            DefaultConverter::from_bin_el_optional(elem, "ambience")?.unwrap_or_default();
        let music_layers = [
            DefaultConverter::from_bin_el_optional(elem, "musicLayer1")?.unwrap_or_default(),
            DefaultConverter::from_bin_el_optional(elem, "musicLayer2")?.unwrap_or_default(),
            DefaultConverter::from_bin_el_optional(elem, "musicLayer3")?.unwrap_or_default(),
            DefaultConverter::from_bin_el_optional(elem, "musicLayer4")?.unwrap_or_default(),
        ];
        let music_progress =
            DefaultConverter::from_bin_el_optional(elem, "musicProgress")?.unwrap_or_default();
        let ambience_progress =
            DefaultConverter::from_bin_el_optional(elem, "ambienceProgress")?.unwrap_or_default();
        let delay_alt_music_fade =
            DefaultConverter::from_bin_el_optional(elem, "delayAltMusicFade")?.unwrap_or_default();

        let solids = parse_fgbg_tiles(get_child(elem, "solids")?, width / 8, height / 8)?;
        let bg = parse_fgbg_tiles(get_child(elem, "bg")?, width / 8, height / 8)?;
        let object_tiles = get_optional_child(elem, "objtiles").map_or_else(
            || {
                Ok(TileGrid {
                    tiles: vec![-1; (width / 8 * height / 8) as usize],
                    stride: (width / 8) as usize,
                })
            },
            |v| parse_object_tiles(v, width / 8, height / 8),
        )?;
        let fg_tiles = get_optional_child(elem, "fgtiles").map_or_else(
            || {
                Ok(TileGrid {
                    tiles: vec![-1; (width / 8 * height / 8) as usize],
                    stride: (width / 8) as usize,
                })
            },
            |v| parse_object_tiles(v, width / 8, height / 8),
        )?;
        let bg_tiles = get_optional_child(elem, "bgtiles").map_or_else(
            || {
                Ok(TileGrid {
                    tiles: vec![-1; (width / 8 * height / 8) as usize],
                    stride: (width / 8) as usize,
                })
            },
            |v| parse_object_tiles(v, width / 8, height / 8),
        )?;
        let entities = DefaultConverter::from_bin_el(elem, "entities")?;
        let triggers = TryFromBinEl::try_from_bin_el(get_child(elem, "triggers")?)?;

        let cache = Default::default();

        Ok(CelesteMapLevel {
            bounds,
            name,
            color,
            camera_offset_x,
            camera_offset_y,
            wind_pattern,
            space,
            underwater,
            whisper,
            dark,
            disable_down_transition,
            enforce_dash_number,

            music,
            alt_music,
            ambience,
            music_layers,
            music_progress,
            ambience_progress,
            delay_alt_music_fade,

            solids,
            bg,
            object_tiles,
            entities,
            triggers,
            fg_decals,
            bg_decals,
            fg_tiles,
            bg_tiles,

            cache,
        })
    }

    fn to_binel(&self) -> BinEl {
        let mut elem = BinEl::new("level");

        let MapRectStrict {
            origin: Point2D { ref x, ref y, .. },
            size:
                Size2D {
                    ref width,
                    ref height,
                    ..
                },
        } = &self.bounds;
        DefaultConverter::set_bin_el(&mut elem, "x", x);
        DefaultConverter::set_bin_el(&mut elem, "y", y);
        DefaultConverter::set_bin_el(&mut elem, "width", width);
        DefaultConverter::set_bin_el(&mut elem, "height", height);
        DefaultConverter::set_bin_el(&mut elem, "fgdecals", &self.fg_decals);
        DefaultConverter::set_bin_el(&mut elem, "bgdecals", &self.bg_decals);
        DefaultConverter::set_bin_el(&mut elem, "name", &self.name);
        DefaultConverter::set_bin_el(&mut elem, "c", &self.color);
        DefaultConverter::set_bin_el_default(&mut elem, "cameraOffsetX", &self.camera_offset_x);
        DefaultConverter::set_bin_el_default(&mut elem, "cameraOffsetY", &self.camera_offset_y);
        DefaultConverter::set_bin_el_default(&mut elem, "windPattern", &self.wind_pattern);
        DefaultConverter::set_bin_el_default(&mut elem, "space", &self.space);
        DefaultConverter::set_bin_el_default(&mut elem, "underwater", &self.underwater);
        DefaultConverter::set_bin_el_default(&mut elem, "whisper", &self.whisper);
        DefaultConverter::set_bin_el_default(&mut elem, "dark", &self.dark);
        DefaultConverter::set_bin_el_default(&mut elem, "space", &self.space);
        DefaultConverter::set_bin_el_default(
            &mut elem,
            "disableDownTransition",
            &self.disable_down_transition,
        );
        DefaultConverter::set_bin_el_default(
            &mut elem,
            "enforceDashNumber",
            &self.enforce_dash_number,
        );

        DefaultConverter::set_bin_el_default(&mut elem, "music", &self.music);
        DefaultConverter::set_bin_el_default(&mut elem, "alt_music", &self.alt_music);
        DefaultConverter::set_bin_el_default(&mut elem, "ambience", &self.ambience);
        DefaultConverter::set_bin_el(&mut elem, "musicLayer1", &self.music_layers[0]);
        DefaultConverter::set_bin_el(&mut elem, "musicLayer2", &self.music_layers[1]);
        DefaultConverter::set_bin_el(&mut elem, "musicLayer3", &self.music_layers[2]);
        DefaultConverter::set_bin_el(&mut elem, "musicLayer4", &self.music_layers[3]);
        //DefaultConverter::set_bin_el(&mut elem, "musicLayer5", &self.music_layers[4]);
        //DefaultConverter::set_bin_el(&mut elem, "musicLayer6", &self.music_layers[5]);
        DefaultConverter::set_bin_el_default(&mut elem, "musicProgress", &self.music_progress);
        DefaultConverter::set_bin_el_default(
            &mut elem,
            "ambienceProgress",
            &self.ambience_progress,
        );
        DefaultConverter::set_bin_el_default(
            &mut elem,
            "delayAltMusicFade",
            &self.delay_alt_music_fade,
        );

        GetAttrOrChild::nested_apply_attr_or_child(
            &mut elem,
            "solids",
            serialize_tiles(&self.solids, '0', ""),
        );
        GetAttrOrChild::nested_apply_attr_or_child(
            &mut elem,
            "bg",
            serialize_tiles(&self.bg, '0', ""),
        );
        GetAttrOrChild::nested_apply_attr_or_child(
            &mut elem,
            "objtiles",
            serialize_tiles(&self.object_tiles, -1, ","),
        );
        GetAttrOrChild::nested_apply_attr_or_child(
            &mut elem,
            "fgtiles",
            serialize_tiles(&self.fg_tiles, -1, ","),
        );
        GetAttrOrChild::nested_apply_attr_or_child(
            &mut elem,
            "bgtiles",
            serialize_tiles(&self.bg_tiles, -1, ","),
        );
        DefaultConverter::set_bin_el(&mut elem, "entities", &self.entities);
        DefaultConverter::set_bin_el(&mut elem, "triggers", &self.triggers);

        elem
    }
}

fn serialize_tiles<T: Copy + PartialEq + ToString>(
    tiles: &TileGrid<T>,
    default: T,
    separator: &str,
) -> BinEl {
    let mut elem = BinEl::new("");
    let text = tiles
        .tiles
        .chunks(tiles.stride as usize)
        .map(|s| {
            let last_present = s.iter().rposition(|&c| c != default);
            if let Some(last_present) = last_present {
                &s[..=last_present]
            } else {
                &s[..0]
            }
        })
        .map(|s| s.iter().map(|x| x.to_string()).join(separator))
        .join("\n")
        .trim_end_matches('\n')
        .to_owned();
    DefaultConverter::set_bin_el_default(&mut elem, "innerText", &text);
    elem
}

fn fgbg_transform(
    s: &str,
) -> impl Iterator<Item = impl Iterator<Item = Result<char, CelesteMapError>> + '_> + '_ {
    s.lines().map(|line| line.chars().map(Ok))
}

fn parse_fgbg_tiles(
    elem: &BinEl,
    width: i32,
    height: i32,
) -> Result<TileGrid<char>, CelesteMapError> {
    parse_tiles(elem, width, height, fgbg_transform, '0')
}

fn parse_tiles<'a, T, F, I>(
    elem: &'a BinEl,
    width: i32,
    height: i32,
    transform: F,
    default: T,
) -> Result<TileGrid<T>, CelesteMapError>
where
    F: Fn(&'a str) -> I,
    I: Iterator + 'a,
    I::Item: Iterator<Item = Result<T, CelesteMapError>> + 'a,
    T: Copy + 'static,
{
    let exc = CelesteMapError {
        kind: CelesteMapErrorType::OutOfRange,
        description: format!("{} contains out-of-range data", elem.name),
    };
    let width: usize = width.try_into().map_err(|_| exc.clone())?;
    let height: usize = height.try_into().map_err(|_| exc)?;
    let mut data: Vec<T> = vec![default; width * height];

    let text: &str = elem.text().map(|s| s.as_str()).unwrap_or_default();
    for (y, line) in transform(text).enumerate() {
        for (x, num) in line.enumerate() {
            let num = num?;
            if x < width && y < height {
                data[x + y * width] = num;
            }
        }
    }

    Ok(TileGrid {
        tiles: data,
        stride: width as usize,
    })
}

fn obj_transform(
    s: &str,
) -> impl Iterator<Item = impl Iterator<Item = Result<i32, CelesteMapError>> + '_> + '_ {
    s.lines().map(|line| {
        line.split(',').filter(|&s| !s.is_empty()).map(|tile| {
            tile.parse::<i32>().map_err(|_| CelesteMapError {
                kind: CelesteMapErrorType::ParseError,
                description: format!("Could not parse {} as int", tile),
            })
        })
    })
}

fn parse_object_tiles(
    elem: &BinEl,
    width: i32,
    height: i32,
) -> Result<TileGrid<i32>, CelesteMapError> {
    parse_tiles(elem, width, height, obj_transform, -1)
}

#[derive(Debug, Clone, Lens)]
pub struct RoomGlob {
    text: String,
    regex: regex::RegexSet,
}

impl FromStr for RoomGlob {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let exprs = text
            .split(',')
            .map(|spec| format!("^{}$", spec.split('*').map(regex::escape).join(".*")));
        let regex = regex::RegexSet::new(exprs).unwrap();
        Ok(Self {
            text: text.to_owned(),
            regex,
        })
    }
}

impl ToString for RoomGlob {
    fn to_string(&self) -> String {
        self.text.clone()
    }
}

impl RoomGlob {
    pub fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl PartialEq for RoomGlob {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Data for RoomGlob {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl AttrCoercion for RoomGlob {
    const NICE_NAME: &'static str = "room glob";

    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        String::try_coerce(attr).map(|s| s.parse().unwrap())
    }

    fn serialize(&self) -> BinElAttr {
        AttrCoercion::serialize(&self.text)
    }
}

#[derive(Debug, Default, PartialEq, Clone, Data)]
pub struct FadeDirectives(pub Vec<FadeDirective>);

#[derive(Debug, PartialEq, Clone, Data)]
pub struct FadeDirective {
    pub pos_from: f32,
    pub pos_to: f32,
    pub fade_from: f32,
    pub fade_to: f32,
}

impl FromStr for FadeDirective {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (pos, fade) = text.split_once(',').ok_or(())?;
        let (pos_from, pos_to) = pos.split_once('-').ok_or(())?;
        let (fade_from, fade_to) = fade.split_once('-').ok_or(())?;
        let pos_from = parse_n_number(pos_from).ok_or(())?;
        let pos_to = parse_n_number(pos_to).ok_or(())?;
        let fade_from = fade_from.parse().map_err(|_| ())?;
        let fade_to = fade_to.parse().map_err(|_| ())?;

        Ok(Self {
            pos_from,
            pos_to,
            fade_from,
            fade_to,
        })
    }
}

impl ToString for FadeDirective {
    fn to_string(&self) -> String {
        format!(
            "{}-{},{}-{}",
            format_n_number(self.pos_from),
            format_n_number(self.pos_to),
            self.fade_from,
            self.fade_to,
        )
    }
}

impl FromStr for FadeDirectives {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Ok(Self(vec![]));
        }
        Ok(Self(
            text.split(':')
                .map(|s| s.parse())
                .collect::<Result<Vec<_>, Self::Err>>()?,
        ))
    }
}

impl ToString for FadeDirectives {
    fn to_string(&self) -> String {
        self.0.iter().map(|f| f.to_string()).join(":")
    }
}

impl AttrCoercion for FadeDirectives {
    const NICE_NAME: &'static str = "fade directive";

    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        String::try_coerce(attr).and_then(|s| s.as_str().parse().ok())
    }

    fn serialize(&self) -> BinElAttr {
        BinElAttr::Text(self.to_string())
    }
}

fn parse_n_number(text: &str) -> Option<f32> {
    if let Some(first) = text.chars().next() {
        if first == 'n' {
            text[1..].parse().ok().map(|x: f32| -x)
        } else {
            text.parse().ok()
        }
    } else {
        None
    }
}

fn format_n_number(f: f32) -> String {
    if f < 0.0 {
        format!("n{}", f.abs())
    } else {
        f.to_string()
    }
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
                AttrCoercion::serialize(val)
            }
        })+
    }
}
attr_converter_impl!(i32, u32, String, f32, bool, RoomGlob, FadeDirectives);

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
        let mut elem = BinEl::new("rect");

        <MapComponentConverter>::set_bin_el(&mut elem, "x", &self.origin.x);
        <MapComponentConverter>::set_bin_el(&mut elem, "y", &self.origin.y);
        <MapComponentConverter>::set_bin_el(&mut elem, "w", &self.size.width);
        <MapComponentConverter>::set_bin_el(&mut elem, "h", &self.size.height);
        elem
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
