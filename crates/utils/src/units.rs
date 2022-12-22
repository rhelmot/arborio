pub use euclid::{Angle, Point2D, Rect, Size2D, Transform2D, UnknownUnit, Vector2D};
use serde::{Deserialize, Serialize};

pub struct TileSpace;
pub struct RoomSpace;
pub struct MapSpace;
pub struct ScreenSpace;

pub type TileRect = Rect<i32, TileSpace>;
pub type TilePoint = Point2D<i32, TileSpace>;
pub type TileSize = Size2D<i32, TileSpace>;
pub type TileVector = Vector2D<i32, TileSpace>;

pub type RoomRect = Rect<i32, RoomSpace>;
pub type RoomPoint = Point2D<i32, RoomSpace>;
pub type RoomSize = Size2D<i32, RoomSpace>;
pub type RoomVector = Vector2D<i32, RoomSpace>;

pub type MapRectStrict = Rect<i32, MapSpace>;
pub type MapRectPrecise = Rect<f32, MapSpace>;
pub type MapPointStrict = Point2D<i32, MapSpace>;
pub type MapPointPrecise = Point2D<f32, MapSpace>;
pub type MapSizeStrict = Size2D<i32, MapSpace>;
pub type MapSizePrecise = Size2D<f32, MapSpace>;
pub type MapVectorStrict = Vector2D<i32, MapSpace>;
pub type MapVectorPrecise = Vector2D<f32, MapSpace>;

pub type ScreenRect = Rect<f32, ScreenSpace>;
pub type ScreenPoint = Point2D<f32, ScreenSpace>;
pub type ScreenSize = Size2D<f32, ScreenSpace>;
pub type ScreenVector = Vector2D<f32, ScreenSpace>;

pub type MapToScreen = Transform2D<f32, MapSpace, ScreenSpace>;

pub fn point_tile_to_room(pt: &TilePoint) -> RoomPoint {
    (*pt * 8).cast_unit()
}
pub fn point_room_to_tile(pt: &RoomPoint) -> TilePoint {
    (*pt / 8).cast_unit()
}
pub fn size_tile_to_room(pt: &TileSize) -> RoomSize {
    (*pt * 8).cast_unit()
}
pub fn size_room_to_tile(pt: &RoomSize) -> TileSize {
    (*pt / 8).cast_unit()
}
pub fn vector_tile_to_room(pt: &TileVector) -> RoomVector {
    (*pt * 8).cast_unit()
}
pub fn vector_room_to_tile(pt: &RoomVector) -> TileVector {
    (*pt / 8).cast_unit()
}
pub fn rect_tile_to_room(pt: &TileRect) -> RoomRect {
    (*pt * 8).cast_unit()
}
pub fn rect_room_to_tile(pt: &RoomRect) -> TileRect {
    (*pt / 8).cast_unit()
}

pub fn point_lose_precision(pt: &MapPointPrecise) -> MapPointStrict {
    MapPointStrict::new(pt.x.floor() as i32, pt.y.floor() as i32)
}

pub struct RectPointIter<T, U> {
    rect: Rect<T, U>,
    step: Vector2D<T, U>,
    next_pt: Option<Point2D<T, U>>,
}

impl<T, U> Iterator for RectPointIter<T, U>
where
    T: core::ops::Add<Output = T> + Copy + PartialOrd,
{
    type Item = Point2D<T, U>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.next_pt;
        if let Some(mut next) = cur {
            next.x = next.x + self.step.x;
            if next.x >= self.rect.max_x() {
                next.x = self.rect.min_x();
                next.y = next.y + self.step.y;
                if next.y >= self.rect.max_y() {
                    self.next_pt = None;
                } else {
                    self.next_pt = Some(next);
                }
            } else {
                self.next_pt = Some(next);
            }
        }

        cur
    }
}

