use std::{
  collections::HashMap,
  sync::{mpsc::Receiver, Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipIdentifier, ClipType, CompositedClip, SourceClip},
  networking,
  node::{Node, Position},
  pipeline::Link,
  store::Store,
  task::Task,
  ID,
};
use serde_json::Value;
use uuid::Uuid;

use crate::state::{SharedState, VideoPreviewChunkStatus, VideoPreviewStatus};

pub fn video_preview_handler_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut lock = shared_state.lock().unwrap();

    let video_preview_status = &mut lock.video_preview_data;
    for (id, status) in video_preview_status.iter_mut() {
      match status {
        VideoPreviewStatus::Data(data) => {
          for status in data {
            match status {
              VideoPreviewChunkStatus::Requested => {
                *status = VideoPreviewChunkStatus::Requesting;
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }

    let video_preview_status = video_preview_status.clone();
    drop(lock);

    for (id, status) in video_preview_status {
      match status {
        VideoPreviewStatus::NotRequested => {}
        VideoPreviewStatus::LengthRequested => {
          let mut stream = networking::connect_to_server().unwrap();

          networking::send_message(&mut stream, networking::Message::CompositedClipLength).unwrap();
          let uuid_bytes = id.as_bytes();
          networking::send_data(&mut stream, uuid_bytes).unwrap();
        }
        VideoPreviewStatus::Data(_) => todo!(),
      }
    }
  }
}
