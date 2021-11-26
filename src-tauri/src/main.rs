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
  thread,
};

use gstreamer::{glib, prelude::*};
use uuid::Uuid;

use crate::file_manager_thread::{file_manager_thread, APPLICATION_JSON_PATH};
use crate::{
  classes::{
    clip::{ClipIdentifier, CompositedClip, SourceClip},
    global::uniq_id,
    node::{Node, Position},
    nodes::{concat_node, get_node_register, media_import_node, output_node},
    pipeline::{Link, LinkEndpoint, Pipeline},
    store::{ClipStore, Store},
  },
  state_manager::{SharedState, SharedStateWrapper},
};

use tauri::{
  utils::config::AppUrl, CustomMenuItem, Manager, Menu, MenuItem, Submenu, WindowBuilder,
};

#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate erased_serde;
// extern crate dirs;
extern crate dirs;
extern crate gstreamer;
extern crate gstreamer_pbutils;
extern crate serde;
extern crate serde_json;

mod classes;
mod file_manager_thread;
mod state_manager;
mod tauri_commands;

fn main() {
  let store;

  let mut path = None;
  match dirs::data_dir() {
    Some(p) => {
      path = Some(p.join(APPLICATION_JSON_PATH.to_string()));
    }
    None => println!("Cannot get data directory!"),
  }

  let mut file = String::from("state.json");
  if path.is_some() {
    let path = path.unwrap().clone();
    file = String::from(path.to_str().unwrap());
  }
  let f = std::fs::read(file);

  match f {
    Ok(data) => {
      store = serde_json::from_slice(&data).unwrap();
    }
    _ => {
      let mut clip_store = ClipStore::new();
      let mut nodes = HashMap::new();
      let mut pipeline = Pipeline::new();
      pipeline.target_node_id = None;
      store = Store {
        nodes,
        clips: clip_store,
        pipeline,
        medias: HashMap::new(),
      };
    }
  }
  let register = get_node_register();

  let res = store.pipeline.generate_pipeline_string(&store, &register);
  if res.is_err() {
    println!("Result (error): {};", res.unwrap_err());
  }

  gstreamer::init().expect("GStreamer could not be initialised");

  let (tx, rx) = mpsc::channel();
  let shared_state = SharedState {
    store,
    file_written: false,
    window: None,
    node_register: register.clone(),
    thread_stopper: rx,
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
      tauri_commands::get_file_info,
      tauri_commands::get_node_inputs
    ])
    .setup(move |app| {
      let window = app.get_window("main").unwrap();

      let shared_state = shared_state_clone.clone();
      let temp = shared_state.clone();
      let x = &mut temp.lock().unwrap();
      x.window = Some(window);
      drop(x);

      thread::spawn(move || {
        file_manager_thread(shared_state);
      });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  let _ = tx.send(());
}
