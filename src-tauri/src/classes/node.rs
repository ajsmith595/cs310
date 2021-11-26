use std::{
  collections::{hash_map, HashMap},
  hash::Hash,
};
use uuid::Uuid;

use serde_json::Value;

use super::{global::uniq_id, nodes::NodeRegister, store::Store, ID};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}
impl Position {
  pub fn new() -> Self {
    Self { x: 0, y: 0 }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
  pub position: Position,
  pub id: ID,
  pub node_type: String,
  pub properties: HashMap<String, Value>, // value from serde_json?
  pub group: ID,
}
impl Node {
  pub fn new(node_type: String, group: Option<ID>) -> Self {
    let group_id;
    if group.is_none() {
      group_id = uniq_id();
    } else {
      group_id = group.unwrap();
    }
    Self {
      position: Position::new(),
      id: uniq_id(),
      node_type,
      properties: HashMap::new(),
      group: group_id,
    }
  }

  pub fn get_gstreamer_handle_id(node_id: String, property: String) -> String {
    format!("{}-{}", node_id, property)
  }
}
#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub struct Restrictions {
  pub min: f64,
  pub max: f64,
  pub step: f64,
  pub default: f64,
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub struct PipeableType {
  pub video: i32,
  pub audio: i32,
  pub subtitles: i32,
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub enum Type {
  Pipeable(PipeableType, PipeableType),
  Number(Restrictions),
  String(i32), // TODO: make these easier to read + add more properties. e.g. min string length, max string length, regex for valid string, etc.
  // Maybe some restrictions on video min/max duration, resolution, etc?
  Clip,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeInput {
  pub name: String,
  pub display_name: String,
  pub description: String,
  pub property_type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeOutput {
  pub name: String,
  pub display_name: String,
  pub description: String,
  pub property_type: PipeableType,
}

type NodeTypeFunc<T> = fn(
  node_id: String,
  properties: &HashMap<String, Value>,
  store: &Store,
  node_register: &NodeRegister,
) -> Result<T, String>;

#[derive(Serialize, Clone)]
pub struct NodeType {
  pub id: String,
  pub display_name: String,
  pub description: String,
  pub default_properties: HashMap<String, NodeTypeInput>,

  #[serde(skip_serializing)]
  pub get_properties: NodeTypeFunc<HashMap<String, NodeTypeInput>>,
  #[serde(skip_serializing)]
  pub get_output_types: NodeTypeFunc<HashMap<String, NodeTypeOutput>>,
  #[serde(skip_serializing)]
  pub get_output: NodeTypeFunc<String>,
}

// impl NodeType {
//   pub fn new(
//     id: String,
//     display_name: String,
//     description: String,

//     get_properties: fn(
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<HashMap<String, NodeTypeInput>, String>,

//     get_output_types: fn(
//       node_id: String,
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<HashMap<String, NodeTypeOutput>, String>,
//     get_output: fn(
//       node_id: String,
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<String, String>,

//     store: &Store,
//   ) -> Self {
//     let default_properties = get_properties(HashMap::new(), store, )
//     Self {
//       id,
//     display_name,
//     description,
//     get_properties,
//     get_output_types,
//     get_output,

//     }
//   }
// }
