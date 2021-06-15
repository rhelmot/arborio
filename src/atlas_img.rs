use crate::autotiler::TileReference;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use fltk::prelude::ImageExt;
use std::collections::HashMap;
use std::convert::TryInto;
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

        return Ok(result);
    }

    pub fn lookup(&self, path: &str) -> Option<SpriteReference> {
        let path = path.replace("\\", "/");
        self.sprites_map.get(&path)
            .map(|v| SpriteReference{
            atlas: self.identifier,
            idx: *v as u32,
        })
    }


    pub fn draw(&self, sprite_ref: SpriteReference, x: i32, y: i32, map_scale: u32, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        assert_eq!(self.identifier, sprite_ref.atlas);
        let sprite = &self.sprites[sprite_ref.idx as usize];
        let image_blob = &self.blobs[sprite.blob_idx];
        let resized_width = sprite.bounding_box.width * map_scale / 8;
        let resized_height = sprite.bounding_box.height * map_scale / 8;
        // let resized = resized_sprite_cache.entry(sprite_ref).or_insert_with(||Self::resize(&blob[(sprite.x * 4 + sprite.y * 4 * width as u32) as usize..], width as u32, sprite.width, sprite.height, resized_width, resized_height));
        let resized = todo!();
        // Safety: this object will not live longer than this function, and the blob reference
        // is all but static. No idea about the line depth stuff.
        let mut view = unsafe {
            fltk::image::RgbImage::from_data2(
                resized,
                resized_width as i32,
                resized_height as i32,
                4,
                (resized_width * 4) as i32).unwrap()
        };

        view.draw(x, y, view.width(), view.height());
    }

    pub fn draw_tile(&self, tile_ref: TileReference, x: u32, y: u32, map_scale: u32, destination: ImageViewMut, resized_sprite_cache: &mut HashMap<SpriteReference, ImageBuffer>) {
        assert_eq!(self.identifier, tile_ref.texture.atlas);
        let sprite = &self.sprites[tile_ref.texture.idx as usize];
        // let (ref blob, width) = self.blobs[sprite.blob_idx];
        let image_blob = &self.blobs[sprite.blob_idx];
        let clipped_source = image_blob.subsection(&sprite.bounding_box);
        let resized_width = sprite.bounding_box.width * map_scale / 8;
        let resized_height = sprite.bounding_box.height * map_scale / 8;
        let resized = resized_sprite_cache.entry(tile_ref.texture).or_insert_with(||clipped_source.resize(resized_width, resized_height));
        debug_assert!(tile_ref.tile.x * 8 <= clipped_source.width);
        resized.as_ref().subsection(&Rect {
            x: (tile_ref.tile.x * map_scale) as i32,
            y: (tile_ref.tile.y * map_scale) as i32,
            width: map_scale,
            height: map_scale,
        }).draw_to(destination, x, y);
        // image_blob.as_ref().draw(x as i32, y as i32);
    }
}

fn read_string(reader: &mut io::BufReader<fs::File>) -> Result<String, io::Error> {
    let mut buf1 = [0u8; 1];
    reader.read_exact(&mut buf1)?;
    let strlen = buf1[0] as usize;
    let mut buf = vec![0u8; strlen];
    reader.read_exact(buf.as_mut_slice())?;
    return match String::from_utf8(buf) {
        Ok(v) => Ok(v),
        Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8")),
    }
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
        if has_alpha {
            let alpha = reader.read_u8()?;
            if alpha > 0 {
                let len = buf.len();
                reader.read_exact(&mut buf[current_idx..current_idx+3])?;
                buf[current_idx..current_idx+3].reverse();
                buf[current_idx+3] = alpha;
            }
            // no else case needed: they're already zeros
        } else {
            reader.read_exact(&mut buf[current_idx..current_idx+3])?;
            buf[current_idx..current_idx+3].reverse();
            buf[current_idx+3] = 255;
        }

        if repeat > 0 {
            let (first, second) = buf.split_at_mut(current_idx + 4);
            let src = &mut first[current_idx..];
            let mut dest = &mut second[..repeat * 4];
            for chunk in dest.chunks_mut(4) {
                chunk.clone_from_slice(&src);
            }
        }

        current_px += repeat + 1;
    }

    return Ok(ImageBuffer::from_vec(buf, width * 4));
}