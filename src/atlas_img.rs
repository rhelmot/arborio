use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::path;
use fltk::prelude::ImageExt;
use byteorder::{ReadBytesExt, BigEndian, LittleEndian};

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct SpriteReference {
    atlas: u32,
    idx: u32,
}

pub struct Atlas {
    identifier: u32,
    blobs: Vec<(Vec<u8>, i32)>,
    sprites_map: HashMap<String, usize>,
    sprites: Vec<AtlasSprite>,
}

pub struct AtlasSprite {
    blob_idx: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
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
                    x: x as u32,
                    y: y as u32,
                    width: width as u32,
                    height: height as u32,
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

    fn resize(data: &[u8], line_width: usize, width: usize, height: usize, new_width: usize, new_height: usize) -> Vec<u8> {

        let mut new_data = vec![0_u8; new_height * new_width * 4];
        for row in 0..new_height {
            let old_row = row * height / new_height;
            for col in 0..new_width {
                let old_col = col * width / new_width;
                let old_pos = old_row * line_width + old_col;
                let pos = row * new_width + col;
                new_data[pos * 4..(pos + 1) * 4].clone_from_slice(&data[old_pos * 4..(old_pos + 1) * 4]);
            }
        }

        new_data
    }

    pub fn draw(&self, sprite_ref: SpriteReference, x: i32, y: i32, scale: f32, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        assert_eq!(self.identifier, sprite_ref.atlas);
        let sprite = &self.sprites[sprite_ref.idx as usize];
        let (ref blob, width) = self.blobs[sprite.blob_idx];
        // Safety: this object will not live longer than this function, and the blob reference
        // is all but static. No idea about the line depth stuff.
        let resized_width = ((sprite.width as f32) * scale) as usize;
        let resized_height = ((sprite.height as f32) * scale) as usize;
        let resized = resized_sprite_cache.entry(sprite_ref).or_insert_with(||Self::resize(&blob[(sprite.x * 4 + sprite.y * 4 * width as u32) as usize..], width as usize, sprite.width as usize, sprite.height as usize, resized_width, resized_height));
        // let mut view = unsafe {
        //     fltk::image::RgbImage::from_data2(
        //         &blob[(sprite.x * 4 + sprite.y * 4 * *width as u32) as usize..],
        //         sprite.width as i32,
        //         sprite.height as i32,
        //         4,
        //         width * 4).unwrap()
        // };
        let mut view = unsafe {
            fltk::image::RgbImage::from_data2(
                &resized,
                resized_width as i32,
                resized_height as i32,
                4,
                (resized_width * 4) as i32).unwrap()
        };

        //view.scale((view.width() as f32 * scale) as i32, (view.height() as f32 * scale) as i32, true, true);
        view.draw(x, y, view.width(), view.height());
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

pub fn load_data_file(data_file: path::PathBuf) -> Result<(Vec<u8>, i32), io::Error> {
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

    return Ok((buf, width as i32));
}