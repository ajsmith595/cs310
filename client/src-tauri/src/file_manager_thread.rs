use core::time;
use std::{
  collections::HashMap,
  fs::{self, File},
  sync::{mpsc, Arc, Mutex},
  thread,
};

use cs310_shared::{
  constants::CHUNK_FILENAME_NUMBER_LENGTH,
  networking::{self, send_message, Message},
};
use uuid::Uuid;

use crate::state_manager::SharedState;

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
              let json_string = serde_json::to_string(&locked_state.store).unwrap();
              std::fs::write(p, json_string)
                .expect(format!("Cannot write file '{}'", p.to_str().unwrap()).as_str());

              let mut stream = networking::connect_to_server();

              networking::send_message(&mut stream, networking::Message::SetStore).unwrap();
              let mut file = File::open(p).unwrap();
              networking::send_file(&mut stream, &mut file);

              while match networking::receive_message(&mut stream) {
                Ok(Message::NewChunk) => true,
                _ => false,
              } {
                let temp = networking::receive_data(&mut stream, 16).unwrap();
                let mut node_id_data = [0 as u8; 16];
                node_id_data.copy_from_slice(&temp);
                let node_id = Uuid::from_bytes(node_id_data);
                println!("Node id received: {}", node_id);

                let temp = networking::receive_data(&mut stream, 4).unwrap();
                let mut segment_number_bytes = [0 as u8; 4];
                segment_number_bytes.copy_from_slice(&temp);

                let segment_number = u32::from_le_bytes(segment_number_bytes);
                println!("Segment number: {}", segment_number);

                let clip = locked_state.store.clips.composited.get(&node_id).unwrap();
                let filename = format!(
                  "{}/segment{:0width$}.mp4",
                  clip.get_output_location(),
                  segment_number,
                  width = CHUNK_FILENAME_NUMBER_LENGTH as usize
                );
                fs::create_dir_all(clip.get_output_location()).unwrap();

                let mut file = File::create(filename).unwrap();
                networking::receive_file(&mut stream, &mut file);
                println!("Chunk received");
              }
              println!("All chunks done");
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
