use std::error::Error;
use std::fmt;
use celeste::binel::*;
use std::collections::HashMap;
use std::borrow::Borrow;

#[derive(Debug)]
pub struct CelesteMap {
    pub name: String,
    pub filler: Vec<CelesteMapRect>,
    pub foregrounds: Vec<CelesteMapStyleground>,
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
}

#[derive(Debug)]
pub struct CelesteMapLevel {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub color: i32,
    pub camera_offset_x: f32,
    pub camera_offset_y: f32,
    pub wind_pattern: String,
    pub space: bool,
    pub underwater: bool,
    pub whisper: bool,
    pub dark: bool,
    pub disable_down_transition: bool,
    pub object_tiles_tileset: String,

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
pub struct CelesteMapRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
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
    fn parse_error(description: String) -> CelesteMapError {
        CelesteMapError {
            kind: CelesteMapErrorType::ParseError,
            description,
        }
    }

    fn parse_error_str(description: &str) -> CelesteMapError {
        CelesteMapError::parse_error(format!("{}", description))
    }

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

    let object_tiles = get_child(elem, "fgtiles")?;
    let x = get_attr_int(elem, "x")?;
    let y = get_attr_int(elem, "y")?;
    let width = get_attr_int(elem, "width")? as u32;
    let height = get_attr_int(elem, "height")? as u32;

