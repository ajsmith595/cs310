use std::{cell::Cell, collections::HashMap};

use super::{
  clip::{ClipType, CompositedClip, SourceClip},
  node::{Node, NodeType, PipeableType},
  pipeline::Pipeline,
  ID,
};
#[derive(Clone)]
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
#[derive(Clone)]
pub struct Store {
  pub nodes: HashMap<ID, Node>,
  pub clips: ClipStore,
  pub pipelines: HashMap<ID, Pipeline>,
  pub node_types: HashMap<String, NodeType>,
  pub medias: HashMap<ID, PipeableType>,
}
impl Store {
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      clips: ClipStore::new(),
      pipelines: HashMap::new(),
      node_types: HashMap::new(),
      medias: HashMap::new(),
    }
  }
}
