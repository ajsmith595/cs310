use std::{cell::Cell, collections::HashMap, hash::Hash};

use petgraph::graph::DiGraph;

use super::{
  clip::{ClipType, CompositedClip, SourceClip},
  node::{Node, NodeType, PipeableType},
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

  // restructure:
  /*
  - one pipeline
  - invisible links between composited clip outputs + where they are used.
  - generate graph, and check for loops in that - that will cover everything
  - every node will have a "group". Any connected node must have the same group
  - when displaying a clip's node graph, only show the nodes with the same group
  - positions will be on a per-group-basis
  */
}
