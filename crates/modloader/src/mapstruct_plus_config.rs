use crate::config::Const;
use arborio_maploader::map_struct::{CelesteMapEntity, Node};
use std::collections::HashMap;

pub fn make_entity_env(this: &CelesteMapEntity) -> HashMap<&str, Const> {
    let mut env: HashMap<&str, Const> = HashMap::new();
    env.insert("x", Const::from_num(this.x));
    env.insert("y", Const::from_num(this.y));
    env.insert("width", Const::from_num(this.width));
    env.insert("height", Const::from_num(this.height));
    for (key, val) in &this.attributes {
        env.insert(key.as_str(), Const::from_attr(val));
    }
    if let Some(Node { x, y }) = this.nodes.first() {
        env.insert("firstnodex", Const::from_num(*x));
        env.insert("firstnodey", Const::from_num(*y));
    }
    if let Some(Node { x, y }) = this.nodes.last() {
        env.insert("lastnodex", Const::from_num(*x));
        env.insert("lastnodey", Const::from_num(*y));
    }

    env
}

pub fn make_node_env<'a>(
    this: &CelesteMapEntity,
    mut env: HashMap<&'a str, Const>,
    node_idx: usize,
) -> HashMap<&'a str, Const> {
    env.insert("nodeidx", Const::from_num(node_idx as f64));
    if let Some(Node { x, y }) = this.nodes.get(node_idx) {
        env.insert("nodex", Const::from_num(*x));
        env.insert("nodey", Const::from_num(*y));
    }
    if let Some(Node { x, y }) = this.nodes.get(node_idx + 1) {
        env.insert("nextnodex", Const::from_num(*x));
        env.insert("nextnodey", Const::from_num(*y));
        env.insert("nextnodexorbase", Const::from_num(*x));
        env.insert("nextnodeyorbase", Const::from_num(*y));
    } else {
        env.insert("nextnodexorbase", Const::from_num(this.x));
        env.insert("nextnodeyorbase", Const::from_num(this.y));
    }
    if let Some(Node { x, y }) = this.nodes.get(node_idx.wrapping_sub(1)) {
        env.insert("prevnodex", Const::from_num(*x));
        env.insert("prevnodey", Const::from_num(*y));
        env.insert("prevnodexorbase", Const::from_num(*x));
        env.insert("prevnodeyorbase", Const::from_num(*y));
    } else {
        env.insert("prevnodexorbase", Const::from_num(this.x));
        env.insert("prevnodeyorbase", Const::from_num(this.y));
    }

    env
}
