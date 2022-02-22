#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use core::time;
use std::{
  fs::File,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use crate::state_uploader_thread::state_uploader_thread;
use crate::{file_uploader_thread::file_uploader_thread, state_manager::ConnectionStatus};
use cs310_shared::{
  constants::{init, store_json_location},
  networking::{self, Message},
  nodes::get_node_register,
  store::Store,
};

use crate::state_manager::{SharedState, SharedStateWrapper};

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
mod state_manager;
mod state_uploader_thread;
mod tauri_commands;

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
  };

  let shared_state = Arc::new(Mutex::new(shared_state));

  let shared_state_clone = shared_state.clone();

  let thread_spawned = Arc::new(Mutex::new(false));

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
      tauri_commands::get_node_inputs,
      tauri_commands::get_output_directory,
      tauri_commands::get_clip_type,
      tauri_commands::get_connection_status
    ])
    .on_page_load(move |app, _ev| {
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

      {
        let shared_state = shared_state_clone.clone();
        thread::spawn(move || {
          store_fetcher_thread(shared_state);
        });
      }

      {
        let shared_state = shared_state_clone.clone();
        thread::spawn(move || {
          state_uploader_thread(shared_state);
        });
      }
      {
        let shared_state = shared_state_clone.clone();
        thread::spawn(move || {
          file_uploader_thread(shared_state);
        });
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  let _ = tx.send(());
}

fn store_fetcher_thread(state: Arc<Mutex<SharedState>>) {
  loop {
    set_connection_status(&state, ConnectionStatus::InitialisingConnection);

    let stream = networking::connect_to_server();

    if let Ok(mut stream) = stream {
      networking::send_message(&mut stream, Message::GetStore).unwrap();
      let mut json_file = File::create(store_json_location()).unwrap();
      networking::receive_file(&mut stream, &mut json_file);

      let store = Store::from_file(store_json_location());

      if let Ok(store) = store {
        set_connection_status(&state, ConnectionStatus::Connected);
        let mut state = state.lock().unwrap();
        state.store = Some(store);

        break;
      } else {
        set_connection_status(
          &state,
          ConnectionStatus::InitialConnectionFailed(format!("Invalid server response")),
        );
      }
    } else {
      set_connection_status(
        &state,
        ConnectionStatus::InitialConnectionFailed(stream.unwrap_err().to_string()),
      );
    }

    thread::sleep(time::Duration::from_secs(5));
  }
  println!("Store fetcher thread complete");
}

fn set_connection_status(state: &Arc<Mutex<SharedState>>, status: ConnectionStatus) {
  let mut state = state.lock().unwrap();

  state.connection_status = status.clone();

  if let Some(window) = &state.window {
    window.emit("connection-status", status.clone()).unwrap();
  }
}
