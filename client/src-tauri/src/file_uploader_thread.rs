use core::time;
use std::{
  collections::HashMap,
  fs::{self, File},
  sync::{mpsc, Arc, Mutex},
  thread,
};

use cs310_shared::{
  clip,
  constants::{
    media_output_location, store_json_location, CHUNK_FILENAME_NUMBER_LENGTH, CHUNK_LENGTH,
  },
  networking::{self, send_message, Message},
};
use uuid::Uuid;

use crate::state_manager::SharedState;

pub fn file_uploader_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut x = shared_state.lock().unwrap();
    let rx = &x.thread_stopper;

    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        break;
      }
      Err(mpsc::TryRecvError::Empty) => {
        let mut clips: Vec<(&Uuid, &mut clip::SourceClip)> = x
          .store
          .clips
          .source
          .iter_mut()
          .filter(|(id, clip)| match clip.status {
            clip::SourceClipServerStatus::LocalOnly => true,
            _ => false,
          })
          .collect();

        if clips.len() > 0 {
          let (id, clip) = clips.first_mut().unwrap();
          clip.status = clip::SourceClipServerStatus::Uploading;

          let id = id.clone();
          let clip = clip.clone();

          drop(x);

          let mut stream = networking::connect_to_server();

          networking::send_message(&mut stream, networking::Message::UploadFile).unwrap();
          networking::send_data(&mut stream, id.as_bytes()).unwrap();
          let file_path = clip.file_location;

          let last_time_progress = Mutex::new(std::time::Instant::now());
          let mut file = File::open(file_path.clone()).unwrap();

          let state_clone = shared_state.clone();
          networking::send_file_with_progress(&mut stream, &mut file, |perc, bytes_complete| {
            let now = std::time::Instant::now();
            let mut prev_time = last_time_progress.lock().unwrap();
            let duration = now.duration_since(*prev_time).as_millis();

            if duration > 20 {
              // do an update at most every 300 ms
              println!("Perc complete: {}%; bytes done: {}", perc, bytes_complete);
              *prev_time = now;

              match &state_clone.lock().unwrap().window {
                Some(window) => {
                  window.emit("file-upload-progress", (id, perc)).unwrap();
                }
                None => todo!(),
              }
            }
          });
          networking::send_message(&mut stream, networking::Message::EndFile).unwrap();

          shared_state
            .lock()
            .unwrap()
            .window
            .as_ref()
            .unwrap()
            .emit("file-upload-progress", (id, 100))
            .unwrap();
        } else {
          drop(x);
        }

        // https://github.com/sdroege/gstreamer-rs/blob/master/examples/src/bin/events.rs
      }
    }
    thread::sleep(time::Duration::from_millis(1000));
  }
}
