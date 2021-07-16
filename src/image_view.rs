use fltk::prelude::*;
use crate::map_struct::Rect;
use std::convert::TryInto;
use core::fmt;
use std::fmt::Formatter;

pub struct ImageBuffer {
    line_width: u32,
    data: Box<[u8]>,
}

impl fmt::Debug for ImageBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("ImageBuffer {{ line_width: {}, data: Box([_; {}]) }}", self.line_width, self.data.len()))
    }
}

impl ImageBuffer {
    pub(crate) fn new(height: u32, width: u32) -> Self {
        Self {
            data: vec![0; (height * width) as usize * 4].into_boxed_slice(),
            line_width: width * 4,
        }
    }
    pub(crate) fn from_vec(data: Vec<u8>, line_width: u32) -> ImageBuffer {
        assert_eq!(line_width % 4, 0);
        assert!(data.len() == 0 || data.len() % line_width as usize == 0);
        Self {
            line_width,
            data: data.into_boxed_slice(),
        }
    }
    pub(crate) fn height(&self) -> u32 {
        (self.data.len() as u32).checked_div(self.line_width).unwrap_or(0)
    }
    pub(crate) fn width(&self) -> u32 {
        self.line_width / 4
    }
    pub(crate) fn draw_clipped(&self, bounding_box: &Rect, x: i32, y: i32) {
        self.as_ref().draw_clipped(bounding_box, x, y);
    }
    pub(crate) fn as_ref(&self) -> ImageView {
        ImageView {
            height: self.height(),
            width: self.line_width / 4,
            line_width: self.line_width,
            buffer: &self.data,
        }
    }
    pub(crate) fn as_mut(&mut self) -> ImageViewMut {
        ImageViewMut {
            height: self.height(),
            width: self.line_width / 4,
            line_width: self.line_width,
            buffer: &mut self.data,
        }
    }
    pub(crate) fn subsection(&self, section: &Rect) -> ImageView {
        self.as_ref().subsection(section)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ImageView<'a> {
    height: u32,
    width: u32,
    line_width: u32,
    buffer: &'a [u8],
}

impl <'a> ImageView<'a> {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub(crate) fn draw(self, x: i32, y: i32) {
        debug_assert_eq!(self.line_width * (self.height - 1) + self.width * 4, self.buffer.len() as u32);
        let mut view = unsafe {
            fltk::image::RgbImage::from_data2(
                self.buffer,
                self.width as i32,
                self.height as i32,
                4,
                (self.line_width) as i32).unwrap()
        };

