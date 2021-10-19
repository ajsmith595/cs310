use std::{collections::HashMap, hash::Hash};

use serde_json::Value;

use super::ID;

pub struct Position {
  pub x: i32,
  pub y: i32,
}

pub struct Node {
  pub position: Position,
  pub id: ID,
  pub node_type: String,
  //   pub properties: Map<String, Value>, // value from serde_json?
}

pub struct Restrictions {
  pub min: f32,
  pub max: f32,
  pub step: f32,
}

pub enum Type {
  Number(Restrictions),
  String(i32), // TODO: make these easier to read + add more properties. e.g. min string length, max string length, regex for valid string, etc.
  Video,       // Maybe some restrictions on video min/max duration, resolution, etc?
  Audio,
  Image,
}

pub struct NodeTypeProperty {
  name: String,
  display_name: String,
  description: String,
  property_type: Type,
}
pub struct NodeType {
  pub id: String,
  pub display_name: String,
  pub description: String,
  pub properties: HashMap<String, NodeTypeProperty>,
  pub get_output_types:
    fn(properties: HashMap<String, Value>) -> Result<HashMap<String, NodeTypeProperty>, String>,
  pub get_output: fn(
    properties: HashMap<String, Value>,
    node_store: HashMap<String, Node>,
    misc_type_store: HashMap<String, HashMap<String, Value>>,
  ) -> Result<String, String>,
}
