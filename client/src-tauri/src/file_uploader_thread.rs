use core::time;
use std::{
  fs::File,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use cs310_shared::{clip, networking};
use uuid::Uuid;

use crate::state_manager::{ConnectionStatus, SharedState};

pub fn file_uploader_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let x = shared_state.lock().unwrap();
    let rx = &x.thread_stopper;

    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        break;
      }
      Err(mpsc::TryRecvError::Empty) => {
        match x.connection_status {
          ConnectionStatus::InitialisingConnection
          | ConnectionStatus::InitialConnectionFailed(_) => {
            drop(x);
          }
          _ => {
            let store = x.store.as_ref();

            if let Some(store) = store {
              let mut clips = store.clips.source.clone();
              let mut clips: Vec<(&Uuid, &mut clip::SourceClip)> = clips
                .iter_mut()
                .filter(|(_, clip)| match clip.status {
                  clip::SourceClipServerStatus::LocalOnly => clip.original_file_location.is_some(),
                  _ => false,
                })
                .collect();

              if clips.len() > 0 {
                let (id, clip) = clips.first_mut().unwrap();
                clip.status = clip::SourceClipServerStatus::Uploading;

                let id = id.clone();
                let clip = clip.clone();

                drop(x);

                let stream = networking::connect_to_server();

                if let Ok(mut stream) = stream {
                  networking::send_message(&mut stream, networking::Message::UploadFile).unwrap();
                  networking::send_data(&mut stream, id.as_bytes()).unwrap();
                  let file_path = clip.original_file_location.unwrap();

                  let last_time_progress = Mutex::new(std::time::Instant::now());
                  let mut file = File::open(file_path.clone()).unwrap();

                  let state_clone = shared_state.clone();
                  networking::send_file_with_progress(
                    &mut stream,
                    &mut file,
                    |perc, bytes_complete| {
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
                    },
                  );
                  networking::send_message(&mut stream, networking::Message::EndFile).unwrap();

                  let mut lock_shared_state = shared_state.lock().unwrap();

                  if let Some(store) = &mut lock_shared_state.store {
                    let clip = store.clips.source.get_mut(&id);
                    if let Some(clip) = clip {
                      clip.status = clip::SourceClipServerStatus::Uploaded;
                    }
                  }
                  lock_shared_state
                    .window
                    .as_ref()
                    .unwrap()
                    .emit("file-upload-progress", (id, 100))
                    .unwrap();
                } else {
                  // cannot connect to server
                }
              } else {
                drop(x);
              }

              // https://github.com/sdroege/gstreamer-rs/blob/master/examples/src/bin/events.rs
            } else {
              drop(x);
            }
          }
        }
      }
    }
    thread::sleep(time::Duration::from_millis(1000));
  }
}
