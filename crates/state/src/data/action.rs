use arborio_utils::units::{MapRectStrict, TileGrid, TilePoint, TileVector};
use arborio_utils::uuid::next_uuid;
use std::cell::RefCell;
use std::collections::HashSet;

use crate::data::project_map::{LevelState, MapState, MapStateUpdate};
use arborio_maploader::map_struct::{
    CelesteMapDecal, CelesteMapEntity, CelesteMapLevel, CelesteMapLevelUpdate,
    CelesteMapStyleground,
};

// HERE LIVES THE UNDO/REDOABLES
// guidelines:
// - should all be ABSOLUTE, or can be made absolute through mutation before apply
//   (so undo/redo phased merging works)
// - should only require a single reference to do their jobs, e.g. to the map or to the room
// - should all have a precise inverse, so history tracking is easy
// - events with the same phase should completely supersede each other!!
// UGH THE LAST ONE IS BEING HACKED AROUND NOW

pub fn apply_map_action(
    map: &mut MapState,
    event: Vec<MapAction>,
) -> Result<Vec<MapAction>, String> {
    let mut result: Result<Vec<MapAction>, String> = event
        .into_iter()
        .map(|event| match event {
            MapAction::AddStyleground { loc, style } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx <= vec.len() {
                    vec.insert(loc.idx, *style);
                    Ok(MapAction::RemoveStyleground { loc })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::UpdateStyleground { loc, mut style } => {
                if let Some(style_ref) = map.styles_mut(loc.fg).get_mut(loc.idx) {
                    std::mem::swap(style_ref, &mut style);
                    Ok(MapAction::UpdateStyleground { loc, style })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::RemoveStyleground { loc } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx < vec.len() {
                    let style = vec.remove(loc.idx);
                    Ok(MapAction::AddStyleground {
                        loc,
                        style: Box::new(style),
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::MoveStyleground { loc, target } => {
                let vec = map.styles_mut(loc.fg);
                if loc.idx < vec.len() {
                    let style = vec.remove(loc.idx);
                    let vec = map.styles_mut(target.fg);
                    let real_target = if target.idx <= vec.len() { target } else { loc };
                    let vec = map.styles_mut(real_target.fg);
                    vec.insert(real_target.idx, style);
                    Ok(MapAction::MoveStyleground {
                        loc: real_target,
                        target: loc,
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::MetaUpdate { mut update } => {
                map.data.apply(&mut update);
                Ok(MapAction::MetaUpdate { update })
            }
            MapAction::AddRoom { idx, mut room } => {
                let idx = idx.unwrap_or(map.data.levels.len());
                if room.name.is_empty()
                    || map
                        .data
                        .levels
                        .iter()
                        .any(|iroom| room.name == iroom.data.name)
                {
                    room.name = pick_new_name(map);
                }
                if idx <= map.data.levels.len() {
                    map.data.levels.insert(
                        idx,
                        LevelState {
                            data: *room,
                            cache: RefCell::new(Default::default()),
                            floats: Default::default(),
                        },
                    );
                    Ok(MapAction::DeleteRoom { idx })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::DeleteRoom { idx } => {
                if idx <= map.data.levels.len() {
                    let room = map.data.levels.remove(idx);
                    Ok(MapAction::AddRoom {
                        idx: Some(idx),
                        room: Box::new(room.data),
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
            MapAction::RoomAction { idx, event } => {
                if let Some(room) = map.data.levels.get_mut(idx) {
                    room.cache.borrow_mut().render_cache_valid = false;
                    Ok(MapAction::RoomAction {
                        idx,
                        event: apply_room_event(room, event)?,
                    })
                } else {
                    Err("Out of range".to_owned())
                }
            }
        })
        .collect();
    result.as_mut().map(|result| result.reverse()).ok();
    result
}

fn apply_room_event(room: &mut LevelState, event: RoomAction) -> Result<RoomAction, String> {
    match event {
        RoomAction::UpdateRoomMisc { mut update } => {
            room.data.apply(&mut update);
            Ok(RoomAction::UpdateRoomMisc { update })
        }
        RoomAction::MoveRoom { mut bounds } => {
            if room.data.bounds.size != bounds.size {
                room.data.solids.resize((bounds.size / 8).cast_unit(), '0');
                room.data.bg.resize((bounds.size / 8).cast_unit(), '0');
                room.data
                    .object_tiles
                    .resize((bounds.size / 8).cast_unit(), -1);
                room.cache.borrow_mut().render_cache = None;
            }
            std::mem::swap(&mut room.data.bounds, &mut bounds);
            Ok(RoomAction::MoveRoom { bounds })
        }
        RoomAction::TileUpdate {
            fg,
            offset,
            mut data,
        } => {
            let target = if fg {
                &mut room.data.solids
            } else {
                &mut room.data.bg
            };
            apply_tiles(&offset, &mut data, target, '\0');
            Ok(RoomAction::TileUpdate { fg, offset, data })
        }
        RoomAction::ObjectTileUpdate { offset, mut data } => {
            apply_tiles(&offset, &mut data, &mut room.data.object_tiles, -2);
            Ok(RoomAction::ObjectTileUpdate { offset, data })
        }
        RoomAction::EntityAdd {
            mut entity,
            trigger,
            genid,
        } => {
            let id = if genid {
                let id = room.data.next_id();
                entity.id = id;
                id
            } else if room.entity(entity.id, trigger).is_some() {
                return Err("Entity/trigger already exists".to_owned());
            } else {
                entity.id
            };
            if trigger {
                room.data.triggers.push(*entity);
            } else {
                room.data.entities.push(*entity)
            }
            Ok(RoomAction::EntityRemove { id, trigger })
        }
        RoomAction::EntityUpdate {
            mut entity,
            trigger,
        } => {
            if let Some(e) = room.entity_mut(entity.id, trigger) {
                std::mem::swap(e, &mut entity);
                Ok(RoomAction::EntityUpdate { entity, trigger })
            } else {
                Err("No such entity".to_owned())
            }
        }
        RoomAction::EntityRemove { id, trigger } => {
            let entities = if trigger {
                &mut room.data.triggers
            } else {
                &mut room.data.entities
            };
            for (idx, entity) in entities.iter_mut().enumerate() {
                if entity.id == id {
                    let entity = entities.remove(idx);
                    return Ok(RoomAction::EntityAdd {
                        entity: Box::new(entity),
                        trigger,
                        genid: true,
                    });
                }
            }
            Err("No such entity".to_owned())
        }
        RoomAction::DecalAdd {
            fg,
            mut decal,
            genid,
        } => {
            let id = if genid {
                let id = next_uuid();
                decal.id = id;
                id
            } else if room.decal(decal.id, fg).is_some() {
                return Err("Decal already exists".to_owned());
            } else {
                decal.id
            };
            let decals = if fg {
                &mut room.data.fg_decals
            } else {
                &mut room.data.bg_decals
            };
            decals.push(*decal);
            Ok(RoomAction::DecalRemove { fg, id })
        }
        RoomAction::DecalUpdate { fg, mut decal } => {
            if let Some(decal_dest) = room.decal_mut(decal.id, fg) {
                std::mem::swap(decal_dest, &mut decal);
                Ok(RoomAction::DecalUpdate { fg, decal })
            } else {
                Err("No such decal".to_owned())
            }
        }
        RoomAction::DecalRemove { fg, id } => {
            // tfw drain_filter is unstable
            let decals = if fg {
                &mut room.data.fg_decals
            } else {
                &mut room.data.bg_decals
            };
            for (idx, decal) in decals.iter_mut().enumerate() {
                if decal.id == id {
                    let decal = decals.remove(idx);
                    return Ok(RoomAction::DecalAdd {
                        fg,
                        decal: Box::new(decal),
                        genid: false,
                    });
                }
            }
            Err("No such decal".to_owned())
        }
        RoomAction::TileFloatSet { fg, mut float } => {
            let target = if fg {
                &mut room.floats.fg
            } else {
                &mut room.floats.bg
            };
            std::mem::swap(&mut float, target);
            Ok(RoomAction::TileFloatSet { fg, float })
        }
        RoomAction::ObjFloatSet { mut float } => {
            std::mem::swap(&mut float, &mut room.floats.obj);
            Ok(RoomAction::ObjFloatSet { float })
        }
    }
}

#[derive(Debug, Clone)]
pub enum MapAction {
    AddStyleground {
        loc: StylegroundSelection,
        style: Box<CelesteMapStyleground>,
    },
    UpdateStyleground {
        loc: StylegroundSelection,
        style: Box<CelesteMapStyleground>,
    },
    RemoveStyleground {
        loc: StylegroundSelection,
    },
    MoveStyleground {
        loc: StylegroundSelection,
        target: StylegroundSelection,
    },
    AddRoom {
        idx: Option<usize>, // made absolute through mutation
        room: Box<CelesteMapLevel>,
    },
    DeleteRoom {
        idx: usize,
    },
    RoomAction {
        idx: usize,
        event: RoomAction,
    },
    MetaUpdate {
        update: Box<MapStateUpdate>,
    },
}

#[derive(Debug, Clone)]
pub enum RoomAction {
    MoveRoom {
        bounds: MapRectStrict,
    },
    UpdateRoomMisc {
        update: Box<CelesteMapLevelUpdate>,
    },
    TileUpdate {
        fg: bool,
        offset: TilePoint,
        data: TileGrid<char>,
    },
    ObjectTileUpdate {
        offset: TilePoint,
        data: TileGrid<i32>,
    },
    EntityAdd {
        entity: Box<CelesteMapEntity>,
        trigger: bool,
        genid: bool,
    },
    EntityUpdate {
        entity: Box<CelesteMapEntity>,
        trigger: bool,
    },
    EntityRemove {
        id: i32,
        trigger: bool,
    },
    DecalAdd {
        fg: bool,
        decal: Box<CelesteMapDecal>,
        genid: bool,
    },
    DecalUpdate {
        fg: bool,
        decal: Box<CelesteMapDecal>,
    },
    DecalRemove {
        fg: bool,
        id: u32,
    },
    TileFloatSet {
        fg: bool,
        float: Option<(TilePoint, TileGrid<char>)>,
    },
    ObjFloatSet {
        float: Option<(TilePoint, TileGrid<i32>)>,
    },
}

pub fn apply_tiles<T: Copy + Eq>(
    offset: &TilePoint,
    data: &mut TileGrid<T>,
    target: &mut TileGrid<T>,
    ignore: T,
) -> bool {
    let mut dirty = false;
    let mut line_start = *offset;
    let mut cur = line_start;
    for (idx, tile) in data.tiles.iter_mut().enumerate() {
        if *tile != ignore {
            if let Some(tile_ref) = target.get_mut(cur) {
                if *tile_ref != *tile {
                    std::mem::swap(tile_ref, tile);
                    dirty = true;
                }
            }
        }
        if (idx + 1) % data.stride == 0 {
            line_start += TileVector::new(0, 1);
            cur = line_start;
        } else {
            cur += TileVector::new(1, 0);
        }
    }
    dirty
}

pub fn pick_new_name(map: &MapState) -> String {
    let all_names = map
        .data
        .levels
        .iter()
        .map(|room| &room.data.name)
        .collect::<HashSet<_>>();
    for ch in 'a'..='z' {
        if !all_names.contains(&format!("{ch}-00")) {
            if ch == 'a' {
                return "a-00".to_string();
            } else {
                let ch = (ch as u8 - 1) as char;
                for num in 0..=99 {
                    let result = format!("{ch}-{num:02}");
                    if !all_names.contains(&result) {
                        return result;
                    }
                }
            }
        }
    }

    let mut num = 0;
    loop {
        let result = format!("lvl_{num}");
        if !all_names.contains(&result) {
            break result;
        } else {
            num += 1;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StylegroundSelection {
    pub fg: bool,
    pub idx: usize,
}
