use crate::autotiler::TileReference;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::path;
use crate::image_view::{ImageView, ImageBuffer, ImageViewMut};
use crate::map_struct::Rect;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct SpriteReference {
    atlas: u32,
    idx: u32,
}

pub struct Atlas {
    identifier: u32,
    pub(crate) blobs: Vec<ImageBuffer>,
    sprites_map: HashMap<String, usize>,
    sprites: Vec<AtlasSprite>,
}

pub struct AtlasSprite {
    blob_idx: usize,
    bounding_box: Rect,
    offset_x: i32,
    offset_y: i32,
    real_width: u32,
    real_height: u32,
}

impl Atlas {
    pub fn load(meta_file: &path::Path) -> Result<Atlas, io::Error> {
        let fp = fs::File::open(meta_file.to_path_buf())?;
        let mut reader = io::BufReader::new(fp);

        // this code ripped shamelessly from ahorn
        let _version = reader.read_u32::<LittleEndian>()?;
        let _cmd = read_string(&mut reader)?;
        let _checksum = reader.read_u32::<LittleEndian>()?;

        let mut result = Atlas {
            identifier: rand::random(),
            blobs: Vec::new(),
            sprites_map: HashMap::new(),
            sprites: Vec::new(),
        };

        let count = reader.read_u16::<LittleEndian>()?;
        for _ in 0..count {
            let data_file = read_string(&mut reader)? + ".data";
            let data_path = meta_file.with_file_name(&data_file);
            let blob_idx = result.blobs.len();
            result.blobs.push(load_data_file(data_path)?);

            let sprites = reader.read_u16::<LittleEndian>()?;
            for _ in 0..sprites {
                let sprite_path = read_string(&mut reader)?.replace("\\", "/");
                let x = reader.read_u16::<LittleEndian>()?;
                let y = reader.read_u16::<LittleEndian>()?;
                let width = reader.read_u16::<LittleEndian>()?;
                let height = reader.read_u16::<LittleEndian>()?;
                let offset_x = reader.read_i16::<LittleEndian>()?;
                let offset_y = reader.read_i16::<LittleEndian>()?;
                let real_width = reader.read_u16::<LittleEndian>()?;
                let real_height = reader.read_u16::<LittleEndian>()?;

                let sprite_idx = result.sprites.len();
                result.sprites_map.insert(sprite_path, sprite_idx);
                result.sprites.push(AtlasSprite {
                    blob_idx,
                    bounding_box: Rect {
                        x: x as i32,
                        y: y as i32,
                        width: width as u32,
                        height: height as u32,
                    },
                    offset_x: offset_x as i32,
                    offset_y: offset_y as i32,
                    real_width: real_width as u32,
                    real_height: real_height as u32,
                });
            }
        }

        Ok(result)
    }

    pub fn lookup(&self, path: &str) -> Option<SpriteReference> {
        let path = path.replace("\\", "/");
        self.sprites_map.get(&path)
            .map(|v| SpriteReference{
            atlas: self.identifier,
            idx: *v as u32,
        })
    }

    pub(crate) fn resized_sprite<'a>(&'a self, sprite_ref: SpriteReference, map_scale: u32, resized_sprite_cache: &'a mut HashMap<SpriteReference, ImageBuffer>) -> ImageView<'a> {
        assert_eq!(self.identifier, sprite_ref.atlas);
        let resized = resized_sprite_cache.entry(sprite_ref).or_insert_with(|| {
            let sprite = &self.sprites[sprite_ref.idx as usize];
            let image_blob = &self.blobs[sprite.blob_idx];
            let clipped_source = image_blob.subsection(&sprite.bounding_box);
            let resized_width = sprite.bounding_box.width * map_scale / 8;
            let resized_height = sprite.bounding_box.height * map_scale / 8;
            clipped_source.resize(map_scale, 8)
        });
        resized.as_ref()
    }

    pub(crate) fn resized_tile<'a>(&'a self, tile_ref: TileReference, map_scale: u32, resized_sprite_cache: &'a mut HashMap<SpriteReference, ImageBuffer>) -> ImageView<'a> {
        let resized_sprite = self.resized_sprite(tile_ref.texture, map_scale, resized_sprite_cache);
        resized_sprite.subsection(&Rect {
            x: (tile_ref.tile.x * map_scale) as i32,
            y: (tile_ref.tile.y * map_scale) as i32,
            width: map_scale,
            height: map_scale,
        })
    }

    pub fn draw(&self, sprite_ref: SpriteReference, x: i32, y: i32, map_scale: u32, destination: ImageViewMut, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
        let resized = self.resized_sprite(sprite_ref, map_scale, resized_sprite_cache);
        resized.draw_to(destination, x as u32, y as u32);
    }

    pub fn draw_tile(&self, tile_ref: TileReference, x: u32, y: u32, map_scale: u32, destination: ImageViewMut, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
        let resized = self.resized_tile(tile_ref, map_scale, resized_sprite_cache);
        resized.draw_to(destination, x as u32, y as u32);
    }
}

fn read_string(reader: &mut io::BufReader<fs::File>) -> Result<String, io::Error> {
    let strlen = reader.read_u8()? as usize;
    let mut buf = vec![0u8; strlen];
    reader.read_exact(buf.as_mut_slice())?;

    String::from_utf8(buf).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8")
    })
}

pub fn load_data_file(data_file: path::PathBuf) -> Result<ImageBuffer, io::Error> {
    let fp = fs::File::open(data_file)?;

    let mut reader = io::BufReader::new(fp);

    let width = reader.read_u32::<LittleEndian>()?;
    let height = reader.read_u32::<LittleEndian>()?;
    let has_alpha = reader.read_u8()? != 0;

    let total_px = (width * height) as usize;
    let mut current_px: usize = 0;
    let mut buf: Vec<u8> = vec![0u8; total_px * 4];

    while current_px < total_px {
        let current_idx = current_px * 4;
        let repeat = reader.read_u8()?;
        let repeat = if repeat > 0 { repeat - 1 } else { 0 } as usize; // this is off-by-one from the julia code because it's more ergonomic
        let alpha = if has_alpha {
            reader.read_u8()?
        } else {
            255
        };
        if alpha > 0 {
            reader.read_exact(&mut buf[current_idx..current_idx+3])?;
            buf[current_idx..current_idx+3].reverse();
            buf[current_idx+3] = alpha;
        }
        // no else case needed: they're already zeros

        if repeat > 0 {
            let (first, second) = buf.split_at_mut(current_idx + 4);
            let src = &mut first[current_idx..];
            let dest = &mut second[..repeat * 4];
            for chunk in dest.chunks_mut(4) {
                chunk.clone_from_slice(&src);
            }
        }

        current_px += repeat + 1;
    }

    Ok(ImageBuffer::from_vec(buf, width * 4))
}