use std::error::Error;
use std::fmt;
use celeste::binel::*;
use std::collections::HashMap;
use std::borrow::Borrow;


#[derive(Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct CelesteMap {
    pub name: String,
    pub filler: Vec<Rect>,
    pub foregrounds: Vec<CelesteMapStyleground>,
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
}

#[derive(Debug)]
pub struct CelesteMapLevel {
    pub name: String,
    pub bounds: Rect,
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
}

#[derive(Debug)]
pub struct CelesteMapEntity {
    pub id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub attributes: HashMap<String, BinElAttr>,
}

#[derive(Debug)]
pub struct CelesteMapDecal {
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
    pub fn fg_tile(&self, x: i32, y: i32) -> Option<char> {
        let w = self.bounds.width as i32 / 8;
        let h = self.bounds.height as i32 / 8;
        if x < 0 || y < 0 || x >= w || y >= h {
            return None;
        }

        return Some(self.fg_tiles[(x + y * w) as usize]);
    }
}

macro_rules! ok_when {
    ($e:expr, $kind:expr) => {
        if $e.is_err() && $e.unwrap_err().kind != $kind {
            return Err($e.unwrap_err())
        } else {
            $e.ok()
        }
    }
}


macro_rules! expect_elem {
    ($elem:expr, $name:expr) => {
        if ($elem.name != $name) {
            return Err(CelesteMapError {
                kind: CelesteMapErrorType::ParseError,
                description: format!("Expected {} element, found {}", $name, $elem.name)
            })
        }
    }
}

pub fn from_binfile(binfile: BinFile) -> Result<CelesteMap, CelesteMapError> {
    expect_elem!(binfile.root, "Map");

    let filler = get_child(&binfile.root, "Filler")?;
    let levels = get_child(&binfile.root, "levels")?;
    let style = get_child(&binfile.root, "Style")?;
    let style_fg = get_child(&style, "Foregrounds")?;
    let style_bg = get_child(&style, "Backgrounds")?;

    let filler_parsed = filler.children().map(|child| parse_filler_rect(child)).collect::<Result<_, CelesteMapError>>()?;
    let style_fg_parsed = style_fg.children().map(|child| parse_styleground(child)).collect::<Result<_, CelesteMapError>>()?;
    let style_bg_parsed = style_bg.children().map(|child| parse_styleground(child)).collect::<Result<_, CelesteMapError>>()?;
    let levels_parsed = levels.children().map(|child| parse_level(child)).collect::<Result<_, CelesteMapError>>()?;

    return Ok(CelesteMap {
        name: binfile.package,
        filler: filler_parsed,
        foregrounds: style_fg_parsed,
        backgrounds: style_bg_parsed,
        levels: levels_parsed,
    });
}

