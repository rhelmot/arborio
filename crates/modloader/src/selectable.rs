use arborio_utils::interned::Interned;
use arborio_utils::vizia::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct TileSelectable {
    pub id: char,
    pub name: &'static str,
    pub texture: Option<&'static str>,
}

impl Default for TileSelectable {
    fn default() -> Self {
        TileSelectable {
            id: '0',
            name: "Empty",
            texture: None,
        }
    }
}

impl Data for TileSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl PartialEq for TileSelectable {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TileSelectable {}

//unsafe impl Send for TileSelectable {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntitySelectable {
    pub entity: Interned,
    pub template: usize,
}

impl Default for EntitySelectable {
    fn default() -> Self {
        Self {
            entity: "does not exist".into(),
            template: 0,
        }
    }
}

impl Data for EntitySelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TriggerSelectable {
    pub trigger: Interned,
    pub template: usize,
}

impl Default for TriggerSelectable {
    fn default() -> Self {
        Self {
            trigger: "does not exist".into(),
            template: 0,
        }
    }
}

impl Data for TriggerSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DecalSelectable(pub Interned);

impl Default for DecalSelectable {
    fn default() -> Self {
        Self("does not exist".into())
    }
}

impl Data for DecalSelectable {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
