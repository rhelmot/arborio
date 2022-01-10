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
