use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeProperty, PipeableType, Type},
  store::Store,
};

pub fn output_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from("media"),
    NodeTypeProperty {
      name: String::from("media"),
      display_name: String::from("Media"),
      description: String::from("Media to output to clip"),
      property_type: vec![Type::Pipeable(None)],
    },
  );

  properties.insert(
    String::from("clip"),
    NodeTypeProperty {
      name: String::from("clip"),
      display_name: String::from("Clip"),
      description: String::from("Clip to output"),
      property_type: vec![Type::Clip],
    },
  );

  NodeType {
    id: String::from("output"),
    display_name: String::from("Output"),
    description: String::from("Output media to a clip"),
    properties,
    get_output_types: |_, _| Ok(HashMap::new()),
    get_output: |_, _| todo!(),
  }
}
