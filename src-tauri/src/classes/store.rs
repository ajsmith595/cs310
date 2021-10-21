use std::collections::HashMap;

use super::{
  clip::{ClipType, CompositedClip, SourceClip},
  node::{Node, NodeType, PipeableType},
  pipeline::Pipeline,
  ID,
};

pub struct ClipStore {
  pub source: HashMap<ID, SourceClip>,
  pub composited: HashMap<ID, CompositedClip>,
}
pub struct Store {
  pub nodes: HashMap<ID, Node>,
  pub clips: ClipStore,
  pub pipelines: HashMap<ID, Pipeline>,
  pub node_types: HashMap<String, NodeType>,
  pub medias: HashMap<ID, PipeableType>,
}
