use crate::autotiler::TileReference;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use fltk::prelude::ImageExt;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::io;
use std::io::Read;
use std::path;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct SpriteReference {
    atlas: u32,
    idx: u32,
}

pub struct Atlas {
    identifier: u32,
    blobs: Vec<(Vec<u8>, u32)>,
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

pub struct RoomBuffer {
    pub line_width: u32,
    pub data: Box<[u8]>,
}
impl RoomBuffer {
    pub(crate) fn new(height: u32, width: u32) -> Self {
        let line_width = width * 4;
        Self {
            line_width,
            data: vec![0; line_width as usize * height as usize].into_boxed_slice(),
        }
    }
    fn width(&self) -> u32 {
        self.line_width / 4
    }
    fn height(&self) -> u32 {
        (self.data.len() / self.line_width as usize) as u32
    }
    pub(crate) fn draw(&self, x: i32, y: i32) {
        let mut view = unsafe {
            fltk::image::RgbImage::from_data2(
                &self.data,
                self.width() as i32,
                self.height() as i32,
                4,
                (self.line_width) as i32).unwrap()
        };

        view.draw(x, y, view.width(), view.height());
        //panic!("testing");
    }
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

    // This resize projects each resized pixel onto the source coordinates and sets it's color to the
    // weighted average of all of the source pixels it intersects with, weighted by the intersection area
    fn resize(data: &[u8], line_width: u32, width: u32, height: u32, new_width: u32, new_height: u32) -> Vec<u8> {
        let vertical_scale = new_height as f32 / height as f32;
        let horizontal_scale = new_width as f32 / width as f32;

        let mut new_data = vec![0_u8; (new_height * new_width * 4) as usize];
        for row in 0..new_height {
            for col in 0..new_width {

                let pixel = Self::composite_pixel(data, line_width as usize, row as usize, col as usize, vertical_scale, horizontal_scale);

                let pos = (row * new_width + col) as usize;
                // Write out destination pixel
                new_data[pos * 4..(pos + 1) * 4].clone_from_slice(&pixel);
            }
        }

        new_data
    }
    // Helper function for `resize`. It composites one pixel
    fn composite_pixel(data: &[u8], line_width: usize, row: usize, col: usize, vertical_scale: f32, horizontal_scale: f32) -> [u8; 4] {

        // The row range, in source coordinates that the resized pixel covers
        let source_row_start = row as f32 / vertical_scale;
        let source_row_end = (row + 1) as f32 / vertical_scale;

        // The column range, in source coordinates that the resized pixel covers
        let source_col_start = col as f32 / horizontal_scale;
        let source_col_end = (col + 1) as f32 / horizontal_scale;

        let mut pixel = [0_f32; 4];
        // For each row in the source that overlaps with the resized
        for source_row in
        (source_row_start.floor() as usize)..(source_row_end.ceil() as usize)
        {
            let vertical_overlap_start =
                f32::max(row as f32, source_row as f32 * vertical_scale);
            let vertical_overlap_end = f32::min(
                (row + 1) as f32,
                (source_row + 1) as f32 * vertical_scale,
            );
            // Calculate the fraction of the resized row that the current source row covers
            // This will be at most one, if it completely covers the resized row
            let vertical_overlap = vertical_overlap_end - vertical_overlap_start;
            // For each column in the source that overlaps with the resized
            for source_col in
            (source_col_start.floor() as usize)..(source_col_end.ceil() as usize)
            {
                let horizontal_overlap_start =
                    f32::max(col as f32, source_col as f32 * horizontal_scale);
                let horizontal_overlap_end = f32::min(
                    (col + 1) as f32,
                    (source_col + 1) as f32 * horizontal_scale,
                );
                // Calculate the fraction of the resized column that the current source column covers
                // This will be at most one, if it completely covers the resized column
                let horizontal_overlap = horizontal_overlap_end - horizontal_overlap_start;

                let overlap = vertical_overlap * horizontal_overlap;

                // Position in the source data to read a pixel from
                let source_pos = source_row * line_width + source_col;

                let source: &[_; 4] = &data[source_pos * 4..(source_pos + 1) * 4]
                    .try_into()
                    .unwrap();
                // Add source pixel into destination pixel, weighted by overlap area
                for i in 0..4 {
                    pixel[i] += source[i] as f32 * overlap;
                }
            }
        }

        // Convert destination pixel from f32s to u8s
        let rounded_pixel: [u8; 4] = array_init::array_init(|i| pixel[i].round() as u8);

        rounded_pixel
    }

    pub fn draw(&self, sprite_ref: SpriteReference, x: i32, y: i32, map_scale: u32, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        assert_eq!(self.identifier, sprite_ref.atlas);
        let sprite = &self.sprites[sprite_ref.idx as usize];
        let (ref blob, width) = self.blobs[sprite.blob_idx];
        let resized_width = sprite.width * map_scale / 8;
        let resized_height = sprite.height * map_scale / 8;
        let resized = resized_sprite_cache.entry(sprite_ref).or_insert_with(||Self::resize(&blob[(sprite.x * 4 + sprite.y * 4 * width as u32) as usize..], width as u32, sprite.width, sprite.height, resized_width, resized_height));
        // Safety: this object will not live longer than this function, and the blob reference
        // is all but static. No idea about the line depth stuff.
        let mut view = unsafe {
            fltk::image::RgbImage::from_data2(
                &resized,
                resized_width as i32,
                resized_height as i32,
                4,
                (resized_width * 4) as i32).unwrap()
        };

        view.draw(x, y, view.width(), view.height());
    }

    pub fn draw_tile(&self, tile_ref: TileReference, x: u32, y: u32, map_scale: u32, destination: &mut RoomBuffer, resized_sprite_cache: &mut HashMap<SpriteReference, Vec<u8>>) {
        assert_eq!(self.identifier, tile_ref.texture.atlas);
        let sprite = &self.sprites[tile_ref.texture.idx as usize];
        let (ref blob, width) = self.blobs[sprite.blob_idx];
        let resized_width = sprite.width * map_scale / 8;
        let resized_height = sprite.height * map_scale / 8;
        let resized = resized_sprite_cache.entry(tile_ref.texture).or_insert_with(||Self::resize(&blob[(sprite.x * 4 + sprite.y * 4 * width as u32) as usize..], width as u32, sprite.width, sprite.height, resized_width, resized_height));
        for row in 0..map_scale {
            let source = (tile_ref.tile.x * 4 * map_scale + tile_ref.tile.y * resized_width * 4 * map_scale + row * resized_width * 4) as usize;
            let dest = (x * 4 + (y + row) * destination.line_width) as usize;

            destination.data[dest..dest + map_scale as usize * 4].clone_from_slice(&resized[source..source + map_scale as usize * 4]);
        }
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

pub fn load_data_file(data_file: path::PathBuf) -> Result<(Vec<u8>, u32), io::Error> {
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

    return Ok((buf, width));
}