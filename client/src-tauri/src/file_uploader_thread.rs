use crate::state::{ConnectionStatus, SharedState};
use core::time;
use cs310_shared::{
  clip::{self, SourceClip},
  networking,
};
use std::{
  fs::File,
  sync::{mpsc, Arc, Mutex},
  thread,
};
use uuid::Uuid;

/**
 * Goes through all the source clips which are not yet uploaded, and uploads them to the server
 */
pub fn file_uploader_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut state = shared_state.lock().unwrap();
    let rx = &state.thread_stopper;

    // We use `drop(state)` to unlock the lock when we're done with it
    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        break;
      }
      Err(mpsc::TryRecvError::Empty) => {
        match state.connection_status {
          ConnectionStatus::InitialisingConnection
          | ConnectionStatus::InitialConnectionFailed(_) => {
            drop(state);
          }
          _ => {
            // Initial connection to the server is fine

            let store = state.store.as_mut();

            if let Some(store) = store {
              let clips = &mut store.clips.source;
              let mut clips: Vec<(&Uuid, &mut clip::SourceClip)> = clips
                .iter_mut()
                .filter(|(_, clip)| match clip.status {
                  clip::SourceClipServerStatus::LocalOnly => clip.original_file_location.is_some(), // Get all the clips which have not been uploaded yet
                  _ => false,
                })
                .collect();

              if clips.len() > 0 {
                // We only upload one clip at a time, so we just get the first one
                let (id, clip) = clips.first_mut().unwrap();
                clip.status = clip::SourceClipServerStatus::Uploading;

                let id = id.clone();
                let clip = clip.clone();

                let store_clone = store.clone();
                drop(state);
                let mut state = shared_state.lock().unwrap();
                state
                  .window
                  .as_mut()
                  .unwrap()
                  .emit("store-update", store_clone)
                  .unwrap();

                drop(state);

                let res = upload_file(id.clone(), clip.clone(), shared_state.clone());

                let mut lock_shared_state = shared_state.lock().unwrap();

                if let Some(store) = &mut lock_shared_state.store {
                  let clip = store.clips.source.get_mut(&id);
                  if let Some(clip) = clip {
                    clip.status = match res {
                      Ok(()) => clip::SourceClipServerStatus::Uploaded,
                      Err(err) => clip::SourceClipServerStatus::LocalOnly,
                    };
                  }
                }
                lock_shared_state
                  .window
                  .as_ref()
                  .unwrap()
                  .emit("store-update", lock_shared_state.store.clone())
                  .unwrap();
              } else {
                drop(state);
              }
            } else {
              drop(state);
            }
          }
        }
      }
    }

    thread::sleep(time::Duration::from_millis(1000));
  }
}

/**
 * Uploads a particular file to the server, with error handling
*/
fn upload_file(
  id: Uuid,
  clip: SourceClip,
  shared_state: Arc<Mutex<SharedState>>,
) -> Result<(), std::io::Error> {
  let mut stream = networking::connect_to_server()?;

  networking::send_message(&mut stream, networking::Message::UploadFile)?;
  networking::send_data(&mut stream, id.as_bytes())?;
  let file_path = clip.original_file_location.unwrap();

  let last_time_progress = Mutex::new(std::time::Instant::now());
  let mut file = File::open(file_path.clone()).unwrap();

  let state_clone = shared_state.clone();

  // We send the file with progress callback
  networking::send_file_with_progress(&mut stream, &mut file, |perc, bytes_complete| {
    let now = std::time::Instant::now();
    let mut prev_time = last_time_progress.lock().unwrap();
    let duration = now.duration_since(*prev_time).as_millis();

    if duration > 100 {
      // do an update at most every 0.1s
      *prev_time = now;
      match &state_clone.lock().unwrap().window {
        Some(window) => {
          window.emit("file-upload-progress", (id, perc)).unwrap(); // We notify the UI so it can display the progress of the upload to the user
        }
        None => {}
      }
    }
  })?;
  networking::send_message(&mut stream, networking::Message::EndFile)?;

  Ok(())
}
