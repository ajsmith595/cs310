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
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      clips: ClipStore::new(),
      pipeline: Pipeline::new(),
      medias: HashMap::new(),
    }
  }
}
