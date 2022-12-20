use byteorder::{LittleEndian, ReadBytesExt};
use image::{DynamicImage, GenericImageView, ImageFormat};
use imgref::Img;
use rgb::RGBA8;
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::io;
use std::io::Read; // trait method import
use std::path;
use std::sync::{Arc, Mutex};

use arborio_utils::interned::{intern_owned, Interned, InternedMap};
use arborio_utils::units::*;
use arborio_utils::vizia::prelude::Canvas;
use arborio_utils::vizia::vg::{Color, ImageFlags, ImageId, ImageSource, Paint, Path};
use arborio_walker::{ConfigSource, ConfigSourceTrait};

use crate::autotiler::TileReference;

#[derive(Debug)]
enum BlobData {
    Waiting(Img<Vec<RGBA8>>),
    WaitingEncoded(DynamicImage),
    Loaded(ImageId),
}

impl BlobData {
    fn image_id(&mut self, canvas: &mut Canvas) -> ImageId {
        match self {
            BlobData::Waiting(buf) => {
                let res = canvas
                    .create_image(buf.as_ref(), ImageFlags::NEAREST)
                    .unwrap();
                *self = BlobData::Loaded(res);
                res
            }
            BlobData::WaitingEncoded(dat) => {
                let res = canvas
                    .create_image(
                        ImageSource::try_from(dat.borrow()).unwrap(),
                        ImageFlags::NEAREST,
                    )
                    .unwrap();
                *self = BlobData::Loaded(res);
                res
            }
            BlobData::Loaded(res) => *res,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Atlas {
    blobs: Vec<Arc<Mutex<BlobData>>>, // TODO: we can get rid of this mutex (and BlobData altogether) if we can somehow push image data into opengl at load time
    pub sprites_map: InternedMap<Arc<AtlasSprite>>,
}

#[derive(Debug)]
pub struct AtlasSprite {
    blob: Arc<Mutex<BlobData>>,
    bounding_box: Rect<u16, UnknownUnit>,
    trim_offset: Vector2D<i16, UnknownUnit>,
    untrimmed_size: Size2D<u16, UnknownUnit>,
}

impl Atlas {
    pub fn load(&mut self, config: &mut ConfigSource, atlas: &str) {
        if let Err(e) = self.load_crunched(config, atlas) {
            log::error!(
                "Failed loading crunched atlas {} of {}: {}",
                atlas,
                config,
                e
            );
        }

        for path in config.list_all_files(&path::PathBuf::from("Graphics/Atlases").join(atlas)) {
            if path.extension().and_then(|ext| ext.to_str()) == Some("png") {
                if let Err(e) = self.load_loose(config, atlas, &path) {
                    log::error!(
                        "Failed loading image {} of {}: {}",
                        path.display(),
                        config,
                        e
                    );
                }
            }
        }
    }

    fn load_loose(
        &mut self,
        config: &mut ConfigSource,
        atlas: &str,
        path: &path::Path,
    ) -> Result<(), io::Error> {
        let reader = if let Some(fp) = config.get_file(path) {
            fp
        } else {
            return Err(io::ErrorKind::NotFound.into());
        };

        let format = match path.extension().and_then(OsStr::to_str) {
            Some("jpg" | "jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("gif") => ImageFormat::Gif,
            _ => todo!(),
        };

        let img = image::load(reader, format)
            .map_err(|_| -> io::Error { io::ErrorKind::InvalidData.into() })?;

        let (width, height) = img.dimensions();
        let sprite_path = path
            .strip_prefix(&path::PathBuf::from("Graphics/Atlases").join(atlas))
            .unwrap()
            .with_extension("");
        let sprite_path = sprite_path
            .into_os_string()
            .into_string()
            .expect("Non-unicode asset path");

        self.blobs
            .push(Arc::new(Mutex::new(BlobData::WaitingEncoded(img))));
        self.sprites_map.insert(
            intern_owned(sprite_path),
            Arc::new(AtlasSprite {
                blob: self.blobs[self.blobs.len() - 1].clone(),
                bounding_box: Rect {
                    origin: Point2D::new(0, 0),
                    size: Size2D::new(width as u16, height as u16),
                },
                trim_offset: Vector2D::new(0, 0),
                untrimmed_size: Size2D::new(width as u16, height as u16),
            }),
        );

        Ok(())
    }

    fn load_crunched(&mut self, config: &mut ConfigSource, atlas: &str) -> Result<(), io::Error> {
        let meta_file = path::PathBuf::from("Graphics/Atlases").join(atlas.to_owned() + ".meta");
        let mut reader = if let Some(fp) = config.get_file(&meta_file) {
            fp
        } else {
            return Ok(());
        };

        // this code ripped shamelessly from ahorn
        let _version = reader.read_u32::<LittleEndian>()?;
        let _cmd = read_string(&mut reader)?;
        let _checksum = reader.read_u32::<LittleEndian>()?;

        let count = reader.read_u16::<LittleEndian>()?;
        for _ in 0..count {
            let data_file = read_string(&mut reader)? + ".data";
            let data_path = meta_file.with_file_name(data_file);
            self.blobs
                .push(Arc::new(Mutex::new(BlobData::Waiting(load_data_file(
                    config, data_path,
                )?))));

            let sprites = reader.read_u16::<LittleEndian>()?;
            for _ in 0..sprites {
                let sprite_path = read_string(&mut reader)?.replace('\\', "/");
                let x = reader.read_u16::<LittleEndian>()?;
                let y = reader.read_u16::<LittleEndian>()?;
                let width = reader.read_u16::<LittleEndian>()?;
                let height = reader.read_u16::<LittleEndian>()?;
                let offset_x = reader.read_i16::<LittleEndian>()?;
                let offset_y = reader.read_i16::<LittleEndian>()?;
                let real_width = reader.read_u16::<LittleEndian>()?;
                let real_height = reader.read_u16::<LittleEndian>()?;

                self.sprites_map.insert(
                    sprite_path.into(),
                    Arc::new(AtlasSprite {
                        blob: self.blobs[self.blobs.len() - 1].clone(),
                        bounding_box: Rect {
                            origin: Point2D::new(x, y),
                            size: Size2D::new(width, height),
                        },
                        trim_offset: Vector2D::new(offset_x, offset_y),
                        untrimmed_size: Size2D::new(real_width, real_height),
                    }),
                );
            }
        }

        Ok(())
    }
}

fn read_string<R: io::Read>(reader: &mut R) -> Result<String, io::Error> {
    let strlen = reader.read_u8()? as usize;
    let mut buf = vec![0u8; strlen];
    reader.read_exact(buf.as_mut_slice())?;

    String::from_utf8(buf).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid utf8"))
}

pub fn load_data_file(
    config: &mut ConfigSource,
    data_file: path::PathBuf,
) -> Result<Img<Vec<RGBA8>>, io::Error> {
    let mut reader = if let Some(reader) = config.get_file(&data_file) {
        reader
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{:?} not found", data_file),
        ));
    };

    let width = reader.read_u32::<LittleEndian>()?;
    let height = reader.read_u32::<LittleEndian>()?;
    let has_alpha = reader.read_u8()? != 0;

    let total_px = (width * height) as usize;
    let mut current_px: usize = 0;
    let mut buf: Vec<RGBA8> = vec![RGBA8::new(0, 0, 0, 0); total_px];

    while current_px < total_px {
        let repeat = reader.read_u8()? as usize;
        debug_assert_ne!(repeat, 0);
        let alpha = if has_alpha { reader.read_u8()? } else { 255 };
        if alpha > 0 {
            let mut px = [0u8; 3];
            reader.read_exact(&mut px)?;
            let pixel = RGBA8::new(px[2], px[1], px[0], alpha);

            buf[current_px..][..repeat].fill(pixel);
        }
        // no else case needed: they're already zeros

        current_px += repeat;
    }
    Ok(Img::new(buf, width as usize, height as usize))
}

#[derive(Clone)]
pub struct MultiAtlas {
    sprites_map: InternedMap<Arc<AtlasSprite>>,
}

impl MultiAtlas {
    pub fn from(sprites_map: InternedMap<Arc<AtlasSprite>>) -> Self {
        Self { sprites_map }
    }

    pub fn iter_paths(&self) -> impl Iterator<Item = &Interned> + '_ {
        self.sprites_map.keys()
    }

    pub fn sprite_dimensions(&self, sprite_path: &str) -> Option<Size2D<u16, UnknownUnit>> {
        self.sprites_map
            .get(sprite_path.replace('\\', "/").as_str())
            .map(|s| s.untrimmed_size)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite(
        &self,
        canvas: &mut Canvas,
        sprite_path: &str,
        point: Point2D<f32, UnknownUnit>,
        slice: Option<Rect<f32, UnknownUnit>>,
        justify: Option<Vector2D<f32, UnknownUnit>>,
        scale: Option<Point2D<f32, UnknownUnit>>,
        color: Option<Color>,
        rot: f32,
    ) -> Result<(), String> {
        let sprite = match self
            .sprites_map
            .get(sprite_path.replace('\\', "/").as_str())
        {
            None => {
                return Err(format!("No such texture: {}", sprite_path));
            }
            Some(sprite) => sprite,
        };
        let color = color.unwrap_or_else(Color::white);

        let justify = justify.unwrap_or_else(|| Vector2D::new(0.5, 0.5));
        let slice =
            slice.unwrap_or_else(|| Rect::new(Point2D::zero(), sprite.untrimmed_size.cast()));
        let scale = scale.unwrap_or_else(|| Point2D::new(1.0, 1.0));

        // what atlas-space point does the screen-space point specified correspond to in the atlas?
        // if point is cropped then we give a point outside the crop. idgaf
        let atlas_origin = sprite.bounding_box.origin.cast() + sprite.trim_offset;
        let justify_offset =
            slice.origin.to_vector() + slice.size.cast().to_vector().component_mul(justify);
        let atlas_center = atlas_origin.cast() + justify_offset;
        // we draw so atlas_center corresponds to point

        // what canvas-space bounds should we clip to?
        let slice_visible = slice.intersection(&Rect::new(
            -sprite.trim_offset.cast::<f32>().to_point(),
            sprite.bounding_box.size.cast(),
        ));
        let slice_visible = if let Some(slice_visible) = slice_visible {
            slice_visible
        } else {
            return Ok(());
        };
        let canvas_rect = slice_visible
            .translate(-justify_offset)
            .scale(scale.x, scale.y); //.translate(point.to_vector());

        // how do we transform the entire fucking atlas to get the rectangle we want to end up inside canvas_rect?
        let atlas_offset = -atlas_center.to_vector().component_mul(scale.to_vector());

        let mut image_blob = sprite.blob.lock().unwrap();
        let image_id = image_blob.image_id(canvas);
        let (sx, sy) = canvas.image_size(image_id).unwrap();
        let paint = Paint::image_tint(
            image_id,
            atlas_offset.x,
            atlas_offset.y,
            sx as f32 * scale.x,
            sy as f32 * scale.y,
            0.0,
            color,
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
        canvas.fill_path(&mut path, &paint);
        canvas.restore();

        Ok(())
    }

    pub fn draw_tile(
        &self,
        canvas: &mut Canvas,
        tile_ref: TileReference,
        ox: f32,
        oy: f32,
        color: Color,
    ) -> Result<(), String> {
        self.draw_sprite(
            canvas,
            &tile_ref.texture,
            Point2D::new(ox, oy),
            Some(Rect::new(
                Point2D::new(tile_ref.tile.x as f32 * 8.0, tile_ref.tile.y as f32 * 8.0),
                Size2D::new(8.0, 8.0),
            )),
            Some(Vector2D::new(0.0, 0.0)),
            None,
            Some(color),
            0.0,
        )
    }
}
