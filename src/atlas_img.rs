use std::cell::{Ref, RefCell};
use std::sync::Mutex;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::{ErrorKind, Read};
use std::path;
use std::path::PathBuf;
use femtovg::{ImageId, ImageSource, Paint, Path, Color};
use imgref::Img;
use rgb::RGBA8;

use crate::autotiler::TileReference;
use crate::units::*;
use crate::config::walker::ConfigSource;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
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
    blobs: Vec<Mutex<BlobData>>,
    sprites_map: HashMap<String, usize>,
    sprites: Vec<AtlasSprite>,
}

#[derive(Debug)]
pub struct AtlasSprite {
    blob_idx: usize,
    bounding_box: Rect<u16, UnknownUnit>,
    trim_offset: Vector2D<i16, UnknownUnit>,
    untrimmed_size: Size2D<u16, UnknownUnit>,
}

impl Atlas {
    pub fn load<T: ConfigSource>(config: &mut T, atlas: &str) -> Atlas {
        let mut result = Atlas {
            blobs: Vec::new(),
            sprites_map: HashMap::new(),
            sprites: Vec::new(),
        };

        result.load_crunched(config, atlas).expect("Fatal error parsing packed atlas");

        result
    }

    fn load_crunched<T: ConfigSource>(&mut self, config: &mut T, atlas: &str) -> Result<(), io::Error> {
        let meta_file = PathBuf::from("Graphics/Atlases").join(atlas.to_owned() + ".meta");
        let mut reader = if let Some(fp) = config.get_file(&meta_file) { fp } else { return Ok(()) };

        // this code ripped shamelessly from ahorn
        let _version = reader.read_u32::<LittleEndian>()?;
        let _cmd = read_string(&mut reader)?;
        let _checksum = reader.read_u32::<LittleEndian>()?;

        let count = reader.read_u16::<LittleEndian>()?;
        for _ in 0..count {
            let data_file = read_string(&mut reader)? + ".data";
            let data_path = meta_file.with_file_name(&data_file);
            let blob_idx = self.blobs.len();
            self.blobs.push(Mutex::new(BlobData::Waiting(load_data_file(config, data_path)?)));

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

                let sprite_idx = self.sprites.len();
                self.sprites_map.insert(sprite_path, sprite_idx);
                self.sprites.push(AtlasSprite {
                    blob_idx,
                    bounding_box: euclid::Rect {
                        origin: euclid::Point2D::new(x, y),
                        size: euclid::Size2D::new(width, height),
                    },
                    trim_offset: Vector2D::new(offset_x, offset_y),
                    untrimmed_size: Size2D::new(real_width, real_height),
                });
            }
        }

        Ok(())
    }

    pub fn iter_paths(&self) -> impl Iterator<Item = &str> {
        self.sprites_map.iter().map(|x| x.0.as_str())
    }

    pub fn lookup(&self, path: &str) -> Option<SpriteReference> {
        let path = path.replace("\\", "/");
        self.sprites_map.get(&path)
            .map(|v| SpriteReference{
            atlas: 0,
            idx: *v as u32,
        })
    }

    pub fn sprite_dimensions(&self, sprite_ref: SpriteReference) -> Size2D<u16, UnknownUnit> {
        let sprite = &self.sprites[sprite_ref.idx as usize];
        sprite.untrimmed_size
    }

    pub fn draw_sprite(
        &self,
        canvas: &mut vizia::Canvas,
        sprite_ref: SpriteReference,
        point: Point2D<f32, UnknownUnit>,
        slice: Option<Rect<f32, UnknownUnit>>,
        justify: Option<Vector2D<f32, UnknownUnit>>,
        scale: Option<Point2D<f32, UnknownUnit>>,
        color: Option<Color>,
        rot: f32,
    ) {
        let sprite = &self.sprites[sprite_ref.idx as usize];
        let color = color.unwrap_or(Color::white().into());

        let justify = justify.unwrap_or(Vector2D::new(0.5, 0.5));
        let slice = slice.unwrap_or(Rect::new(Point2D::zero(), sprite.untrimmed_size.cast()));
        let scale = scale.unwrap_or(Point2D::new(1.0, 1.0));

        // what atlas-space point does the screen-space point specified correspond to in the atlas?
        // if point is cropped then we give a point outside the crop. idgaf
        let atlas_origin = sprite.bounding_box.origin.cast() + sprite.trim_offset;
        let justify_offset = slice.origin.to_vector() + slice.size.cast().to_vector().component_mul(justify);
        let atlas_center = atlas_origin.cast() + justify_offset;
        // we draw so atlas_center corresponds to point

        // what canvas-space bounds should we clip to?
        let slice_visible = slice.intersection(&Rect::new(-sprite.trim_offset.cast::<f32>().to_point(), sprite.bounding_box.size.cast()));
        let slice_visible = if let Some(slice_visible) = slice_visible { slice_visible } else { return };
        let canvas_rect = slice_visible.translate(-justify_offset).scale(scale.x, scale.y); //.translate(point.to_vector());

        // how do we transform the entire fucking atlas to get the rectangle we want to end up inside canvas_rect?
        let atlas_offset = -atlas_center.to_vector().component_mul(scale.to_vector());

        let mut image_blob = self.blobs[sprite.blob_idx].lock().unwrap();
        let image_id = image_blob.image_id(canvas);
        let (sx, sy) = canvas.image_size(image_id).unwrap();
        let paint = Paint::image_tint(
            image_id,
            atlas_offset.x, atlas_offset.y,
            sx as f32 * scale.x, sy as f32 * scale.y,
            0.0, color
        );
        let mut path = Path::new();
        path.rect(
            canvas_rect.min_x(),
            canvas_rect.min_y(),
            canvas_rect.width(),
            canvas_rect.height(),
        );
        canvas.save();
        canvas.translate(point.x, point.y);
        canvas.rotate(rot.to_radians());
        canvas.fill_path(&mut path, paint);
        canvas.restore();
    }

    pub fn draw_tile(
        &self,
        canvas: &mut vizia::Canvas,
        tile_ref: TileReference,
        ox: f32,
        oy: f32,
        color: femtovg::Color
    ) {
        self.draw_sprite(
            canvas,
            tile_ref.texture,
            Point2D::new(ox, oy),
            Some(Rect::new(
                Point2D::new(tile_ref.tile.x as f32 * 8.0, tile_ref.tile.y as f32 * 8.0),
                Size2D::new(8.0, 8.0)
            )),
            Some(Vector2D::new(0.0, 0.0)),
            None,
            Some(color),
            0.0,
        );
    }
}

