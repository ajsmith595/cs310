#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::{
  collections::HashMap,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use crate::{
  file_uploader_thread::file_uploader_thread, state::ConnectionStatus,
  task_manager::task_manager_thread, video_preview_handler_thread::video_preview_handler_thread,
  video_preview_handler_thread::video_previewer_downloader_thread,
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
  let path = tauri::api::path::data_dir().unwrap(); // Get the recommended directory for application data
  let path = format!(
    "{}\\AdamSmith\\VideoEditor",
    path.into_os_string().into_string().unwrap()
  );

  println!("Initialising...");
  init(path, false); // Sets up utility functions
  println!("Initialised");

  let register = get_node_register(); // Gets the complete register of node types

  let (tx, rx) = mpsc::channel(); // Thread stopper communication - allows a message to be send from the main thread, so that all threads can then be stopped
  let shared_state = SharedState {
    store: None,
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

  let thread_spawned = Arc::new(Mutex::new(false)); // Prevents threads from being spawned twice in the event that the page is reloaded
  let threads = Arc::new(Mutex::new(Some(Vec::new())));

  // For the page load callback
  let shared_state_clone = shared_state.clone();
  let threads_clone = threads.clone();

  tauri::Builder::default()
    .manage(SharedStateWrapper(shared_state))
    // Register all the Tauri commands available to the frontend
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
      tauri_commands::update_clip,
      tauri_commands::request_video_length,
      tauri_commands::request_video_preview,
      tauri_commands::get_video_preview_data
    ])
    .on_page_load(move |app, _| {
      let threads = threads_clone.clone();
      if *thread_spawned.lock().unwrap() {
        // Do not spawn threads again if they're already spawned
        return;
      }
      *thread_spawned.lock().unwrap() = true;

      let window = app.get_window("main").unwrap();
      shared_state_clone.clone().lock().as_mut().unwrap().window = Some(window);

      let threads_to_spawn = [
        store_fetcher_thread,
        file_uploader_thread,
        network_task_manager_thread,
        video_preview_handler_thread,
        video_previewer_downloader_thread,
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
          shared_state.lock().unwrap().task_manager_notifier = Some(task_manager_notifier); // Register the special communication setup for the task manager
          task_manager_thread(shared_state, task_manager_receiver);
        }))
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  let _ = tx.send(()); // send a request to the thread stopper when the window closes

  let threads = threads.lock().unwrap().take().unwrap();
  for t in threads {
    t.join().unwrap(); // then join all the threads
  }
}
