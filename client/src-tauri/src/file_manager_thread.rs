use core::time;
use std::{
  collections::HashMap,
  fs,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use cs310_shared::classes::state_manager::SharedState;

pub fn APPLICATION_DATA_ROOT() -> String {
  let path = dirs::data_dir().unwrap();
  format!(
    "{}\\AdamSmith\\VideoEditor",
    path.into_os_string().into_string().unwrap()
  )
}
pub fn APPLICATION_MEDIA_OUTPUT() -> String {
  format!("{}\\output", APPLICATION_DATA_ROOT())
}
pub fn APPLICATION_JSON_PATH() -> String {
  format!("{}\\pipeline.json", APPLICATION_DATA_ROOT())
}

pub fn file_manager_thread(shared_state: Arc<Mutex<SharedState>>) {
  let mut path = None;
  {
    match dirs::data_dir() {
      Some(p) => {
        path = Some(p.join(APPLICATION_JSON_PATH()));
        let mut directory = path.clone().unwrap();
        directory.pop();
        if !directory.exists() {
          fs::create_dir_all(directory);
        }
      }
      None => println!("Cannot get data directory!"),
    }
  }

  loop {
    let x = shared_state.lock().unwrap();
    let rx = &x.thread_stopper;
    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        break;
      }
      Err(mpsc::TryRecvError::Empty) => {
        drop(x);
        if !shared_state.lock().unwrap().file_written {
          println!("Saving new state to file...");
          let mut locked_state = shared_state.lock().unwrap();
          match path {
            Some(ref p) => {
              std::fs::write(p, serde_json::to_string(&locked_state.store).unwrap())
                .expect(format!("Cannot write file '{}'", p.to_str().unwrap()).as_str());
            }
            None => println!("No path to write to"),
          }

          locked_state.file_written = true;
          drop(locked_state);
          // https://github.com/sdroege/gstreamer-rs/blob/master/examples/src/bin/events.rs
        }
      }
    }
    thread::sleep(time::Duration::from_millis(1000));
  }
}