fn parse_level(elem: &BinEl) -> Result<CelesteMapLevel, CelesteMapError> {
    expect_elem!(elem, "level");

    let x = require_present(get_attr_int(elem, "x"), &elem.name, "x")?;
    let y = require_present(get_attr_int(elem, "y"), &elem.name, "y")?;
    let width = require_present(get_attr_int(elem, "width"), &elem.name, "width")? as u32;
    let height = require_present(get_attr_int(elem, "height"), &elem.name, "height")? as u32;
    let object_tiles = ok_when!(get_child(elem, "fgtiles"), CelesteMapErrorType::MissingChild);
    let fg_decals = ok_when!(get_child(elem, "fgdecals"), CelesteMapErrorType::MissingChild);
    let bg_decals = ok_when!(get_child(elem, "bgdecals"), CelesteMapErrorType::MissingChild);

    Ok(CelesteMapLevel {
        bounds: Rect {
            x, y, width, height,
        },
        name: get_attr_text(elem, "name").and_then(|o|o.ok_or_else(||CelesteMapError::missing_attribute(&elem.name, "name")))?,
        color: get_attr_int(elem, "c")?.unwrap_or_default(),
        camera_offset_x: get_attr_float(elem, "cameraOffsetX")?.unwrap_or_default(),
        camera_offset_y: get_attr_float(elem, "cameraOffsetY")?.unwrap_or_default(),
        wind_pattern: get_attr_text(elem, "windPattern")?.unwrap_or_default(),
        space: get_attr_bool(elem, "space")?.unwrap_or_default(),
        underwater: get_attr_bool(elem, "underwater")?.unwrap_or_default(),
        whisper: get_attr_bool(elem, "whisper")?.unwrap_or_default(),
        dark: get_attr_bool(elem, "dark")?.unwrap_or_default(),
        disable_down_transition: get_attr_bool(elem, "disableDownTransition")?.unwrap_or_default(),

        music: get_attr_text(elem, "music")?.unwrap_or_default(),
        alt_music: get_attr_text(elem, "alt_music")?.unwrap_or_default(),
        ambience: get_attr_text(elem, "ambience")?.unwrap_or_default(),
        music_layers: [
            get_attr_bool(elem, "musicLayer1")?.unwrap_or_default(),
            get_attr_bool(elem, "musicLayer2")?.unwrap_or_default(),
            get_attr_bool(elem, "musicLayer3")?.unwrap_or_default(),
            get_attr_bool(elem, "musicLayer4")?.unwrap_or_default(),
            get_attr_bool(elem, "musicLayer5")?.unwrap_or_default(),
            get_attr_bool(elem, "musicLayer6")?.unwrap_or_default(),
        ],
        music_progress: get_attr_text(elem, "musicProgress")?.unwrap_or_default(),
        ambience_progress: get_attr_text(elem, "ambienceProgress")?.unwrap_or_default(),

        fg_tiles: parse_fgbg_tiles(get_child(elem, "solids")?, width/8, height/8)?,
        bg_tiles: parse_fgbg_tiles(get_child(elem, "bg")?, width/8, height/8)?,
        object_tiles: match object_tiles {
            Some(v) => parse_object_tiles(v, width, height),
            None => Ok(vec![-1; (width/8 * height/8) as usize])
        }?,
        entities: get_child(elem, "entities")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        triggers: get_child(elem, "triggers")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        fg_decals: match fg_decals {
            Some(v) => v.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },
        bg_decals: match bg_decals {
            Some(v) => v.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },
    })
}

