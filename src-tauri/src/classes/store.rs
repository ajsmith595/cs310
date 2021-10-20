use std::collections::HashMap;

use super::{
  clip::{ClipType, CompositedClip, SourceClip},
  node::Node,
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
  pub pipeline_store: HashMap<ID, Pipeline>,
}
