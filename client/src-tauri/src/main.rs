#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use core::{panic, time};
use std::{
  cell::Cell,
  collections::HashMap,
  fs::{create_dir_all, File},
  io::Write,
  net::TcpStream,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use file_manager_thread::{APPLICATION_DATA_ROOT, APPLICATION_MEDIA_OUTPUT};
use gstreamer::{glib, prelude::*};
use uuid::Uuid;

use crate::file_manager_thread::{file_manager_thread, APPLICATION_JSON_PATH};
use crate::pipeline_executor_thread::pipeline_executor_thread;
use cs310_shared::{
  clip::{ClipIdentifier, CompositedClip, SourceClip},
  constants::init,
  global::uniq_id,
  networking::{self, Message, SERVER_HOST, SERVER_PORT},
  node::{Node, Position},
  nodes::{concat_node, get_node_register, media_import_node, output_node},
  pipeline::{Link, LinkEndpoint, Pipeline},
  store::{ClipStore, Store},
};

use crate::state_manager::{SharedState, SharedStateWrapper};

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

mod file_manager_thread;
mod pipeline_executor_thread;
mod state_manager;
mod tauri_commands;

// fn main2() {
//   let mut stream = TcpStream::connect(format!("{}:{}", SERVER_HOST, SERVER_PORT));

//   if stream.is_err() {
//     panic!(&stream.unwrap_err().to_string()[..]);
//   }
//   let mut stream = stream.unwrap();

//   networking::send_message(&mut stream, Message::GetStore).unwrap();
//   let (message, data) = networking::receive_message(&mut stream).unwrap();
//   let mut new_data = [0 as u8; 8];
//   new_data.clone_from_slice(&data[0..8]);
//   let data_length = u64::from_ne_bytes(new_data);

//   let data = networking::receive_data(&mut stream, data_length).unwrap();
//   let str = String::from_utf8(data).unwrap();
//   println!("Received: {}", str);

//   thread::sleep(time::Duration::from_millis(1000));

//   networking::send_message(&mut stream, Message::GetStore).unwrap();
//   let (message, data) = networking::receive_message(&mut stream).unwrap();
//   let mut new_data = [0 as u8; 8];
//   new_data.clone_from_slice(&data[0..8]);
//   let data_length = u64::from_ne_bytes(new_data);

//   let data = networking::receive_data(&mut stream, data_length).unwrap();
//   let str = String::from_utf8(data).unwrap();
//   println!("Received: {}", str);

//   stream.shutdown(std::net::Shutdown::Both).unwrap();
// }

fn main() {
  if let Some(directory) = dirs::data_dir() {
    if !directory.join(APPLICATION_MEDIA_OUTPUT()).exists() {
      create_dir_all(directory.join(APPLICATION_MEDIA_OUTPUT()));
    }
  }

  let mut path = None;
  match dirs::data_dir() {
    Some(p) => {
      path = Some(p.join(APPLICATION_JSON_PATH()));
    }
    None => println!("Cannot get data directory!"),
  }

  init(APPLICATION_DATA_ROOT());

  let mut file = String::from("state.json");
  if path.is_some() {
    let path = path.unwrap().clone();
    file = String::from(path.to_str().unwrap());
  }

  let store = Store::from_file(file);

  let store = match store {
    Ok(store) => store,
    Err(_) => Store::new(APPLICATION_MEDIA_OUTPUT()),
  };

  let register = get_node_register();

  let res = store.pipeline.gen_graph_new(&store, &register);
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
    pipeline_executed: false,
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
      tauri_commands::get_node_inputs,
      tauri_commands::get_output_directory,
    ])
    .setup(move |app| {
      let window = app.get_window("main").unwrap();

      let temp = shared_state_clone.clone();
      let x = &mut temp.lock().unwrap();
      x.window = Some(window);
      drop(x);

      {
        let shared_state = shared_state_clone.clone();
        thread::spawn(move || {
          file_manager_thread(shared_state);
        });
      }
      // {
      //   let shared_state = shared_state_clone.clone();
      //   thread::spawn(move || {
      //     pipeline_executor_thread(shared_state);
      //   });
      // };
      // we no longer run the pipeline on the client!

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  let _ = tx.send(());
}
