use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType, CompositedClip},
  node::{Node, NodeType, NodeTypeInput, PipeableType, Type},
  store::Store,
};

pub const IDENTIFIER: &str = "output";
pub mod INPUTS {
  pub const MEDIA: &str = "media";
  pub const CLIP: &str = "clip";
}
pub mod OUTPUTS {}

fn default_properties() -> HashMap<String, NodeTypeInput> {
  let mut default_properties = HashMap::new();

  default_properties.insert(
    String::from(INPUTS::MEDIA),
    NodeTypeInput {
      name: String::from(INPUTS::MEDIA),
      display_name: String::from("Media"),
      description: String::from("Media to output to clip"),
      property_type: Type::Pipeable(
        PipeableType {
          video: 0,
          audio: 0,
          subtitles: 0,
        },
        PipeableType {
          video: i32::MAX,
          audio: i32::MAX,
          subtitles: i32::MAX,
        },
      ),
    },
  );

  default_properties.insert(
    String::from(INPUTS::CLIP),
    NodeTypeInput {
      name: String::from(INPUTS::CLIP),
      display_name: String::from("Clip"),
      description: String::from("Clip to output"),
      property_type: Type::Clip,
    },
  );
  default_properties
}

pub fn output_node() -> NodeType {
  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Output"),
    description: String::from("Output media to a clip"),
    default_properties: default_properties(),
    get_properties: |_, _, _, _| Ok(default_properties()),
    get_output_types: |_, _, _, _| Ok(HashMap::new()),
    get_output: |_, properties: &HashMap<String, Value>, store: &Store, _| {
      let media = properties.get(INPUTS::MEDIA).unwrap();
      if let Value::String(media) = media {
        let clip = get_clip(properties, store);
        if clip.is_err() {
          return Err(clip.unwrap_err());
        }
        let clip = clip.unwrap();
        return Ok(format!(
          "{}. ! videoconvert name={}",
          media,
          clip.get_gstreamer_id()
        ));
      }
      return Err(format!("Media is invalid type"));
    },
  }
}

pub fn get_clip(
  properties: &HashMap<String, Value>,
  store: &Store,
) -> Result<CompositedClip, String> {
  let clip = properties.get(INPUTS::CLIP);
  if clip.is_none() {
    return Err(String::from("No clip given"));
  }
  let clip = clip.unwrap().to_owned();
  let clip = serde_json::from_value::<ClipIdentifier>(clip).unwrap();
  return Ok(store.clips.composited.get(&clip.id).unwrap().clone());
}
