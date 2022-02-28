#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use core::time;
use std::{
  collections::HashMap,
  fs::File,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use crate::{
  file_uploader_thread::file_uploader_thread, state::ConnectionStatus,
  task_manager::task_manager_thread,
};
use crate::{
  network_task_manager::network_task_manager_thread, store_fetcher_thread::store_fetcher_thread,
};
use cs310_shared::{
  constants::{init, store_json_location},
  networking::{self, Message},
  nodes::get_node_register,
  store::Store,
};

use crate::state::{SharedState, SharedStateWrapper};

use tauri::Manager;

#[macro_use]
extern crate serde_derive;
// #[macro_use]
// extern crate erased_serde;
// extern crate dirs;
extern crate dirs;
extern crate gst;
extern crate gst_pbutils;
extern crate serde;
extern crate serde_json;

mod file_uploader_thread;
mod network_task_manager;
mod state;
mod store_fetcher_thread;
mod task_manager;
mod tauri_commands;
mod video_preview_handler_thread;

fn main() {
  let path = dirs::data_dir().unwrap();
  let path = format!(
    "{}\\AdamSmith\\VideoEditor",
    path.into_os_string().into_string().unwrap()
  );

  println!("Initialising...");
  init(path, false);
  println!("Initialised");

  let register = get_node_register();
  let (tx, rx) = mpsc::channel();

  let shared_state = SharedState {
    store: None,
    file_written: false,
    connection_status: ConnectionStatus::InitialisingConnection,
    window: None,
    node_register: register.clone(),
    thread_stopper: rx,
    task_manager_notifier: None,
    tasks: Vec::new(),
    network_jobs: Vec::new(),
    video_preview_data: HashMap::new(),
  };

  let shared_state = Arc::new(Mutex::new(shared_state));

  let shared_state_clone = shared_state.clone();

  let thread_spawned = Arc::new(Mutex::new(false));

  let threads = Arc::new(Mutex::new(Some(Vec::new())));
  let threads_clone = threads.clone();

  tauri::Builder::default()
    .manage(SharedStateWrapper(shared_state))
    .invoke_handler(tauri::generate_handler![
      tauri_commands::import_media,
      tauri_commands::get_initial_data,
      tauri_commands::get_node_outputs,
      tauri_commands::get_node_inputs,
      tauri_commands::get_output_directory,
      tauri_commands::get_clip_type,
      tauri_commands::get_connection_status,
      tauri_commands::create_composited_clip,
      tauri_commands::add_link,
      tauri_commands::update_node,
      tauri_commands::add_node,
      tauri_commands::delete_node,
      tauri_commands::delete_links,
      tauri_commands::update_clip
    ])
    .on_page_load(move |app, _ev| {
      let threads = threads_clone.clone();
      if *thread_spawned.lock().unwrap() {
        println!("Not starting threads again!");
        return;
      }
      *thread_spawned.lock().unwrap() = true;
      println!("Starting up threads...");

      let window = app.get_window("main").unwrap();

      let temp = shared_state_clone.clone();
      let x = &mut temp.lock().unwrap();
      x.window = Some(window);
      drop(x);
      let threads_to_spawn = [
        store_fetcher_thread,
        file_uploader_thread,
        network_task_manager_thread,
      ];

      let mut threads_lock = threads.lock().unwrap();
      let mutable_lock = threads_lock.as_mut().unwrap();
      for thread in threads_to_spawn {
        let shared_state = shared_state_clone.clone();
        mutable_lock.push(thread::spawn(move || {
          thread(shared_state);
        }))
      }

      {
        let shared_state = shared_state_clone.clone();
        mutable_lock.push(thread::spawn(move || {
          let (task_manager_notifier, task_manager_receiver) = mpsc::channel();
          shared_state.lock().unwrap().task_manager_notifier = Some(task_manager_notifier);
          task_manager_thread(shared_state, task_manager_receiver);
        }))
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  let _ = tx.send(());

  let threads = threads.lock().unwrap().take().unwrap();
  for t in threads {
    t.join().unwrap();
  }
}
