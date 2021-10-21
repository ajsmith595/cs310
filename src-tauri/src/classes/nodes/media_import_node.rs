use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{Node, NodeType, NodeTypeProperty, Type},
  store::Store,
};

pub fn media_import_node() -> NodeType {
  let mut properties = HashMap::new();

  properties.insert(
    String::from("clip"),
    NodeTypeProperty {
      name: String::from("clip"),
      display_name: String::from("Clip"),
      description: String::from("Clip to import"),
      property_type: vec![Type::Clip],
    },
  );

  NodeType {
    id: String::from("clip_import"),
    display_name: String::from("Clip Import"),
    description: String::from("Import a source or composited clip"),
    properties,
    get_output_types: |properties: &HashMap<String, Value>, store: &Store| {
      let clip = properties.get("clip");
      if clip.is_none() {
        return Err(String::from("No clip given"));
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
          let pipeline = store.pipelines.get(&composited_clip.pipeline_id);
          if pipeline.is_none() {
            return Err(String::from("Pipeline ID of clip is invalid"));
          }
          let pipeline = pipeline.unwrap();
          property_type = pipeline.get_output_type(composited_clip.id.clone());
        }
      }
      let mut hm = HashMap::new();
      hm.insert(
        String::from("output"),
        NodeTypeProperty {
          name: String::from("output"),
          display_name: String::from("Output"),
          description: String::from("The clip itself"),
          property_type: vec![property_type],
        },
      );
      return Ok(hm);
    },
    get_output: |_, _| todo!(),
  }
}
