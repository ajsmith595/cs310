use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableType, Restrictions, Type},
  store::Store,
};

pub const IDENTIFIER: &str = "audio_gain";
pub mod INPUTS {
  pub const MEDIA: &str = "media";
  pub const SIGMA: &str = "gain";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}

fn default_properties() -> HashMap<String, NodeTypeInput> {
  let mut default_properties = HashMap::new();
  {
    default_properties.insert(
      String::from(INPUTS::MEDIA),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA),
        display_name: String::from("Media"),
        description: String::from("The media to be gained"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 0,
            audio: 1,
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
      String::from(INPUTS::SIGMA),
      NodeTypeInput {
        name: String::from(INPUTS::SIGMA),
        display_name: String::from("Gain Amount"),
        description: String::from("The amount to gain by"),
        property_type: Type::Number(Restrictions {
          min: (-12 as f64),
          max: (12 as f64),
          step: (0.1 as f64),
          default: (0 as f64),
        }),
      },
    );
  }
  default_properties
}

pub fn audio_gain() -> NodeType {
  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Audio Gain"),
    description: String::from("Increase the volume of a source"),
    default_properties: default_properties(),
    get_properties: |_, _, _, _| Ok(default_properties()),
    get_output_types: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      let mut hm = HashMap::new();
      hm.insert(
        OUTPUTS::OUTPUT.to_string(),
        NodeTypeOutput {
          name: OUTPUTS::OUTPUT.to_string(),
          description: "The gained media".to_string(),
          display_name: "Output".to_string(),
          property_type: PipeableType {
            video: 0,
            audio: 1,
            subtitles: 0,
          }, // TODO: get prev node's type and analyse or whatevs
        },
      );
      return Ok(hm);
    },
    get_output: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      let media = properties.get(INPUTS::MEDIA).unwrap();
      if let Value::String(media) = media {
        let sigma = properties.get(INPUTS::SIGMA).unwrap();
        if let Value::Number(sigma) = sigma {
          let out_id = Node::get_gstreamer_handle_id(node_id.clone(), OUTPUTS::OUTPUT.to_string());
          return Ok(format!(
            "{} ! gaussianblur sigma={} name={}",
            media,
            sigma.as_f64().unwrap().to_owned(),
            out_id.clone()
          ));
        }
      }
      return Err(format!(
        "Media is invalid type (gaussian blur): \n{:#?}\n\n",
        properties
      ));
    },
  }
}
