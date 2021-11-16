use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeProperty, PipeableType, Restrictions, Type},
  store::Store,
};

pub const IDENTIFIER: &str = "blur";
pub mod INPUTS {
  pub const MEDIA: &str = "media";
  pub const SIGMA: &str = "sigma";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}

pub fn blur_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from(INPUTS::MEDIA),
    NodeTypeProperty {
      name: String::from(INPUTS::MEDIA),
      display_name: String::from("Media"),
      description: String::from("The media to be blurred"),
      property_type: vec![Type::Pipeable(None)],
    },
  );

  properties.insert(
    String::from(INPUTS::SIGMA),
    NodeTypeProperty {
      name: String::from(INPUTS::SIGMA),
      display_name: String::from("Blur Amount"),
      description: String::from(
        "The sigma value for the blur; the higher the value, the more the media is blurred",
      ),
      property_type: vec![Type::Number(Restrictions {
        min: (0.0 as f64),
        max: (100.0 as f64),
        step: (0.01 as f64),
        default: (1.2 as f64),
      })],
    },
  );

  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Blur"),
    description: String::from("Blur a media source"),
    properties,
    get_output_types: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      let mut hm = HashMap::new();
      hm.insert(
        OUTPUTS::OUTPUT.to_string(),
        NodeTypeProperty {
          name: OUTPUTS::OUTPUT.to_string(),
          description: "The blurred media".to_string(),
          display_name: "Output".to_string(),
          property_type: vec![Type::Pipeable(None)],
        },
      );
      return Ok(hm);
    },
    get_output: |node_id: String, properties: &HashMap<String, Value>, store: &Store, _| {
      println!("{:?}", properties);
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
      return Err(format!("Media is invalid type"));
    },
  }
}
