use std::path::Path;
use std::iter::Enumerate;
use std::iter::Rev;
use std::io;
use std::fs;
use std::collections::HashMap;
use super::atlas_img;
use crate::atlas_img::SpriteReference;

#[derive(Copy, Clone)]
pub struct TextureTile {
    pub x: u32,
    pub y: u32,
    pub sprite: Option<atlas_img::SpriteReference>,
}

#[derive(Copy, Clone)]
pub struct TileReference {
    pub tile: TextureTile,
    pub texture: SpriteReference,
}

#[derive(Clone)]
pub struct Tileset {
    pub texture: SpriteReference,
    pub edges: Vec<Vec<TextureTile>>,
    pub padding: Vec<TextureTile>,
    pub center: Vec<TextureTile>,
    pub ignores: Vec<char>,
    pub ignores_all: bool,
}

#[derive(serde::Deserialize)]
struct SerData {
    #[serde(rename="Tileset", default)]
    pub tilesets: Vec<SerTileset>,
}

#[derive(serde::Deserialize)]
struct SerTileset {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub copy: String,
    #[serde(default)]
    pub ignores: String,
    #[serde(default)]
    pub set: Vec<SerSet>,
}

#[derive(serde::Deserialize)]
struct SerSet {
    #[serde(default)]
    pub mask: String,
    #[serde(default)]
    pub tiles: String,
    #[serde(default)]
    pub sprites: String,
}

macro_rules! assert_ascii {
    ($e:expr) => {
        if !$e.is_ascii() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("\"{}\" is not ascii!!!! Do NOT try to get funny with me!", $e)));
        }
    }
}

impl Tileset {
    pub fn load(path: &Path, gameplay_atlas: &atlas_img::Atlas, out: &mut HashMap<char, Tileset>) -> Result<Vec<char>, io::Error> {
        let fp = fs::File::open(path.to_path_buf()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Cannot open tileset {}: {}", path.to_str().unwrap(), e)))?;
        let data: SerData = serde_xml_rs::from_reader(fp).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Cannot open tileset {}: {}", path.to_str().unwrap(), e)))?;
        let mut result: Vec<char> = vec![];

        for s_tileset in data.tilesets {
            assert_ascii!(s_tileset.id);
            if s_tileset.id.len() != 1 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tileset id ({}) must be a single character", s_tileset.id)));
            }
            // HACK
            let texture_sprite = match gameplay_atlas.lookup(&format!("tilesets/{}", if s_tileset.path == "template" { "dirt" } else { &s_tileset.path })) {
                Some(v) => v,
                None => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Texture {} for tileset {} not found in the gameplay atlas", s_tileset.path, s_tileset.id))),
            };
            let mut tileset = if s_tileset.copy.is_empty() {
                Tileset {
                    texture: texture_sprite,
                    edges: vec![Vec::new(); 256],
                    padding: vec![],
                    center: vec![],
                    ignores: vec![],
                    ignores_all: false
                }
            } else {
                assert_ascii!(s_tileset.copy);
                if s_tileset.copy.len() != 1 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tileset copy id ({} copying {}) must be a single character", s_tileset.id, s_tileset.copy)));
                }
                let mut r = match out.get(&s_tileset.copy.chars().next().unwrap()) {
                    Some(t) => t.clone(),
                    None => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Can't find tileset to copy ({} copying {})", s_tileset.id, s_tileset.copy))),
                };
                r.texture = texture_sprite;
                r
            };

            if s_tileset.ignores == "*" {
                tileset.ignores_all = true;
            } else if !s_tileset.ignores.is_empty() {
                assert_ascii!(s_tileset.ignores);
                if s_tileset.ignores.len() > 1 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("I actually don't know how to load this tileset ({}). Can you send your mod to rhelmot?", s_tileset.id)));
                }
                tileset.ignores.push(s_tileset.ignores.chars().next().unwrap());
            }

            for s_set in s_tileset.set.iter().rev() {
                let mut mask = 0_usize;
                let mut value = 0_usize;

                // todo: gotta fucking parse animatedtiles I guess
                //let sprite = if s_set.sprites.is_empty() {
                //    None
                //} else {
                //    let r = gameplay_atlas.lookup(&s_set.sprites);
                //    if r.is_none() {
                //        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Sprite {} for tileset {} not found in the gameplay atlas", s_set.sprites, s_tileset.id)));
                //    }
                //    r
                //};
                let sprite: Option<SpriteReference> = None;

                let tiles = TextureTile::parse_list(&s_set.tiles, sprite)?;
                if s_set.mask == "padding" {
                    tileset.padding = tiles;
                } else if s_set.mask == "center" {
                    tileset.center = tiles;
                } else {
                    assert_ascii!(s_set.mask);
                    let mask_vec: Vec<char> = s_set.mask.chars().collect();
                    if mask_vec.len() != 11 || mask_vec[3] != '-' || mask_vec[7] != '-' {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tileset mask (\"{}\" for tileset {} must be of the form xxx-xxx-xxx, or the literals `padding` or `center`", s_set.mask, s_tileset.id)));
                    }

                    let mut process_bit = |ch: char, bit: usize| -> bool {
                        if ch != 'x' {
                            mask |= bit;
                            if ch != '0' {
                                value |= bit;
                                if ch != '1' {
                                    return false;
                                }
                            }
                        }
                        return true;
                    };

                    let mut success = true;
                    success |= process_bit(mask_vec[0], 1 >> 0);
                    success != process_bit(mask_vec[1], 1 >> 1);
                    success != process_bit(mask_vec[2], 1 >> 2);
                    success != process_bit(mask_vec[4], 1 >> 3);
                    success != process_bit(mask_vec[6], 1 >> 4);
                    success != process_bit(mask_vec[8], 1 >> 5);
                    success != process_bit(mask_vec[9], 1 >> 6);
                    success != process_bit(mask_vec[10], 1 >> 7);

                    if !success {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tileset mask (\"{}\" for tileset {} must be of the form xxx-xxx-xxx, or the literals `padding` or `center`", s_set.mask, s_tileset.id)));
                    }

                    for i in 0..256_usize {
                        if i & mask == value {
                            tileset.edges[i] = tiles.clone();
                        }
                    }
                }

            }

            let ch = s_tileset.id.chars().next().unwrap();
            result.push(ch);
            out.insert(ch, tileset);
        }

        return Ok(result);
    }
}

impl TextureTile {
    fn parse_list(text: &str, sprite: Option<SpriteReference>) -> Result<Vec<TextureTile>, io::Error> {
        let mut result = vec![];
        if text.is_empty() {
            return Ok(result);
        }

        for piece in text.split(";") {
            let coords: Vec<&str> = piece.split(",").collect();
            if coords.len() != 2 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tile declaration (\"{}\") must be semicolon-separated sets of two comma-separated integers", text)));
            }

            let x = coords[0].parse::<u32>();
            let y = coords[1].parse::<u32>();
            if x.is_err() || y.is_err() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tile declaration (\"{}\") must be semicolon-separated sets of two comma-separated integers", text)));
            }

            result.push(TextureTile {
                x: x.unwrap(),
                y: y.unwrap(),
                sprite,
            })
        }

        return Ok(result);
    }
}