use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeProperty, PipeableType, Type},
  store::Store,
};

pub const IDENTIFIER: &str = "output";
pub mod INPUTS {
  pub const MEDIA: &str = "media";
  pub const CLIP: &str = "clip";
}
pub mod OUTPUTS {}

pub fn output_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from(INPUTS::MEDIA),
    NodeTypeProperty {
      name: String::from(INPUTS::MEDIA),
      display_name: String::from("Media"),
      description: String::from("Media to output to clip"),
      property_type: vec![Type::Pipeable(None)],
    },
  );

  properties.insert(
    String::from(INPUTS::CLIP),
    NodeTypeProperty {
      name: String::from(INPUTS::CLIP),
      display_name: String::from("Clip"),
      description: String::from("Clip to output"),
      property_type: vec![Type::Clip],
    },
  );

  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Output"),
    description: String::from("Output media to a clip"),
    properties,
    get_output_types: |_, _| Ok(HashMap::new()),
    get_output: |_, _| todo!(),
  }
}
