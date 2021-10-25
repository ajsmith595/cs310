#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{cell::Cell, collections::HashMap};

use uuid::Uuid;

use crate::classes::{
  clip::{ClipIdentifier, SourceClip},
  node::{Node, Position},
  nodes::{get_node_register, media_import_node, output_node},
  pipeline::{Link, LinkEndpoint, Pipeline},
  store::{ClipStore, Store},
};

#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate erased_serde;
// extern crate dirs;
// extern crate gstreamer;
extern crate serde;
extern crate serde_json;

mod classes;

fn main() {
  let mut clip_store = ClipStore::new();
  let clip_id = Uuid::new_v4().to_string();
  clip_store.source.insert(
    clip_id.clone(),
    SourceClip {
      id: clip_id.clone(),
      name: "Test Clip".to_string(),
      file_location: "Test clip 1.mp4".to_string(),
    },
  );

  let mut media_import_node1 = Node::new(media_import_node::IDENTIFIER.to_string());
  media_import_node1.properties.insert(
    media_import_node::INPUTS::CLIP.to_string(),
    serde_json::to_value(ClipIdentifier {
      id: clip_id.clone(),
      clip_type: classes::clip::ClipType::Source,
    })
    .unwrap(),
  );
  let output_node1 = Node::new(output_node::IDENTIFIER.to_string());

  let mut nodes = HashMap::new();
  nodes.insert(
    media_import_node1.id.clone(),
    Cell::new(media_import_node1.clone()),
  );
  nodes.insert(output_node1.id.clone(), Cell::new(output_node1.clone()));

  let mut pipelines = HashMap::new();
  let mut pipeline1 = Pipeline::new();
  pipeline1.links.push(Link {
    from: LinkEndpoint {
      node_id: media_import_node1.id.clone(),
      property: media_import_node::OUTPUTS::OUTPUT.to_string(),
    },
    to: LinkEndpoint {
      node_id: output_node1.id.clone(),
      property: output_node::INPUTS::MEDIA.to_string(),
    },
  });
  pipelines.insert("main".to_string(), pipeline1);
  let store = Store {
    nodes,
    clips: clip_store,
    pipelines: pipelines,
    node_types: get_node_register(),
    medias: HashMap::new(),
  };

  tauri::Builder::default()
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
