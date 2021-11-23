#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{
  cell::Cell,
  collections::HashMap,
  fs::File,
  io::Write,
  sync::{mpsc, Arc, Mutex},
};

use gstreamer::{glib, prelude::*};
use uuid::Uuid;

use crate::{
  classes::{
    clip::{ClipIdentifier, CompositedClip, SourceClip},
    global::uniq_id,
    node::{Node, Position},
    nodes::{concat_node, get_node_register, media_import_node, output_node},
    pipeline::{Link, LinkEndpoint, Pipeline},
    store::{ClipStore, Store},
  },
  state_manager::{SharedState, SharedStateWrapper, StoredState},
};

use tauri::{
  utils::config::AppUrl, CustomMenuItem, Manager, Menu, MenuItem, Submenu, WindowBuilder,
};

#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate erased_serde;
// extern crate dirs;
extern crate gstreamer;
extern crate gstreamer_pbutils;
extern crate serde;
extern crate serde_json;

mod classes;
mod state_manager;
mod tauri_commands;

fn main() {
  let store;

  let f = std::fs::read("state.json");

  match f {
    Ok(data) => {
      store = serde_json::from_slice(&data).unwrap();
    }
    _ => {
      let mut clip_store = ClipStore::new();
      let source_clip1;
      let composited_clip1;
      let source_clip2;
      {
        source_clip1 = uniq_id();
        clip_store.source.insert(
          source_clip1.clone(),
          SourceClip {
            id: source_clip1.clone(),
            name: "Test Clip 1".to_string(),
            file_location: "input/test_input.mp4".to_string(),
            thumbnail_location: None,
          },
        );

        source_clip2 = uniq_id();
        clip_store.source.insert(
          source_clip2.clone(),
          SourceClip {
            id: source_clip2.clone(),
            name: "Test Clip 2".to_string(),
            file_location: "input/test_input2.mp4".to_string(),
            thumbnail_location: None,
          },
        );

        composited_clip1 = uniq_id();
        clip_store.composited.insert(
          composited_clip1.clone(),
          CompositedClip {
            id: composited_clip1.clone(),
            name: "Test Composited Clip".to_string(),
          },
        );
      }

      let group_id = uniq_id();

      let mut media_import_node1 = Node::new(
        media_import_node::IDENTIFIER.to_string(),
        Some(group_id.clone()),
      );
      media_import_node1.properties.insert(
        media_import_node::INPUTS::CLIP.to_string(),
        serde_json::to_value(ClipIdentifier {
          id: source_clip1.clone(),
          clip_type: classes::clip::ClipType::Source,
        })
        .unwrap(),
      );

      let mut media_import_node2 = Node::new(
        media_import_node::IDENTIFIER.to_string(),
        Some(group_id.clone()),
      );
      media_import_node2.properties.insert(
        media_import_node::INPUTS::CLIP.to_string(),
        serde_json::to_value(ClipIdentifier {
          id: source_clip2.clone(),
          clip_type: classes::clip::ClipType::Source,
        })
        .unwrap(),
      );
      let mut concat_node1 = Node::new(concat_node::IDENTIFIER.to_string(), Some(group_id.clone()));
      let mut output_node1 = Node::new(output_node::IDENTIFIER.to_string(), Some(group_id.clone()));
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

      let mut pipeline = Pipeline::new();
      pipeline.target_node_id = Some(output_node1.id.clone());
      pipeline.links.push(Link {
        from: LinkEndpoint {
          node_id: media_import_node1.id.clone(),
          property: media_import_node::OUTPUTS::OUTPUT.to_string(),
        },
        to: LinkEndpoint {
          node_id: concat_node1.id.clone(),
          property: concat_node::INPUTS::MEDIA1.to_string(),
        },
      });
      pipeline.links.push(Link {
        from: LinkEndpoint {
          node_id: media_import_node2.id.clone(),
          property: media_import_node::OUTPUTS::OUTPUT.to_string(),
        },
        to: LinkEndpoint {
          node_id: concat_node1.id.clone(),
          property: concat_node::INPUTS::MEDIA2.to_string(),
        },
      });
      pipeline.links.push(Link {
        from: LinkEndpoint {
          node_id: concat_node1.id.clone(),
          property: concat_node::OUTPUTS::OUTPUT.to_string(),
        },
        to: LinkEndpoint {
          node_id: output_node1.id.clone(),
          property: output_node::INPUTS::MEDIA.to_string(),
        },
      });
      store = Store {
        nodes,
        clips: clip_store,
        pipeline,
        medias: HashMap::new(),
      };
    }
  }
  let register = get_node_register();

  println!("{}", serde_json::ser::to_string(&store).unwrap());

  let res = store.pipeline.generate_pipeline_string(&store, &register);
  println!("Result: {:#?};", res);

  gstreamer::init().expect("GStreamer could not be initialised");
  // execute_pipeline(res, 60);
  // println!("Pipeline executed");

  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&store).unwrap().as_bytes())
    .unwrap();
  let shared_state = SharedState {
    stored_state: StoredState {
      store,
      file_written: false,
    },
    window: None,
    node_register: register.clone(),
  };

  let shared_state = Arc::new(Mutex::new(shared_state));

  let shared_state_clone = shared_state.clone();
  tauri::Builder::default()
    .manage(SharedStateWrapper(shared_state))
    .invoke_handler(tauri::generate_handler![
      tauri_commands::import_media,
      tauri_commands::get_initial_data,
      tauri_commands::change_clip_name,
      tauri_commands::create_composited_clip,
      tauri_commands::get_node_outputs,
      tauri_commands::update_node,
      tauri_commands::store_update,
      tauri_commands::get_file_info
    ])
    .setup(move |app| {
      let window = app.get_window("main").unwrap();

      let shared_state = shared_state_clone.clone();
      let temp = shared_state.clone();
      let x = &mut temp.lock().unwrap();
      x.window = Some(window);
      drop(x);
      // thread::spawn(move || {
      //   pipeline_executor_thread(shared_state);
      // });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
