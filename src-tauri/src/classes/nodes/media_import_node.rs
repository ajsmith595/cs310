use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{self, Node, NodeType, NodeTypeProperty, PipeableType, Type},
  nodes::NodeRegister,
  pipeline,
  store::Store,
};

pub const IDENTIFIER: &str = "clip_import";
pub mod INPUTS {
  pub const CLIP: &str = "clip";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}
pub fn media_import_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from(INPUTS::CLIP),
    NodeTypeProperty {
      name: String::from("clip"),
      display_name: String::from("Clip"),
      description: String::from("Clip to import"),
      property_type: vec![Type::Clip],
    },
  );

  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Clip Import"),
    description: String::from("Import a source or composited clip"),
    properties,
    get_output_types: |_,
                       properties: &HashMap<String, Value>,
                       store: &Store,
                       node_register: &NodeRegister| {
      let clip = properties.get(INPUTS::CLIP);
      if clip.is_none() {
        let mut hm = HashMap::new();
        hm.insert(
          String::from(OUTPUTS::OUTPUT),
          NodeTypeProperty {
            name: String::from(OUTPUTS::OUTPUT),
            display_name: String::from("Output"),
            description: String::from("The clip itself"),
            property_type: vec![node::Type::Pipeable(None)],
          },
        );
        return Ok(hm);
      }
      let clip = clip.unwrap().to_owned();
      let clip = serde_json::from_value::<ClipIdentifier>(clip);
      if clip.is_err() {
        return Err(String::from("Clip identifier is malformed"));
      }
      let clip = clip.unwrap();
      let property_type;
      match clip.clip_type {
        ClipType::Source => {
          // If it's a source clip, we get the relevant source clip from the store, and we get its clip type directly (by looking at the file)
          let source_clip = store.clips.source.get(&clip.id);
          if source_clip.is_none() {
            return Err(String::from("Clip ID is invalid"));
          }
          let source_clip = source_clip.unwrap();
          property_type = source_clip.get_clip_type();
        }
        ClipType::Composited => {
          let composited_clip = store.clips.composited.get(&clip.id);
          if composited_clip.is_none() {
            return Err(String::from("Clip ID is invalid"));
          }
          let composited_clip = composited_clip.unwrap();
          let prop_type =
            store
              .pipeline
              .get_output_type(composited_clip.id.clone(), store, node_register);
          if prop_type.is_err() {
            return Err(format!(
              "Failed to get output type for composited clip ({})",
              prop_type.unwrap_err()
            ));
          }
          property_type = prop_type.unwrap();
        }
      }
      let mut hm = HashMap::new();
      hm.insert(
        String::from(OUTPUTS::OUTPUT),
        NodeTypeProperty {
          name: String::from(OUTPUTS::OUTPUT),
          display_name: String::from("Output"),
          description: String::from("The clip itself"),
          property_type: vec![property_type],
        },
      );
      return Ok(hm);
    },
    get_output: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      let clip_identifier = get_clip_identifier(properties);
      if clip_identifier.is_err() {
        return Err(clip_identifier.unwrap_err());
      }
      let clip_identifier = clip_identifier.unwrap();
      match &clip_identifier.clip_type {
        ClipType::Source => {
          // If it's a source clip, we get the relevant source clip from the store, and we get its clip type directly (by looking at the file)
          let source_clip = store.clips.source.get(&clip_identifier.id);
          if source_clip.is_none() {
            return Err(String::from("Clip ID is invalid"));
          }
          let source_clip = source_clip.unwrap();
          return Ok(format!(
            "filesrc location=\"{}\" ! qtdemux ! h264parse ! d3d11h264dec ! videoconvert name={}",
            source_clip.file_location,
            Node::get_gstreamer_handle_id(node_id, OUTPUTS::OUTPUT.to_string())
          ));
        }
        ClipType::Composited => {
          let composited_clip = store.clips.composited.get(&clip_identifier.id);
          if composited_clip.is_none() {
            return Err(String::from("Clip ID is invalid"));
          }
          let composited_clip = composited_clip.unwrap();
          return Ok(format!(
            "{}. ! videoconvert name='{}'",
            composited_clip.get_gstreamer_id(),
            Node::get_gstreamer_handle_id(node_id, OUTPUTS::OUTPUT.to_string())
          ));
        }
      }
    },
  }
}

pub fn get_clip_identifier(properties: &HashMap<String, Value>) -> Result<ClipIdentifier, String> {
  let clip = properties.get(INPUTS::CLIP);
  if clip.is_none() {
    return Err(String::from("No clip given"));
  }
  let clip = clip.unwrap().to_owned();
  let clip = serde_json::from_value::<ClipIdentifier>(clip);
  if clip.is_err() {
    return Err(String::from("Clip identifier is malformed"));
  }
  let clip = clip.unwrap();

  Ok(clip)
}
