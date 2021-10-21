use std::{
  collections::{hash_map, HashMap},
  hash::Hash,
};

use serde_json::Value;

use super::{store::Store, ID};

pub struct Position {
  pub x: i32,
  pub y: i32,
}

pub struct Node {
  pub position: Position,
  pub id: ID,
  pub node_type: String,
  pub properties: HashMap<String, Value>, // value from serde_json?
}

pub struct Restrictions {
  pub min: f32,
  pub max: f32,
  pub step: f32,
}

pub enum PipeableType {
  Video,
  Audio,
  Image,
}

pub enum Type {
  Pipeable(Option<PipeableType>),
  Number(Restrictions),
  String(i32), // TODO: make these easier to read + add more properties. e.g. min string length, max string length, regex for valid string, etc.
  // Maybe some restrictions on video min/max duration, resolution, etc?
  Clip,
}

pub struct NodeTypeProperty {
  pub name: String,
  pub display_name: String,
  pub description: String,
  pub property_type: Vec<Type>,
}
pub struct NodeType {
  pub id: String,
  pub display_name: String,
  pub description: String,
  pub properties: HashMap<String, NodeTypeProperty>,
  pub get_output_types: fn(
    properties: &HashMap<String, Value>,
    store: &Store,
  ) -> Result<HashMap<String, NodeTypeProperty>, String>,
  pub get_output: fn(properties: &HashMap<String, Value>, store: &Store) -> Result<String, String>,
}
