use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableType, Type},
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

fn default_properties() -> HashMap<String, NodeTypeInput> {
  let mut default_properties = HashMap::new();
  {
    default_properties.insert(
      String::from(INPUTS::MEDIA1),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA1),
        display_name: String::from("Media 1"),
        description: String::from("The first media to play"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 1,
            audio: 0,
            subtitles: 0,
          },
          PipeableType {
            video: 1,
            audio: i32::MAX,
            subtitles: i32::MAX,
          },
        ),
      },
    );

    default_properties.insert(
      String::from(INPUTS::MEDIA2),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA2),
        display_name: String::from("Media 2"),
        description: String::from("The second media to play"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 1,
            audio: 0,
            subtitles: 0,
          },
          PipeableType {
            video: 1,
            audio: i32::MAX,
            subtitles: i32::MAX,
          },
        ),
      },
    );
  }

  default_properties
}

pub fn concat_node() -> NodeType {
  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Concatenation"),
    description: String::from("Concatenate two media sources"),
    default_properties: default_properties(),
    get_properties: |_, _, _, _| Ok(default_properties()),
    get_output_types: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      // let media1 = properties.get(INPUTS::MEDIA1).unwrap();
      // let media2 = properties.get(INPUTS::MEDIA2).unwrap();
      // if let (Value::String(media1), Value::String(media2)) = (media1, media2) {
      let mut hm = HashMap::new();
      hm.insert(
        OUTPUTS::OUTPUT.to_string(),
        NodeTypeOutput {
          name: OUTPUTS::OUTPUT.to_string(),
          description: "The concatenation of the two media".to_string(),
          display_name: "Output".to_string(),
          property_type: PipeableType {
            video: 1,
            audio: 0,
            subtitles: 0, // TODO: get media types
          },
        },
      );
      return Ok(hm);
      // }
      // return Err(format!("Media is invalid type"));
    },
    get_output: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
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
