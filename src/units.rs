use euclid::*;

pub struct RoomSpace;
pub struct MapSpace;
pub struct ScreenSpace;

pub type RoomRect = Rect<i32, RoomSpace>;
pub type RoomPoint = Point2D<i32, RoomSpace>;
pub type RoomSize = Point2D<i32, RoomSpace>;
pub type RoomVector = Point2D<i32, RoomSpace>;

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
pub type RoomToMap = Translation2D<i32, RoomSpace, MapSpace>;