        view.draw(x, y, self.width as i32, self.height as i32);
    }
    pub(crate) fn draw_clipped(self, clip_box: &Rect, x: i32, y: i32) {
        assert!(clip_box.x >= 0 && clip_box.y >= 0);
        let clipped_x = x.max(clip_box.x);
        let clipped_width = (x + self.width as i32).min(clip_box.x + clip_box.width as i32) - clipped_x;
        let clipped_y = y.max(clip_box.y);
        let clipped_height = (y + self.height as i32).min(clip_box.y + clip_box.height as i32) - clipped_y;
        if clipped_width <= 0 || clipped_height <= 0 {
            return;
        }

        self.subsection(&Rect {
            x: clipped_x - x,
            y: clipped_y - y,
            width: clipped_width as u32,
            height: clipped_height as u32,
        }).draw(clipped_x, clipped_y);
    }
    pub(crate) fn draw_tiled(&self, bounds: &Rect, tile_width: u32, tile_height: u32) {
        bounds.tile(tile_width, tile_height, |mut r: Rect| {
            let draw_x = r.x;
            let draw_y = r.y;
            if r.x < 0 {
                r.x = 0;
            }
            if r.y < 0 {
                r.y = 0;
            }
            self.draw_clipped(&r, draw_x, draw_y);
        })
    }
    pub(crate) fn draw_to(self, destination: ImageViewMut, x: u32, y: u32) {
        assert!(self.width + x <= destination.width);
        assert!(self.height + y <= destination.height);
        for row in 0..self.height {
            let source = (row * self.line_width) as usize;
            let dest = (x * 4 + (y + row) * destination.line_width) as usize;

            destination.buffer[dest..dest + self.width as usize * 4].clone_from_slice(&self.buffer[source..source + self.width as usize * 4]);
        }
    }
    pub(crate) fn from_buffer(buffer: &'a [u8], line_width: u32) -> Self {
        assert_eq!(buffer.len() % line_width as usize, 0);
        assert_eq!(line_width % 4, 0);
        Self {
            height: buffer.len() as u32 / line_width,
            width: line_width / 4,
            line_width,
            buffer,
        }
    }
    pub(crate) fn subsection(&self, section: &Rect) -> Self {
        debug_assert_eq!(self.line_width * (self.height - 1) + self.width * 4, self.buffer.len() as u32);
        assert!(section.x >= 0);
        assert!(section.y >= 0);
        let (x, y) = (section.x as u32, section.y as u32);
        assert!(section.width + x <= self.width);
        assert!(section.height + y <= self.height);
        let buffer;
        if section.width > 0 && section.height > 0 {
            buffer = &self.buffer[(x * 4 + y * self.line_width) as usize..(x * 4 + section.width * 4 + (y + section.height - 1) * self.line_width) as usize];
            debug_assert_eq!(buffer.len(), (section.width * 4 + (section.height - 1) * self.line_width) as usize);
        } else {
            buffer = &[];
        }
        ImageView {
            height: section.height,
            width: section.width,
            line_width: self.line_width,
            buffer,
        }
    }
    pub(crate) fn to_owned(self) -> ImageBuffer {
        if self.width * 4 == self.line_width && (self.line_width * self.height) as usize == self.buffer.len() {
            ImageBuffer::from_vec(self.buffer.to_owned(), self.line_width)
        } else {
            let mut owned = ImageBuffer::new(self.height, self.width);
            self.draw_to(owned.as_mut(), 0, 0);
            owned
        }
    }
    // This resize projects each resized pixel onto the source coordinates and sets it's color to the
    // weighted average of all of the source pixels it intersects with, weighted by the intersection area
    pub(crate) fn resize(&self, new_scale: u32, old_scale: u32) -> ImageBuffer {
        debug_assert_eq!(self.line_width * (self.height - 1) + self.width * 4, self.buffer.len() as u32);
        let ImageView { width, height, .. } = *self;
        let new_width = self.width * new_scale / old_scale;
        let new_height = self.height * new_scale / old_scale;
        let line_width = new_width * 4;

        let mut new_data = vec![0_u8; (new_height * line_width) as usize];
        for row in 0..new_height {
            for col in 0..new_width {

                let pixel = self.composite_pixel(row as usize, col as usize, new_scale as usize, old_scale as usize);

                let pos = (row * line_width + col * 4) as usize;
                new_data[pos..pos + 4].clone_from_slice(&pixel);
            }
        }

        ImageBuffer::from_vec(new_data, line_width)
    }
    // Helper function for `resize`. It composites one pixel
    fn composite_pixel(&self, row: usize, col: usize, new_scale: usize, old_scale: usize) -> [u8; 4] {
        let data = self.buffer;
        let line_width = self.line_width;
        let line_count = data.len() / line_width as usize;

        // The row range, in source coordinates that the resized pixel covers
        let source_row_start = row * old_scale / new_scale;
        let source_row_end = ((row + 1) * old_scale - 1) / new_scale;

        // The column range, in source coordinates that the resized pixel covers
        let source_col_start = col * old_scale / new_scale;
        let source_col_end = ((col + 1) * old_scale - 1) / new_scale;

        let mut pixel = [0; 4];
        // For each row in the source that overlaps with the resized
        for source_row in source_row_start..=source_row_end
        {
            let vertical_overlap_start =
                usize::max(row * old_scale, source_row * new_scale);
            let vertical_overlap_end = usize::min(
                (row + 1) * old_scale,
                (source_row + 1) * new_scale
            );
            // Calculate the fraction of the resized row that the current source row covers
            // This will be at most one, if it completely covers the resized row
            let vertical_overlap = vertical_overlap_end - vertical_overlap_start;
            // For each column in the source that overlaps with the resized
            for source_col in source_col_start..=source_col_end
            {
                let horizontal_overlap_start =
                    usize::max(col * old_scale, source_col * new_scale);
                let horizontal_overlap_end = usize::min(
                    (col + 1) * old_scale,
                    (source_col + 1) * new_scale,
                );
                // Calculate the fraction of the resized column that the current source column covers
                // This will be at most one, if it completely covers the resized column
                let horizontal_overlap = horizontal_overlap_end - horizontal_overlap_start;

                let overlap = vertical_overlap * horizontal_overlap;

                // Position in the source data to read a pixel from
                let source_pos = source_row * line_width as usize + source_col * 4;

                let source: &[_; 4] = &data[source_pos..source_pos+4].try_into().unwrap();
                // Add source pixel into destination pixel, weighted by overlap area
                for i in 0..4 {
                    pixel[i] += source[i] as usize * overlap;
                }
            }
        }

        // Convert destination pixel from f32s to u8s
        let rounded_pixel: [u8; 4] = array_init::array_init(|i| (pixel[i] / old_scale / old_scale) as u8);

        rounded_pixel
    }
}
pub struct ImageViewMut<'a> {
    height: u32,
    width: u32,
    line_width: u32,
    buffer: &'a mut [u8],
}
impl<'a> ImageViewMut<'a> {
    pub(crate) fn as_ref(&self) -> ImageView {
        ImageView {
            height: self.height,
            width: self.width,
            line_width: self.line_width,
            buffer: &self.buffer,
        }
    }
    pub(crate) fn reborrow(&mut self) -> ImageViewMut {
        ImageViewMut {
            height: self.height,
            width: self.width,
            line_width: self.line_width,
            buffer: &mut self.buffer,
        }
    }
    pub(crate) fn subsection(self, section: &Rect) -> Self {
        debug_assert_eq!(self.line_width * (self.height - 1) + self.width * 4, self.buffer.len() as u32);
        assert!(section.x >= 0, "section.x = {}", section.x);
        assert!(section.y >= 0);
        let (x, y) = (section.x as u32, section.y as u32);
        assert!(section.width + x <= self.width);
        assert!(section.height + y <= self.height);
        let buffer;
        if section.width > 0 && section.height > 0 {
            buffer = &mut self.buffer[(x * 4 + y * self.line_width) as usize..(x * 4 + section.width * 4 + (y + section.height - 1) * self.line_width) as usize];
            debug_assert_eq!(buffer.len(), (section.width * 4 + (section.height - 1) * self.line_width) as usize);
        } else {
            buffer = &mut [];
        }
        ImageViewMut {
            height: section.height,
            width: section.width,
            line_width: self.line_width,
            buffer,
        }
    }
    pub(crate) fn clear(&mut self) {
        for row in 0..self.height {
            self.buffer[(row * self.line_width) as usize..(row * self.line_width + self.width * 4) as usize].fill(0);
        }
    }
}
