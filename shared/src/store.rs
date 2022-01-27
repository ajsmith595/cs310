use std::{cell::Cell, collections::HashMap, hash::Hash};

use petgraph::graph::DiGraph;
use serde_json::Value;

use super::{
    clip::{ClipType, CompositedClip, SourceClip},
    node::{Node, NodeType, PipeableType},
    nodes::NodeRegister,
    pipeline::Pipeline,
    ID,
};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipStore {
    pub source: HashMap<ID, SourceClip>,
    pub composited: HashMap<ID, CompositedClip>,
}
impl ClipStore {
    pub fn new() -> Self {
        Self {
            source: HashMap::new(),
            composited: HashMap::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Store {
    pub nodes: HashMap<ID, Node>,
    pub clips: ClipStore,
    pub pipeline: Pipeline,
    pub medias: HashMap<ID, PipeableType>,
}
impl Store {
    pub fn new(base_output_location: String) -> Self {
        Self {
            nodes: HashMap::new(),
            clips: ClipStore::new(),
            pipeline: Pipeline::new(),
            medias: HashMap::new(),
        }
    }

    pub fn from_file(filename: String) -> Result<Self, String> {
        let f = std::fs::read(filename);

        if f.is_err() {
            return Err(f.unwrap_err().to_string());
        }
        let data = f.unwrap();

        let store = serde_json::from_slice(&data);

        if store.is_err() {
            return Err(store.unwrap_err().to_string());
        }
        Ok(store.unwrap())
    }
}
