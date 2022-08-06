use arborio_maploader::map_struct::{CelesteMapDecal, CelesteMapEntity, CelesteMapLevel};
use arborio_utils::units::{TileGrid, TilePoint};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    ObjectTile(TilePoint),
    EntityBody(i32, bool),
    EntityNode(i32, usize, bool),
    Decal(u32, bool),
}

#[derive(Serialize, Deserialize)]
pub enum AppSelectable {
    InRoom(Vec<AppInRoomSelectable>),
    Rooms(Vec<CelesteMapLevel>),
}

#[derive(Serialize, Deserialize)]
pub enum AppInRoomSelectable {
    FgTiles(TilePoint, TileGrid<char>),
    BgTiles(TilePoint, TileGrid<char>),
    ObjectTiles(TilePoint, TileGrid<i32>),
    Entity(CelesteMapEntity, bool),
    Decal(CelesteMapDecal, bool),
}
