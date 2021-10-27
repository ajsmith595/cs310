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
  pub pipelines: HashMap<ID, Pipeline>,
  pub medias: HashMap<ID, PipeableType>,
}
impl Store {
  pub fn new() -> Self {
    Self {
      nodes: HashMap::new(),
      clips: ClipStore::new(),
      pipelines: HashMap::new(),
      medias: HashMap::new(),
    }
  }

  // restructure:
  /*
    - one pipeline
    - invisible links between composited clip outputs + where they are used.
    - generate graph, and check for loops in that - that will cover everything
    - when displaying, separate the graph into a set of connected graphs
    - Then, determine which one to show based on the selected clip
    - positions will be on a per-connected-graph basis
    - when displaying multiple connected graphs at once, can do a few approaches:
      - Have a position per-connected-graph store somewhere
      - Simply use connected-graph boundaries based off min/max position inside the graph. Then just align them in some grid formation automatically
  */
}
