use std::{
  collections::HashMap,
  fs::File,
  sync::{mpsc::Receiver, Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipIdentifier, ClipType, CompositedClip, SourceClip},
  constants::{CHUNK_FILENAME_NUMBER_LENGTH, CHUNK_LENGTH},
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

    // first, look at what has been requested by the front end, and set it as 'generating'
    let video_preview_status = &mut lock.video_preview_data;
    for (id, status) in video_preview_status.iter_mut() {
      match status {
        VideoPreviewStatus::Data(_, data) => {
          for status in data {
            match status {
              VideoPreviewChunkStatus::Requested => {
                *status = VideoPreviewChunkStatus::Generating;
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

    // then, look at the statuses...
    for (id, status) in video_preview_status {
      match status {
        VideoPreviewStatus::NotRequested => {
          // don't do anything if nothing's been requested
        }
        VideoPreviewStatus::LengthRequested => {
          let mut stream = networking::connect_to_server().unwrap();

          networking::send_message(&mut stream, networking::Message::CompositedClipLength).unwrap();
          let uuid_bytes = id.as_bytes();
          networking::send_data(&mut stream, uuid_bytes).unwrap();

          println!("Sent data for {}", id);

          let msg = networking::receive_message(&mut stream).unwrap();

          println!("Message received: {:?}", msg);

          match msg {
            networking::Message::CompositedClipLength => {
              let uuid = networking::receive_uuid(&mut stream);
              let duration = networking::receive_u64(&mut stream);
              let number_of_chunks = networking::receive_u32(&mut stream);

              let data_statuses =
                vec![VideoPreviewChunkStatus::NotRequested; number_of_chunks as usize];
              let status = VideoPreviewStatus::Data(duration, data_statuses);

              let mut lock = shared_state.lock().unwrap();
              lock.video_preview_data.insert(id.clone(), status);

              lock
                .window
                .as_ref()
                .unwrap()
                .emit("video-preview-data-update", lock.video_preview_data.clone())
                .unwrap();
            }
            networking::Message::CouldNotGetLength => {
              println!("Could not generate preview :/");
            }
            _ => {
              panic!("Unknown message!: {:?}", msg);
            }
          }
        }
        VideoPreviewStatus::Data(length, data) => {
          let mut start_chunk = None;
          let mut end_chunk = None;
          for i in 0..data.len() {
            let status = &data[i];
            match status {
              VideoPreviewChunkStatus::Generating => {
                if start_chunk.is_none() {
                  start_chunk = Some(i);
                }
                if end_chunk.is_none() {
                  end_chunk = Some(i);
                }
              }
              _ => {
                if start_chunk.is_some() {
                  break;
                }
              }
            }
          }

          if let (Some(start_chunk), Some(end_chunk)) = (start_chunk, end_chunk) {
            let mut stream = networking::connect_to_server().unwrap();
            networking::send_message(&mut stream, networking::Message::GetVideoPreview).unwrap();
            networking::send_data(&mut stream, id.as_bytes()).unwrap();
            networking::send_data(&mut stream, &start_chunk.to_ne_bytes()).unwrap();
            networking::send_data(&mut stream, &end_chunk.to_ne_bytes()).unwrap();

            loop {
              let message = networking::receive_message(&mut stream);
              match message {
                Ok(msg) => match msg {
                  networking::Message::CouldNotGeneratePreview => {
                    panic!("Could not generate preview!");
                  }
                  networking::Message::NewChunk => {
                    let chunk_id = networking::receive_u32(&mut stream);

                    let mut lock = shared_state.lock().unwrap();
                    let entry = lock.video_preview_data.get_mut(&id).unwrap();

                    match entry {
                      VideoPreviewStatus::Data(duration, data) => {
                        if data.get(chunk_id as usize).is_none() {
                          panic!("Something fucked up!");
                        }
                        data[chunk_id as usize] = VideoPreviewChunkStatus::Generated;

                        lock
                          .window
                          .as_ref()
                          .unwrap()
                          .emit("video-preview-data-update", lock.video_preview_data.clone())
                          .unwrap();
                      }
                      _ => {
                        panic!("Something fucked up!");
                      }
                    }
                  }
                  networking::Message::AllChunksGenerated => {
                    break;
                  }
                  _ => {}
                },
                Err(x) => {
                  println!("Error occurred (vid preview)! {:?}", x);
                }
              }
            }
          }
        }
      }
    }

    thread::sleep(Duration::from_secs(1));
  }
}

pub fn video_previewer_downloader_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut lock = shared_state.lock().unwrap();

    // first, look at what has been requested by the front end, and set it as 'generating'

    let video_preview_status = &mut lock.video_preview_data;

    let mut clip_ids = Vec::new();
    for (id, status) in video_preview_status.iter_mut() {
      match status {
        VideoPreviewStatus::Data(_, data) => {
          for status in data {
            match status {
              VideoPreviewChunkStatus::Generated => {
                *status = VideoPreviewChunkStatus::Downloading;
                if !clip_ids.contains(id) {
                  clip_ids.push(id.clone());
                }
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }

    let video_preview_status = video_preview_status.clone();

    let mut clips = HashMap::new();
    for id in clip_ids {
      clips.insert(
        id.clone(),
        lock
          .store
          .as_ref()
          .unwrap()
          .clips
          .composited
          .get(&id)
          .unwrap()
          .clone(),
      );
    }

    drop(lock);

    for (id, status) in video_preview_status {
      match status {
        VideoPreviewStatus::Data(_, data) => {
          for i in 0..(data.len()) {
            let status = &data[i];
            match status {
              VideoPreviewChunkStatus::Downloading => {
                let mut stream = networking::connect_to_server().unwrap();

                networking::send_message(&mut stream, networking::Message::DownloadChunk).unwrap();

                let chunk_id = i as u32;
                networking::send_data(&mut stream, id.as_bytes()).unwrap();
                networking::send_data(&mut stream, &chunk_id.to_ne_bytes()).unwrap();

                let clip = clips.get(&id).unwrap();
                let output_location = format!(
                  "{}/segment{:0>width$}.mp4",
                  clip.get_output_location(),
                  width = CHUNK_FILENAME_NUMBER_LENGTH as usize
                );

                std::fs::create_dir_all(clip.get_output_location()).unwrap();

                let mut file = File::create(output_location).unwrap();
                networking::receive_file(&mut stream, &mut file);

                let mut lock = shared_state.lock().unwrap();

                let existing_data = lock.video_preview_data.get_mut(&id).unwrap();
                match existing_data {
                  VideoPreviewStatus::Data(duration, data) => {
                    data[i] = VideoPreviewChunkStatus::Downloaded;

                    lock
                      .window
                      .as_ref()
                      .unwrap()
                      .emit("video-preview-data-update", lock.video_preview_data.clone())
                      .unwrap();
                  }
                  _ => {}
                }
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }
    thread::sleep(Duration::from_secs(1));
  }
}
