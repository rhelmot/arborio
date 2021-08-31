use super::atlas_img;
use crate::atlas_img::SpriteReference;
use crate::map_struct::CelesteMapLevel;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::prelude::rust_2021::TryInto;
use std::str::FromStr;

use itertools::Itertools;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug)]
pub struct TextureTile {
    pub x: u32,
    pub y: u32,
    pub sprite: Option<atlas_img::SpriteReference>,
}

#[derive(Copy, Clone, Debug)]
pub struct TileReference {
    pub tile: TextureTile,
    pub texture: SpriteReference,
}

#[derive(Clone)]
pub struct Tileset {
    pub id: char,
    pub texture: SpriteReference,
    pub edges: [Vec<TextureTile>; 256],
    pub padding: Vec<TextureTile>,
    pub center: Vec<TextureTile>,
    pub ignores: Vec<char>,
    pub ignores_all: bool,
}

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
    pub fn load(
        path: &Path,
        gameplay_atlas: &atlas_img::Atlas,
    ) -> io::Result<HashMap<char, Tileset>> {
        let string = fs::read_to_string(path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Cannot open tileset {}: {}", path.to_str().unwrap(), e),
            )
        })?;
        let data: SerData =
            serde_xml_rs::from_str(string.trim_start_matches('\u{FEFF}')).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cannot open tileset {}: {:?}", path.to_str().unwrap(), e),
                )
            })?;
        let mut out: HashMap<char, Tileset> = HashMap::new();

        for s_tileset in data.tilesets {
            let id = match s_tileset.id.as_bytes() {
                [ch] => *ch as char,
                _ => {
                    assert_ascii!(s_tileset.id);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Tileset id ({}) must be a single character", s_tileset.id),
                    ));
                }
            };

            // HACK
            let path = format!(
                "tilesets/{}",
                if s_tileset.path == "template" {
                    "dirt"
                } else {
                    &s_tileset.path
                }
            );
            let texture_sprite = gameplay_atlas.lookup(&path).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Texture {} for tileset {} not found in the gameplay atlas",
                        s_tileset.path, s_tileset.id
                    ),
                )
            })?;
            const VEC: Vec<TextureTile> = Vec::new();
            let mut tileset = match s_tileset.copy.as_bytes() {
                [] => Tileset {
                    id,
                    texture: texture_sprite,
                    edges: [VEC; 256],
                    padding: vec![],
                    center: vec![],
                    ignores: vec![],
                    ignores_all: false,
                },
                // A single ASCII character
                [copied_id] => {
                    let copied = out.get(&(*copied_id as char)).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Can't find tileset to copy ({} copying {})",
                                s_tileset.id, s_tileset.copy
                            ),
                        )
                    })?;
                    Tileset {
                        texture: texture_sprite,
                        id,
                        ..copied.clone()
                    }
                }
                [_, _, ..] => {
                    assert_ascii!(s_tileset.copy);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Tileset copy id ({} copying {}) must be a single character",
                            s_tileset.id, s_tileset.copy
                        ),
                    ));
                }
            };

            match s_tileset.ignores.as_bytes() {
                b"*" => tileset.ignores_all = true,
                [] => {}
                [c] => tileset.ignores.push(*c as char),
                [_, _, ..] => {
                    assert_ascii!(s_tileset.ignores);
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("I actually don't know how to load this tileset ({}). Can you send your mod to rhelmot?", s_tileset.id)));
                }
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

                    for i in (0..256_usize).filter(|i| i & mask == value) {
                        tileset.edges[i] = tiles.clone();
                    }
                }
            }

            debug_assert_eq!(id, tileset.id);
            out.insert(id, tileset);
        }

        Ok(out)
    }

    pub fn tile(
        &self,
        level: &CelesteMapLevel,
        foreground: bool,
        x: i32,
        y: i32,
    ) -> Option<TileReference> {
        self.tile_g(x, y, |x, y| level.tile(x, y, foreground))
    }

    fn tile_g<F>(&self, x: i32, y: i32, tile: F) -> Option<TileReference>
    where
        F: Fn(i32, i32) -> Option<char>,
    {
        assert_eq!(tile(x, y), Some(self.id));

        let mut lookup = 0_usize;
        if self.is_filled(tile(x - 1, y - 1)) {
            lookup |= 1 << 0;
        }
        if self.is_filled(tile(x, y - 1)) {
            lookup |= 1 << 1;
        }
        if self.is_filled(tile(x + 1, y - 1)) {
            lookup |= 1 << 2;
        }
        if self.is_filled(tile(x - 1, y)) {
            lookup |= 1 << 3;
        }
        if self.is_filled(tile(x + 1, y)) {
            lookup |= 1 << 4;
        }
        if self.is_filled(tile(x - 1, y + 1)) {
            lookup |= 1 << 5;
        }
        if self.is_filled(tile(x, y + 1)) {
            lookup |= 1 << 6;
        }
        if self.is_filled(tile(x + 1, y + 1)) {
            lookup |= 1 << 7;
        }

        let tiles = if lookup == 0xff {
            if self.is_filled(tile(x - 2, y))
                && self.is_filled(tile(x + 2, y))
                && self.is_filled(tile(x, y - 2))
                && self.is_filled(tile(x, y + 2))
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

        let hash = {
            let mut hasher = DefaultHasher::new();
            (x, y).hash(&mut hasher);
            hasher.finish()
        } as usize;

        Some(TileReference {
            tile: tiles[hash % tiles.len()],
            texture: self.texture,
        })
    }

    fn ignores(&self, tile: char) -> bool {
        self.ignores_all || self.ignores.contains(&tile)
    }

    pub fn is_filled(&self, tile: Option<char>) -> bool {
        match tile {
            Some(ch) if ch == self.id => true,
            Some('0') => false,
            Some(ch) => !self.ignores(ch),
            None => true,
        }
    }
}

impl TextureTile {
    fn parse_list(text: &str, sprite: Option<SpriteReference>) -> io::Result<Vec<TextureTile>> {
        text.split(';').map(|piece| {
            if let Some((Ok(x), Ok(y))) = piece.split(',').map(u32::from_str).collect_tuple() {
                Ok(TextureTile {
                    x,
                    y,
                    sprite,
                })
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("Tile declaration (\"{}\") must be semicolon-separated sets of two comma-separated integers", text)))
            }
        }).collect()
    }
}
