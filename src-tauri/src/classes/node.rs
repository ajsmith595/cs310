use std::{
  collections::{hash_map, HashMap},
  hash::Hash,
};
use uuid::Uuid;

use serde_json::Value;

use super::{store::Store, ID};

#[derive(Clone)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}
impl Position {
  pub fn new() -> Self {
    Self { x: 0, y: 0 }
  }
}

#[derive(Clone)]
pub struct Node {
  pub position: Position,
  pub id: ID,
  pub node_type: String,
  pub properties: HashMap<String, Value>, // value from serde_json?
}
impl Node {
  pub fn new(node_type: String) -> Self {
    Self {
      position: Position::new(),
      id: Uuid::new_v4().to_string(),
      node_type,
      properties: HashMap::new(),
    }
  }
}

impl Node {
  pub fn get_gstreamer_handle_id(node_id: String, property: String) -> String {
    format!("{}-{}", node_id, property)
  }
}
#[derive(Copy, Clone)]
pub struct Restrictions {
  pub min: f32,
  pub max: f32,
  pub step: f32,
}
#[derive(Copy, Clone)]
pub enum PipeableType {
  Video,
  Audio,
  Image,
}

#[derive(Copy, Clone)]
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
    node_id: String,
    properties: &HashMap<String, Value>,
    store: &Store,
  ) -> Result<HashMap<String, NodeTypeProperty>, String>,
  pub get_output: fn(
    node_id: String,
    properties: &HashMap<String, Value>,
    store: &Store,
  ) -> Result<String, String>,
}
