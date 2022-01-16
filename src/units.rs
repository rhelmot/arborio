use euclid::*;
pub use euclid::{Rect, Point2D, Size2D, Vector2D, UnknownUnit};

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

pub type MapRectStrict =  Rect<i32, MapSpace>;
pub type MapRectPrecise = Rect<f32, MapSpace>;
pub type MapPointStrict =  Point2D<i32, MapSpace>;
pub type MapPointPrecise = Point2D<f32, MapSpace>;
pub type MapSizeStrict =  Size2D<i32, MapSpace>;
pub type MapSizePrecise = Size2D<f32, MapSpace>;
pub type MapVectorStrict =  Vector2D<i32, MapSpace>;
pub type MapVectorPrecise = Vector2D<f32, MapSpace>;

pub type ScreenRect = Rect<f32, ScreenSpace>;
pub type ScreenPoint = Point2D<f32, ScreenSpace>;
pub type ScreenSize = Size2D<f32, ScreenSpace>;
pub type ScreenVector = Vector2D<f32, ScreenSpace>;

pub type MapToScreen = Transform2D<f32, MapSpace, ScreenSpace>;

pub fn point_tile_to_room(pt: &TilePoint) -> RoomPoint { (*pt * 8).cast_unit() }
pub fn point_room_to_tile(pt: &RoomPoint) -> TilePoint { (*pt / 8).cast_unit() }
pub fn size_tile_to_room(pt: &TileSize) -> RoomSize { (*pt * 8).cast_unit() }
pub fn size_room_to_tile(pt: &RoomSize) -> TileSize { (*pt / 8).cast_unit() }
pub fn vector_tile_to_room(pt: &TileVector) -> RoomVector { (*pt * 8).cast_unit() }
pub fn vector_room_to_tile(pt: &RoomVector) -> TileVector { (*pt / 8).cast_unit() }
pub fn rect_tile_to_room(pt: &TileRect) -> RoomRect { (*pt * 8).cast_unit() }
pub fn rect_room_to_tile(pt: &RoomRect) -> TileRect { (*pt / 8).cast_unit() }

pub struct RectPointIter<T, U> {
    rect: Rect<T, U>,
    step: T,
    next_pt: Option<Point2D<T, U>>,
}

impl<T, U> Iterator for RectPointIter<T, U>
where
    T: core::ops::Add<Output = T> + Copy + Ord
{
    type Item = Point2D<T, U>;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.next_pt;
        let mut next = cur;
        if let Some(mut next) = cur {
            next.x = next.x + self.step;
            if next.x >= self.rect.max_x() {
                next.x = self.rect.min_x();
                next.y = next.y + self.step;
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
    T: core::ops::Add<T> + Copy + Ord
{
    RectPointIter {
        rect, step,
        next_pt: Some(rect.origin),
    }
}