fn read_string<R: Read>(reader: &mut R) -> Result<String, io::Error> {
    let strlen = reader.read_u8()? as usize;
    let mut buf = vec![0u8; strlen];
    reader.read_exact(buf.as_mut_slice())?;

    String::from_utf8(buf).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8")
    })
}

pub fn load_data_file<T: ConfigSource>(config: &mut T, data_file: path::PathBuf) -> Result<Img<Vec<RGBA8>>, io::Error> {
    let mut reader = if let Some(reader) = config.get_file(&data_file) {
        reader
    } else {
        return Err(io::Error::new(ErrorKind::NotFound, format!("{:?} not found", data_file)))
    };

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

pub struct MultiAtlas<'a> {
    atlases: Vec<&'a Atlas>,
    path_cache: Mutex<HashMap<String, Option<SpriteReference>>>,
}

impl<'a> MultiAtlas<'a> {
    pub fn new() -> Self {
        Self {
            atlases: vec![],
            path_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn add(&mut self, atlas: &'a Atlas) {
        self.atlases.push(atlas);
    }

    pub fn iter_paths(&self) -> impl Iterator<Item = &str> {
        self.atlases.iter()
            .map(|a| a.iter_paths())
            .flatten()
    }

    pub fn lookup(&self, path: &str) -> Option<SpriteReference> {
        *self.path_cache.lock().unwrap().entry(path.to_owned())
            .or_insert_with(|| {
                for (i, atlas) in self.atlases.iter().enumerate() {
                    if let Some(mut sprite) = atlas.lookup(path) {
                        sprite.atlas = i as u32;
                        return Some(sprite);
                    }
                }
                None
            })
    }

    pub fn sprite_dimensions(&self, sprite: SpriteReference) -> Size2D<u16, UnknownUnit> {
        self.atlases.get(sprite.atlas as usize).unwrap().sprite_dimensions(sprite)
    }

    pub fn draw_sprite(
        &self,
        canvas: &mut vizia::Canvas,
        sprite_ref: SpriteReference,
        point: Point2D<f32, UnknownUnit>,
        slice: Option<Rect<f32, UnknownUnit>>,
        justify: Option<Vector2D<f32, UnknownUnit>>,
        scale: Option<Point2D<f32, UnknownUnit>>,
        color: Option<Color>,
        rot: f32,
    ) {
        self.atlases.get(sprite_ref.atlas as usize).unwrap().draw_sprite(
            canvas, sprite_ref, point,
            slice, justify, scale, color, rot
        );
    }

    pub fn draw_tile(
        &self,
        canvas: &mut vizia::Canvas,
        tile_ref: TileReference,
        ox: f32,
        oy: f32,
        color: femtovg::Color
    ) {
        self.atlases.get(tile_ref.texture.atlas as usize).unwrap().draw_tile(
            canvas, tile_ref, ox, oy, color
        );
    }
}