fn parse_fgbg_tiles(elem: &BinEl, width: u32, height: u32) -> Result<Vec<char>, CelesteMapError> {
    let offset_x = get_attr_int(elem, "offsetX")?.unwrap_or_default();
    let offset_y = get_attr_int(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }
    let offset_x = offset_x as u32;
    let offset_y = offset_y as u32;

    let mut data: Vec<char> = vec!['0'; (width * height) as usize];
    let mut x = offset_x;
    let mut y = offset_y;
    for ch in get_attr_text(elem, "innerText")?.unwrap_or_default().chars() {
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

    return Ok(data);
}

fn parse_object_tiles(elem: &BinEl, width: u32, height: u32) -> Result<Vec<i32>, CelesteMapError> {
    let offset_x = get_attr_int(elem, "offsetX")?.unwrap_or_default();
    let offset_y = get_attr_int(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }
    let offset_x = offset_x as u32;
    let offset_y = offset_y as u32;

    let mut data: Vec<i32> = vec![-1; (width * height) as usize];
    let mut y = offset_y;

    for line in get_attr_text(elem, "innerText")?.unwrap_or_default().split('\n') {
        let mut x = offset_x;
        for num in line.split(',') {
            if num.is_empty() {
                continue;
            }
            let ch: i32 = match num.parse() {
                Err(_) => return Err(CelesteMapError { kind: CelesteMapErrorType::ParseError, description: format!("Could not parse {} as int", num)}),
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

    return Ok(data);
}

fn parse_entity_trigger(elem: &BinEl) -> Result<CelesteMapEntity, CelesteMapError> {
    let basic_attrs: Vec<String> = vec!["id".to_string(), "x".to_string(), "y".to_string(), "width".to_string(), "height".to_string()];
    Ok(CelesteMapEntity {
        id: require_present(get_attr_int(elem, "id"), &elem.name, "id")?,
        name: elem.name.clone(),
        x: require_present(get_attr_int(elem, "x"), &elem.name, "x")?,
        y: require_present(get_attr_int(elem, "y"), &elem.name, "y")?,
        width: get_attr_int(elem, "width")?.unwrap_or(0) as u32,
        height: get_attr_int(elem, "height")?.unwrap_or(0) as u32,
        attributes: elem.attributes.iter()
            .map(|kv| (kv.0.clone(), kv.1.clone()))
            .filter(|kv| !basic_attrs.contains(kv.0.borrow()))
            .collect()
    })
}

fn parse_decal(elem: &BinEl) -> Result<CelesteMapDecal, CelesteMapError> {
    Ok(CelesteMapDecal {
        x: require_present(get_attr_int(elem, "x"), &elem.name, "x")?,
        y: require_present(get_attr_int(elem, "y"), &elem.name, "y")?,
        scale_x: require_present(get_attr_float(elem, "scaleX"), &elem.name, "scaleX")?,
        scale_y: require_present(get_attr_float(elem, "scaleY"), &elem.name, "scaleY")?,
        texture: require_present(get_attr_text(elem, "texture"), &elem.name, "texture")?,
    })
}

fn parse_filler_rect(elem: & BinEl) -> Result<Rect, CelesteMapError> {
    expect_elem!(elem, "rect");

    let x = require_present(get_attr_int(elem, "x"), &elem.name, "x")?;
    let y = require_present(get_attr_int(elem, "y"), &elem.name, "y")?;
    let w = require_present(get_attr_int(elem, "w"), &elem.name, "w")?;
    let h = require_present(get_attr_int(elem, "h"), &elem.name, "h")?;

    return Ok(Rect { x: x * 8, y: y * 8, width: w as u32 * 8, height: h as u32 * 8 });
}

fn parse_styleground(elem :&BinEl) -> Result<CelesteMapStyleground, CelesteMapError> {
    Ok(CelesteMapStyleground {
        name: elem.name.clone(),
        texture: get_attr_text(elem, "texture")?,
        x: get_attr_int(elem, "x")?,
        y: get_attr_int(elem, "y")?,
        loop_x: get_attr_bool(elem, "loopx")?,
        loop_y: get_attr_bool(elem, "loopy")?,
        scroll_x: get_attr_float(elem, "scrollx")?,
        scroll_y: get_attr_float(elem, "scrolly")?,
        speed_x: get_attr_float(elem, "speedx")?,
        speed_y: get_attr_float(elem, "speedy")?,
        color: get_attr_text(elem, "color")?,
        blend_mode: get_attr_text(elem, "blendmode")?,
    })
}

fn get_child<'a>(elem: &'a BinEl, name: &str) -> Result<&'a BinEl, CelesteMapError> {
    let children_of_name = elem.get(name);
    if children_of_name.len() != 1 {
        return Err(CelesteMapError::missing_child(elem.name.as_ref(), name));
    }
    return Ok(children_of_name.first().unwrap());
}

fn get_attr_int(elem: &BinEl, name: &str) -> Result<Option<i32>, CelesteMapError> {
    return match elem.attributes.get(name) {
        Some(&BinElAttr::Int(i)) => Ok(Some(i)),
        Some(&BinElAttr::Float(f)) => Ok(Some(f as i32)),
        None => Ok(None),
        Some(attr) => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected int, found {:?}", attr) })
    }
}

fn get_attr_bool(elem: &BinEl, name: &str) -> Result<Option<bool>, CelesteMapError> {
    return match elem.attributes.get(name) {
        Some(&BinElAttr::Bool(b)) => Ok(Some(b)),
        None => Ok(None),
        Some(attr) => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected bool, found {:?}", attr) })
    }
}

fn get_attr_float(elem: &BinEl, name: &str) -> Result<Option<f32>, CelesteMapError> {
    return match elem.attributes.get(name) {
        Some(&BinElAttr::Float(f)) => Ok(Some(f)),
        Some(&BinElAttr::Int(i)) => Ok(Some(i as f32)),
        None => Ok(None),
        Some(attr) => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected float, found {:?}", attr) })
    }
}

fn get_attr_text(elem: &BinEl, name: &str) -> Result<Option<String>, CelesteMapError> {
    return match elem.attributes.get(name) {
        Some(BinElAttr::Text(s)) => Ok(Some(s.clone())),
        Some(BinElAttr::Int(i)) => Ok(Some(i.to_string())),
        None => Ok(None),
        Some(attr) => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected text, found {:?}", attr) })
    }
}

fn require_present<T>(result: Result<Option<T>, CelesteMapError>, element_name: &str, attribute_name: &str) -> Result<T, CelesteMapError> {
    result.and_then(|o| o.ok_or_else(||CelesteMapError::missing_attribute(element_name, attribute_name)))
}