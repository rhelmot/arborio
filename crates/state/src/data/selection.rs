use arborio_maploader::map_struct::{CelesteMapDecal, CelesteMapEntity, CelesteMapLevel};
use arborio_utils::units::{TileGrid, TilePoint};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub enum AppSelection {
    FgTile(TilePoint),
    BgTile(TilePoint),
    ObjectTile(TilePoint),
    FgFloat,
    BgFloat,
    ObjFloat,
    EntityBody(i32, bool),
    EntityNode(i32, usize, bool),
    Decal(u32, bool),
}

impl AppSelection {
    pub fn entity_info(&self) -> Option<(i32, bool)> {
        if let AppSelection::EntityBody(entity_id, trigger)
        | AppSelection::EntityNode(entity_id, _, trigger) = self
        {
            Some((*entity_id, *trigger))
        } else {
            None
        }
    }
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
