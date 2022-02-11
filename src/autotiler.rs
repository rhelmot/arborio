use inflector::Inflector;
use std::collections::HashMap;
use std::io;

use crate::assets;
use crate::units::*;

#[derive(Copy, Clone, Debug)]
pub struct TextureTile {
    pub x: u32,
    pub y: u32,
}

#[derive(Copy, Clone)]
pub struct TileReference {
    pub tile: TextureTile,
    pub texture: &'static str,
}

#[derive(Clone, Debug)]
pub struct Tileset {
    pub id: char,
    pub name: &'static str,
    pub texture: &'static str,
    pub edges: Vec<Vec<TextureTile>>,
    pub padding: Vec<TextureTile>,
    pub center: Vec<TextureTile>,
    pub ignores: Vec<char>,
    pub ignores_all: bool,
}

pub type Autotiler = HashMap<char, Tileset>;

#[derive(serde::Deserialize)]
struct SerData {
    #[serde(rename = "Tileset", default)]
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
    #[allow(unused)] // TODO
    pub sprites: String,
}

macro_rules! assert_ascii {
    ($e:expr) => {
        if !$e.is_ascii() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "\"{}\" is not ascii!!!! Do NOT try to get funny with me!",
                    $e
                ),
            ));
        }
    };
}

impl Tileset {
    pub fn new<T: io::Read>(mut fp: T, texture_prefix: &str) -> Result<Autotiler, io::Error> {
        let mut string = String::new();
        fp.read_to_string(&mut string)?;
        let data: SerData =
            serde_xml_rs::from_str(string.trim_start_matches('\u{FEFF}')).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cannot open tileset: {:?}", e),
                )
            })?;
        let mut out: HashMap<char, Tileset> = HashMap::new();

        for s_tileset in data.tilesets {
            assert_ascii!(s_tileset.id);
            if s_tileset.id.len() != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Tileset id ({}) must be a single character", s_tileset.id),
                ));
            }
            let ch = s_tileset.id.chars().next().unwrap();

            // HACK
            let texture = format!(
                "{}{}",
                texture_prefix,
                if s_tileset.path == "template" {
                    "dirt"
                } else {
                    &s_tileset.path
                }
            );
            let mut tileset = if s_tileset.copy.is_empty() {
                Tileset {
                    id: ch,
                    name: assets::intern(&s_tileset.path.to_title_case()),
                    texture: assets::intern(&texture),
                    edges: vec![Vec::new(); 256],
                    padding: vec![],
                    center: vec![],
                    ignores: vec![],
                    ignores_all: false,
                }
            } else {
                assert_ascii!(s_tileset.copy);
                if s_tileset.copy.len() != 1 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Tileset copy id ({} copying {}) must be a single character",
                            s_tileset.id, s_tileset.copy
                        ),
                    ));
                }
                let mut r = match out.get(&s_tileset.copy.chars().next().unwrap()) {
                    Some(t) => t.clone(),
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Can't find tileset to copy ({} copying {})",
                                s_tileset.id, s_tileset.copy
                            ),
                        ))
                    }
                };
                r.texture = assets::intern(&texture);
                r.id = ch;
                r.name = assets::intern(&s_tileset.path.to_title_case());
                r
            };

            if s_tileset.ignores == "*" {
                tileset.ignores_all = true;
            } else if !s_tileset.ignores.is_empty() {
                // TODO is comma the right separator?
                tileset.ignores = s_tileset
                    .ignores
                    .split(',')
                    .map(|x| x.chars().next().unwrap())
                    .collect();
                // assert_ascii!(s_tileset.ignores);
                // if s_tileset.ignores.len() > 1 {
                //     return Err(io::Error::new(io::ErrorKind::InvalidData, format!("I actually don't know how to new this tileset ({}). Can you send your mod to rhelmot?", s_tileset.id)));
                // }
                // tileset.ignores.push(s_tileset.ignores.chars().next().unwrap());
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
                let tiles = TextureTile::parse_list(&s_set.tiles)?;
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
                        match ch {
                            'x' => {}
                            '0' => mask |= bit,
                            '1' => {
                                mask |= bit;
                                value |= bit;
                            }
                            _ => return false,
                        }
                        true
                    };

                    let mut success = true;
                    success |= process_bit(mask_vec[0], 1 << 0);
                    success |= process_bit(mask_vec[1], 1 << 1);
                    success |= process_bit(mask_vec[2], 1 << 2);
                    success |= process_bit(mask_vec[4], 1 << 3);
                    success |= process_bit(mask_vec[6], 1 << 4);
                    success |= process_bit(mask_vec[8], 1 << 5);
                    success |= process_bit(mask_vec[9], 1 << 6);
                    success |= process_bit(mask_vec[10], 1 << 7);

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

            out.insert(ch, tileset);
        }

        Ok(out)
    }

    fn ignores(&self, tile: char) -> bool {
        self.ignores_all || self.ignores.contains(&tile)
    }

    fn is_filled(&self, tile: Option<char>) -> bool {
        match tile {
            Some(ch) if ch == self.id => true,
            Some('0') | Some('\0') => false,
            Some(ch) => !self.ignores(ch),
            None => true,
        }
    }

    pub fn tile<F>(&self, pt: TilePoint, tile: &mut F) -> Option<TileReference>
    where
        F: Fn(TilePoint) -> Option<char>,
    {
        if tile(pt) != Some(self.id) {
            return None;
        }

        let hash = ((pt.x as u32).wrapping_mul(536870909) ^ (pt.y as u32).wrapping_mul(1073741789))
            as usize;

        let mut lookup = 0_usize;
        if self.is_filled(tile(pt + TileVector::new(-1, -1))) {
            lookup |= 1 << 0;
        }
        if self.is_filled(tile(pt + TileVector::new(0, -1))) {
            lookup |= 1 << 1;
        }
        if self.is_filled(tile(pt + TileVector::new(1, -1))) {
            lookup |= 1 << 2;
        }
        if self.is_filled(tile(pt + TileVector::new(-1, 0))) {
            lookup |= 1 << 3;
        }
        if self.is_filled(tile(pt + TileVector::new(1, 0))) {
            lookup |= 1 << 4;
        }
        if self.is_filled(tile(pt + TileVector::new(-1, 1))) {
            lookup |= 1 << 5;
        }
        if self.is_filled(tile(pt + TileVector::new(0, 1))) {
            lookup |= 1 << 6;
        }
        if self.is_filled(tile(pt + TileVector::new(1, 1))) {
            lookup |= 1 << 7;
        }

        let tiles = if lookup == 0xff {
            if self.is_filled(tile(pt + TileVector::new(-2, 0)))
                && self.is_filled(tile(pt + TileVector::new(2, 0)))
                && self.is_filled(tile(pt + TileVector::new(0, -2)))
                && self.is_filled(tile(pt + TileVector::new(0, 2)))
            {
                &self.center
            } else {
                &self.padding
            }
        } else {
            &self.edges[lookup]
        };

        if tiles.is_empty() {
            return None;
        }

        Some(TileReference {
            tile: tiles[hash % tiles.len()],
            texture: self.texture,
        })
    }
}

impl TextureTile {
    fn parse_list(text: &str) -> Result<Vec<TextureTile>, io::Error> {
        let mut result = vec![];
        if text.is_empty() {
            return Ok(result);
        }

        for piece in text.split(';') {
            let coords: Vec<&str> = piece.split(',').collect();
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
            })
        }

        Ok(result)
    }
}
