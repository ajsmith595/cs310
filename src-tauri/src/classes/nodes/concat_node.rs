use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeProperty, PipeableType, Type},
  store::Store,
};

pub const IDENTIFIER: &str = "concat";
pub mod INPUTS {
  pub const MEDIA1: &str = "media1";
  pub const MEDIA2: &str = "media2";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}

pub fn concat_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from(INPUTS::MEDIA1),
    NodeTypeProperty {
      name: String::from(INPUTS::MEDIA1),
      display_name: String::from("Media 1"),
      description: String::from("The first media to play"),
      property_type: vec![Type::Pipeable(None)],
    },
  );

  properties.insert(
    String::from(INPUTS::MEDIA2),
    NodeTypeProperty {
      name: String::from(INPUTS::MEDIA2),
      display_name: String::from("Media 2"),
      description: String::from("The second media to play"),
      property_type: vec![Type::Pipeable(None)],
    },
  );

  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Concatenation"),
    description: String::from("Concatenate two media sources"),
    properties,
    get_output_types: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      // let media1 = properties.get(INPUTS::MEDIA1).unwrap();
      // let media2 = properties.get(INPUTS::MEDIA2).unwrap();
      // if let (Value::String(media1), Value::String(media2)) = (media1, media2) {
      let mut hm = HashMap::new();
      hm.insert(
        OUTPUTS::OUTPUT.to_string(),
        NodeTypeProperty {
          name: OUTPUTS::OUTPUT.to_string(),
          description: "The concatenation of the two media".to_string(),
          display_name: "Output".to_string(),
          property_type: vec![Type::Pipeable(None)],
        },
      );
      return Ok(hm);
      // }
      // return Err(format!("Media is invalid type"));
    },
    get_output: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      println!("{:?}", properties);
      let media1 = properties.get(INPUTS::MEDIA1).unwrap();
      let media2 = properties.get(INPUTS::MEDIA2).unwrap();
      if let (Value::String(media1), Value::String(media2)) = (media1, media2) {
        let out_id = Node::get_gstreamer_handle_id(node_id.clone(), OUTPUTS::OUTPUT.to_string());
        return Ok(format!(
          "concat name={} {}. ! {}. {}. ! {}.",
          out_id.clone(),
          media1,
          out_id.clone(),
          media2,
          out_id.clone(),
        ));
      }
      return Err(format!("Media is invalid type"));
    },
  }
}
