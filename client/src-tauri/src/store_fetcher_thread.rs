use core::time;
use std::{
  collections::HashMap,
  fs::File,
  sync::{Arc, Mutex},
  thread,
};

use cs310_shared::{
  constants::store_json_location,
  networking::{self, Message},
  store::Store,
};

use crate::state::{ConnectionStatus, SharedState, VideoPreviewStatus};

/**
 * The thread that gets the initial state of the application from the server upon startup
 */
pub fn store_fetcher_thread(state: Arc<Mutex<SharedState>>) {
  loop {
    // Keep trying until successful
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
        let mut video_preview_data = HashMap::new();
        for (id, clip) in &store.clips.composited {
          video_preview_data.insert(id.clone(), VideoPreviewStatus::NotRequested);
        }
        state.store = Some(store);
        state.video_preview_data = video_preview_data;

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