pub fn rect_point_iter<T, U>(rect: Rect<T, U>, step: T) -> RectPointIter<T, U>
where
    T: core::ops::Add<T> + Copy + PartialOrd,
{
    RectPointIter {
        rect,
        step: Vector2D::new(step, step),
        next_pt: Some(rect.origin),
    }
}

pub fn rect_point_iter2<T, U>(rect: Rect<T, U>, step: Vector2D<T, U>) -> RectPointIter<T, U>
where
    T: core::ops::Add<T> + Copy + PartialOrd,
{
    RectPointIter {
        rect,
        step,
        next_pt: Some(rect.origin),
    }
}

pub fn rect_normalize<T, U>(rect: &Rect<T, U>) -> Rect<T, U>
where
    T: Copy
        + PartialOrd
        + euclid::num::Zero
        + std::ops::Add<T, Output = T>
        + std::ops::Neg<Output = T>
        + PartialOrd,
{
    Rect::new(
        Point2D::new(
            rect.origin.x
                + (if rect.size.width < T::zero() {
                    rect.size.width
                } else {
                    T::zero()
                }),
            rect.origin.y
                + (if rect.size.height < T::zero() {
                    rect.size.height
                } else {
                    T::zero()
                }),
        ),
        Size2D::new(shitty_abs(rect.size.width), shitty_abs(rect.size.height)),
    )
}

fn shitty_abs<T>(num: T) -> T
where
    T: Copy + euclid::num::Zero + std::ops::Neg<Output = T> + PartialOrd,
{
    if num < T::zero() {
        -num
    } else {
        num
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileGrid<T> {
    pub tiles: Vec<T>,
    pub stride: usize,
}

impl<T: Sized> TileGrid<T> {
    pub fn empty() -> Self {
        TileGrid {
            tiles: vec![],
            stride: 1,
        }
    }

    pub fn new(size: TileSize, fill: T) -> Self
    where
        T: Clone,
    {
        Self {
            tiles: vec![fill; (size.width * size.height) as usize],
            stride: size.width as usize,
        }
    }

    pub fn get(&self, pt: TilePoint) -> Option<&T> {
        if pt.x < 0 || pt.x >= self.stride as i32 || pt.y < 0 {
            None
        } else {
            self.tiles.get((pt.x + pt.y * self.stride as i32) as usize)
        }
    }

    pub fn get_mut(&mut self, pt: TilePoint) -> Option<&mut T> {
        if pt.x < 0 || pt.x >= self.stride as i32 || pt.y < 0 {
            None
        } else {
            self.tiles
                .get_mut((pt.x + pt.y * self.stride as i32) as usize)
        }
    }

    pub fn size(&self) -> TileSize {
        TileSize::new(self.stride as i32, (self.tiles.len() / self.stride) as i32)
    }

    pub fn resize(&mut self, size: TileSize, fill: T)
    where
        T: Clone,
    {
        // TODO: pick side to clamp to
        let old_stride = self.stride;
        let new_stride = size.width as usize;
        let min_stride = new_stride.min(self.stride);
        let old_lines = self.tiles.len() / self.stride;
        let new_lines = size.height as usize;
        let min_lines = new_lines.min(old_lines);

        let mut result = vec![fill; new_stride * new_lines];

        for line in (0..min_lines).rev() {
            for idx in (0..min_stride).rev() {
                std::mem::swap(
                    &mut self.tiles[line * old_stride + idx],
                    &mut result[line * new_stride + idx],
                );
            }
        }

        self.tiles = result;
        self.stride = new_stride;
    }
}

impl<T: Clone + Default + Sized> TileGrid<T> {
    pub fn get_or_default(&self, pt: TilePoint) -> T {
        self.get(pt).cloned().unwrap_or_default()
    }

    pub fn new_default(size: TileSize) -> Self {
        Self {
            tiles: vec![T::default(); (size.width * size.height) as usize],
            stride: size.width as usize,
        }
    }
}
