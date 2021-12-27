use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;
use std::path;
use femtovg::{ImageId, ImageSource, Paint};
use imgref::Img;
use rgb::RGBA8;

use crate::autotiler::TileReference;

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct SpriteReference {
    atlas: u32,
    idx: u32,
}

enum BlobData {
    Waiting(Img<Vec<RGBA8>>),
    Loaded(ImageId),
}

impl BlobData {
    fn image_id(&mut self, canvas: &mut vizia::Canvas) -> ImageId {
        match self {
            BlobData::Waiting(buf) => {
                let res = canvas.create_image(
                    buf.as_ref(),
                    femtovg::ImageFlags::NEAREST)
                    .unwrap();
                *self = BlobData::Loaded(res);
                res
            }
            BlobData::Loaded(res) => *res
        }
    }
}

pub struct Atlas {
    identifier: u32,
    blobs: Vec<BlobData>,
    sprites_map: HashMap<String, usize>,
    sprites: Vec<AtlasSprite>,
}

pub struct AtlasSprite {
    blob_idx: usize,
    bounding_box: euclid::Rect<u16, euclid::UnknownUnit>,
    offset_x: i16,
    offset_y: i16,
    real_width: u16,
    real_height: u16,
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
            result.blobs.push(BlobData::Waiting(load_data_file(data_path)?));

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
                    bounding_box: euclid::Rect {
                        origin: euclid::Point2D::new(x, y),
                        size: euclid::Size2D::new(width, height),
                    },
                    offset_x, offset_y,
                    real_width, real_height,
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

    pub fn sprite_dimensions(&self, sprite_ref: SpriteReference) -> euclid::Size2D<u16, euclid::UnknownUnit> {
        let sprite = &self.sprites[sprite_ref.idx as usize];
        sprite.bounding_box.size
    }

    pub fn sprite_paint(&mut self, sprite_ref: SpriteReference, canvas: &mut vizia::Canvas) -> Paint {
        let sprite = &self.sprites[sprite_ref.idx as usize];
        let image_blob = &mut self.blobs[sprite.blob_idx];
        Paint::image(
            image_blob.image_id(canvas),
            sprite.bounding_box.origin.x as f32,
            sprite.bounding_box.origin.y as f32,
            sprite.bounding_box.width() as f32,
            sprite.bounding_box.height() as f32,
            0.0, 0.0
        )
    }

    pub fn tile_paint(&mut self, tile_ref: TileReference, canvas: &mut vizia::Canvas) -> Paint {
        let sprite = &self.sprites[tile_ref.texture.idx as usize];
        let image_blob = &mut self.blobs[sprite.blob_idx];
        Paint::image(
            image_blob.image_id(canvas),
            (sprite.bounding_box.origin.x as u32 + tile_ref.tile.x * 8) as f32,
            (sprite.bounding_box.origin.y as u32 + tile_ref.tile.y * 8) as f32,
            8.0, 8.0,
            0.0, 0.0
        )
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

pub fn load_data_file(data_file: path::PathBuf) -> Result<Img<Vec<RGBA8>>, io::Error> {
    let fp = fs::File::open(data_file)?;

    let mut reader = io::BufReader::new(fp);

    let width = reader.read_u32::<LittleEndian>()?;
    let height = reader.read_u32::<LittleEndian>()?;
    let has_alpha = reader.read_u8()? != 0;

    let total_px = (width * height) as usize;
    let mut current_px: usize = 0;
    let mut buf: Vec<RGBA8> = vec![RGBA8::new(0, 0, 0, 0); total_px];

    while current_px < total_px {
        let repeat = reader.read_u8()?;
        let repeat = if repeat > 0 { repeat - 1 } else { 0 } as usize; // this is off-by-one from the julia code because it's more ergonomic
        let alpha = if has_alpha {
            reader.read_u8()?
        } else {
            255
        };
        if alpha > 0 {
            let mut px = [0u8; 3];
            reader.read_exact(&mut px)?;
            buf[current_px] = RGBA8::new(px[2], px[1], px[0], alpha);
        }
        // no else case needed: they're already zeros

        if repeat > 0 {
            let src = buf[current_px].clone();
            for dst_idx in 1..=repeat {
                buf[current_px + dst_idx] = src.clone();
            }
        }

        current_px += repeat + 1;
    }
    Ok(Img::new(buf, width as usize, height as usize))
}
