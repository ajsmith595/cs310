#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{cell::Cell, collections::HashMap};

use uuid::Uuid;

use crate::classes::{
  clip::{ClipIdentifier, CompositedClip, SourceClip},
  node::{Node, Position},
  nodes::{concat_node, get_node_register, media_import_node, output_node},
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
  let source_clip1;
  let composited_clip1;
  let source_clip2;
  {
    source_clip1 = Uuid::new_v4().to_string();
    clip_store.source.insert(
      source_clip1.clone(),
      SourceClip {
        id: source_clip1.clone(),
        name: "Test Clip".to_string(),
        file_location: "Test clip 1.mp4".to_string(),
      },
    );

    source_clip2 = Uuid::new_v4().to_string();
    clip_store.source.insert(
      source_clip2.clone(),
      SourceClip {
        id: source_clip2.clone(),
        name: "Test Clip".to_string(),
        file_location: "Test clip 1.mp4".to_string(),
      },
    );

    composited_clip1 = Uuid::new_v4().to_string();
    clip_store.composited.insert(
      composited_clip1.clone(),
      CompositedClip {
        id: composited_clip1.clone(),
        name: "Test Composited Clip".to_string(),
        pipeline_id: "".to_string(),
      },
    );
  }

  let mut media_import_node1 = Node::new(media_import_node::IDENTIFIER.to_string());
  media_import_node1.properties.insert(
    media_import_node::INPUTS::CLIP.to_string(),
    serde_json::to_value(ClipIdentifier {
      id: source_clip1.clone(),
      clip_type: classes::clip::ClipType::Source,
    })
    .unwrap(),
  );

  let mut media_import_node2 = Node::new(media_import_node::IDENTIFIER.to_string());
  media_import_node2.properties.insert(
    media_import_node::INPUTS::CLIP.to_string(),
    serde_json::to_value(ClipIdentifier {
      id: source_clip2.clone(),
      clip_type: classes::clip::ClipType::Source,
    })
    .unwrap(),
  );
  let mut concat_node1 = Node::new(concat_node::IDENTIFIER.to_string());
  let mut output_node1 = Node::new(output_node::IDENTIFIER.to_string());
  output_node1.properties.insert(
    output_node::INPUTS::CLIP.to_string(),
    serde_json::to_value(ClipIdentifier {
      id: composited_clip1.clone(),
      clip_type: classes::clip::ClipType::Composited,
    })
    .unwrap(),
  );

  let mut nodes = HashMap::new();
  nodes.insert(media_import_node1.id.clone(), media_import_node1.clone());
  nodes.insert(media_import_node2.id.clone(), media_import_node2.clone());
  nodes.insert(concat_node1.id.clone(), concat_node1.clone());
  nodes.insert(output_node1.id.clone(), output_node1.clone());

  let mut pipelines = HashMap::new();
  let mut pipeline1 = Pipeline::new();
  pipeline1.target_node_id = Some(output_node1.id.clone());
  pipeline1.links.push(Link {
    from: LinkEndpoint {
      node_id: media_import_node1.id.clone(),
      property: media_import_node::OUTPUTS::OUTPUT.to_string(),
    },
    to: LinkEndpoint {
      node_id: concat_node1.id.clone(),
      property: concat_node::INPUTS::MEDIA1.to_string(),
    },
  });
  pipeline1.links.push(Link {
    from: LinkEndpoint {
      node_id: media_import_node2.id.clone(),
      property: media_import_node::OUTPUTS::OUTPUT.to_string(),
    },
    to: LinkEndpoint {
      node_id: concat_node1.id.clone(),
      property: concat_node::INPUTS::MEDIA2.to_string(),
    },
  });
  pipeline1.links.push(Link {
    from: LinkEndpoint {
      node_id: concat_node1.id.clone(),
      property: concat_node::OUTPUTS::OUTPUT.to_string(),
    },
    to: LinkEndpoint {
      node_id: output_node1.id.clone(),
      property: output_node::INPUTS::MEDIA.to_string(),
    },
  });
  // pipelines.insert("main".to_string(), pipeline1);
  let store = Store {
    nodes,
    clips: clip_store,
    pipelines: pipelines,
    node_types: get_node_register(),
    medias: HashMap::new(),
  };
  let res = pipeline1.generate_pipeline_string(&store).unwrap();
  println!("Result: {};", res);

  tauri::Builder::default()
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