    Ok(CelesteMapLevel {
        x, y, width, height,
        name: get_attr_text(elem, "name")?,
        color: ok_when!(get_attr_int(elem, "c"), CelesteMapErrorType::MissingAttribute).unwrap_or(0),
        camera_offset_x: ok_when!(get_attr_float(elem, "cameraOffsetX"), CelesteMapErrorType::MissingAttribute).unwrap_or(0.),
        camera_offset_y: ok_when!(get_attr_float(elem, "cameraOffsetY"), CelesteMapErrorType::MissingAttribute).unwrap_or(0.),
        wind_pattern: ok_when!(get_attr_text(elem, "windPattern"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("None")),
        space: ok_when!(get_attr_bool(elem, "space"), CelesteMapErrorType::MissingAttribute).unwrap_or(false),
        underwater: ok_when!(get_attr_bool(elem, "underwater"), CelesteMapErrorType::MissingAttribute).unwrap_or(false),
        whisper: ok_when!(get_attr_bool(elem, "whisper"), CelesteMapErrorType::MissingAttribute).unwrap_or(false),
        dark: ok_when!(get_attr_bool(elem, "dark"), CelesteMapErrorType::MissingAttribute).unwrap_or(false),
        disable_down_transition: ok_when!(get_attr_bool(elem, "disableDownTransition"), CelesteMapErrorType::MissingAttribute).unwrap_or(false),
        object_tiles_tileset: ok_when!(get_attr_text(object_tiles, "tileset"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("Scenery")),

        music: ok_when!(get_attr_text(elem, "music"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("")),
        alt_music: ok_when!(get_attr_text(elem, "alt_music"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("")),
        ambience: ok_when!(get_attr_text(elem, "ambience"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("")),
        music_layers: [
            ok_when!(get_attr_bool(elem, "musicLayer1"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
            ok_when!(get_attr_bool(elem, "musicLayer2"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
            ok_when!(get_attr_bool(elem, "musicLayer3"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
            ok_when!(get_attr_bool(elem, "musicLayer4"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
            ok_when!(get_attr_bool(elem, "musicLayer5"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
            ok_when!(get_attr_bool(elem, "musicLayer6"), CelesteMapErrorType::MissingAttribute).unwrap_or(true),
        ],
        music_progress: ok_when!(get_attr_text(elem, "musicProgress"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("")),
        ambience_progress: ok_when!(get_attr_text(elem, "ambienceProgress"), CelesteMapErrorType::MissingAttribute).unwrap_or(String::from("")),

        fg_tiles: parse_fgbg_tiles(get_child(elem, "solids")?, width/8, height/8)?,
        bg_tiles: parse_fgbg_tiles(get_child(elem, "bgtiles")?, width/8, height/8)?,
        object_tiles: parse_object_tiles(object_tiles, width, height)?,
        entities: get_child(elem, "entities")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        triggers: get_child(elem, "triggers")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        fg_decals: get_child(elem, "fgdecals")?.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
        bg_decals: get_child(elem, "bgdecals")?.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
    })
}

fn parse_fgbg_tiles(elem: &BinEl, width: u32, height: u32) -> Result<Vec<char>, CelesteMapError> {
    let offset_x = get_attr_int(elem, "offsetX")?;
    let offset_y = get_attr_int(elem, "offsetY")?;
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }
    let offset_x = offset_x as u32;
    let offset_y = offset_y as u32;

    let mut data: Vec<char> = vec!['0'; (width * height) as usize];
    println!("Allocated {}", data.len());
    let mut x = offset_x;
    let mut y = offset_y;
    for ch in ok_when!(get_attr_text(elem, "innerText"), CelesteMapErrorType::MissingAttribute).unwrap_or("".to_string()).chars() {
        if ch == '\n' {
            x = offset_x;
            y += 1;
        } else if ch == '\r' {
        } else if x >= width || y >= height {
            return exc;
        } else {
            data[(x + y * width) as usize] = ch;
            x += 1;
        }
    }

    return Ok(data);
}

fn parse_object_tiles(elem: &BinEl, width: u32, height: u32) -> Result<Vec<i32>, CelesteMapError> {
    let offset_x = get_attr_int(elem, "offsetX")?;
    let offset_y = get_attr_int(elem, "offsetY")?;
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }
    let offset_x = offset_x as u32;
    let offset_y = offset_y as u32;

    let mut data: Vec<i32> = vec![-1; (width * height) as usize];
    println!("Allocated {}", data.len());
    let mut y = offset_y;

    for line in ok_when!(get_attr_text(elem, "innerText"), CelesteMapErrorType::MissingAttribute).unwrap_or("".to_string()).split('\n') {
        let mut x = offset_x;
        for num in line.split(',') {
            if num == "" {
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
        id: get_attr_int(elem, "id")?,
        name: elem.name.clone(),
        x: get_attr_int(elem, "x")?,
        y: get_attr_int(elem, "y")?,
        width: ok_when!(get_attr_int(elem, "width"), CelesteMapErrorType::MissingAttribute).unwrap_or(0) as u32,
        height: ok_when!(get_attr_int(elem, "height"), CelesteMapErrorType::MissingAttribute).unwrap_or(0) as u32,
        attributes: elem.attributes.iter()
            .map(|kv| (kv.0.clone(), kv.1.clone()))
            .filter(|kv| !basic_attrs.contains(kv.0.borrow()))
            .collect()
    })
}

fn parse_decal(elem: &BinEl) -> Result<CelesteMapDecal, CelesteMapError> {
    Ok(CelesteMapDecal {
        x: get_attr_int(elem, "x")?,
        y: get_attr_int(elem, "y")?,
        scale_x: get_attr_float(elem, "scaleX")?,
        scale_y: get_attr_float(elem, "scaleY")?,
        texture: get_attr_text(elem, "texture")?,
    })
}

fn parse_filler_rect(elem: & BinEl) -> Result<CelesteMapRect, CelesteMapError> {
    expect_elem!(elem, "rect");

    let x = get_attr_int(elem, "x")?;
    let y = get_attr_int(elem, "y")?;
    let w = get_attr_int(elem, "w")?;
    let h = get_attr_int(elem, "h")?;

    return Ok(CelesteMapRect { x, y, width: w as u32, height: h as u32 });
}

fn parse_styleground(elem :&BinEl) -> Result<CelesteMapStyleground, CelesteMapError> {
    Ok(CelesteMapStyleground {
        name: elem.name.clone(),
        texture: ok_when!(get_attr_text(elem, "texture"), CelesteMapErrorType::MissingAttribute),
        x: ok_when!(get_attr_int(elem, "x"), CelesteMapErrorType::MissingAttribute),
        y: ok_when!(get_attr_int(elem, "y"), CelesteMapErrorType::MissingAttribute),
        loop_x: ok_when!(get_attr_bool(elem, "loopx"), CelesteMapErrorType::MissingAttribute),
        loop_y: ok_when!(get_attr_bool(elem, "loopy"), CelesteMapErrorType::MissingAttribute),
        scroll_x: ok_when!(get_attr_float(elem, "scrollx"), CelesteMapErrorType::MissingAttribute),
        scroll_y: ok_when!(get_attr_float(elem, "scrolly"), CelesteMapErrorType::MissingAttribute),
        speed_x: ok_when!(get_attr_float(elem, "speedx"), CelesteMapErrorType::MissingAttribute),
        speed_y: ok_when!(get_attr_float(elem, "speedy"), CelesteMapErrorType::MissingAttribute),
        color: ok_when!(get_attr_text(elem, "color"), CelesteMapErrorType::MissingAttribute),
        blend_mode: ok_when!(get_attr_text(elem, "blendmode"), CelesteMapErrorType::MissingAttribute),
    })
}

fn get_child<'a>(elem: &'a BinEl, name: &str) -> Result<&'a BinEl, CelesteMapError> {
    let children_of_name = elem.get(name);
    if children_of_name.len() != 1 {
        return Err(CelesteMapError::missing_child(elem.name.as_ref(), name));
    }
    return Ok(children_of_name.first().unwrap());
}

fn get_attr_int(elem: &BinEl, name: &str) -> Result<i32, CelesteMapError> {
    let attr = elem.attributes.get(name).ok_or(CelesteMapError::missing_attribute(elem.name.as_ref(), name))?;
    return match attr {
        BinElAttr::Int(i) => Ok(i.clone()),
        BinElAttr::Float(f) => Ok(f.clone() as i32),
        _ => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected int, found {:?}", attr) })
    }
}

fn get_attr_bool(elem: &BinEl, name: &str) -> Result<bool, CelesteMapError> {
    let attr = elem.attributes.get(name).ok_or(CelesteMapError::missing_attribute(elem.name.as_ref(), name))?;
    return match attr {
        BinElAttr::Bool(b) => Ok(b.clone()),
        _ => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected bool, found {:?}", attr) })
    }
}

fn get_attr_float(elem: &BinEl, name: &str) -> Result<f32, CelesteMapError> {
    let attr = elem.attributes.get(name).ok_or(CelesteMapError::missing_attribute(elem.name.as_ref(), name))?;
    return match attr {
        BinElAttr::Float(f) => Ok(f.clone()),
        BinElAttr::Int(i) => Ok(i.clone() as f32),
        _ => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected float, found {:?}", attr) })
    }
}

fn get_attr_text(elem: &BinEl, name: &str) -> Result<String, CelesteMapError> {
    let attr = elem.attributes.get(name).ok_or(CelesteMapError::missing_attribute(elem.name.as_ref(), name))?;
    return match attr {
        BinElAttr::Text(s) => Ok(s.clone()),
        _ => Err(CelesteMapError { kind: CelesteMapErrorType::BadAttrType, description: format!("Expected text, found {:?}", attr) })
    }
}
