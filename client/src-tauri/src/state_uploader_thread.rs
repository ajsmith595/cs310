use core::time;
use std::{
  collections::HashMap,
  fs::{self, File},
  sync::{mpsc, Arc, Mutex},
  thread,
};

use cs310_shared::{
  constants::{
    media_output_location, store_json_location, CHUNK_FILENAME_NUMBER_LENGTH, CHUNK_LENGTH,
  },
  networking::{self, send_message, Message},
};
use ges::prelude::DiscovererStreamInfoExt;
use glib::Cast;
use gst_pbutils::{DiscovererAudioInfo, DiscovererVideoInfo};
use uuid::Uuid;

use crate::state_manager::SharedState;

#[derive(Serialize)]
struct VideoPreviewSend {
  pub output_directory_path: String,
  pub segment_duration: i32,
}
pub fn state_uploader_thread(shared_state: Arc<Mutex<SharedState>>) {
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

          let json_string = serde_json::to_string(&locked_state.store).unwrap();
          let composited_clips = locked_state.store.clips.composited.clone();
          std::fs::write(store_json_location(), json_string)
            .expect(format!("Cannot write file '{}'", store_json_location()).as_str());

          locked_state.file_written = true;
          drop(locked_state);

          let mut stream = networking::connect_to_server();

          networking::send_message(&mut stream, networking::Message::SetStore).unwrap();
          let mut file = File::open(store_json_location()).unwrap();
          networking::send_file(&mut stream, &mut file);

          loop {
            match networking::receive_message(&mut stream) {
              Ok(Message::NewChunk) => {
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

                let clip = composited_clips.get(&node_id).unwrap();
                let filename = format!(
                  "{}/segment{:0width$}.mp4",
                  clip.get_output_location(),
                  segment_number,
                  width = CHUNK_FILENAME_NUMBER_LENGTH as usize
                );
                fs::create_dir_all(clip.get_output_location()).unwrap();

                let mut file = File::create(filename.clone()).unwrap();
                networking::receive_file(&mut stream, &mut file);
                println!("Chunk received");

                if segment_number == 0 {
                  let discoverer =
                    gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(10)).unwrap();

                  let info = discoverer
                    .discover_uri(format!("file:///{}", filename).as_str())
                    .unwrap();

                  let video_streams = info.video_streams();

                  let mut codec_string = String::from("");
                  for video_stream in video_streams {
                    let structure = video_stream.caps().unwrap();
                    let structure = structure.structure(0).unwrap();

                    let codec_data: gst::Buffer = structure.get("codec_data").unwrap();
                    let codec_data = codec_data.map_readable().unwrap();
                    let codec_data = codec_data.as_slice();

                    let mut byte_number = 0;
                    let mut string = String::from("");
                    for byte in codec_data {
                      if byte_number >= 1 {
                        string = format!("{}{:02x}", string, byte);
                      }

                      byte_number += 1;
                      if byte_number > 3 {
                        break;
                      }
                    }

                    println!("Video stream codec string: {}", string);
                    if codec_string.len() > 0 {
                      codec_string = format!("{},", codec_string);
                    }
                    codec_string = format!("{}avc1.{}", codec_string, string);

                    let video_info = video_stream.clone().downcast::<DiscovererVideoInfo>();
                    if let Ok(video_info) = video_info {
                      let mut caps = video_stream.caps().unwrap();
                      for x in caps.iter() {
                        println!("Caps stuff (iter): {:#?}", x);

                        println!("Name: {}", x.name());
                        for field in x.fields() {
                          println!("Field: {}", field);
                        }
                      }

                      println!("CAPS: {:#?}", caps);
                      caps.simplify();
                      println!("CAPS (simplified): {:#?}", caps);
                    }
                  }
                  for audio_stream in info.audio_streams() {
                    if codec_string.len() > 0 {
                      codec_string = format!("{},", codec_string);
                    }
                    codec_string = format!("{}mp4a.40.2", codec_string);
                  }

                  let codec_string = format!("video/mp4; codecs=\"{}\"", codec_string);

                  let locked_state = shared_state.lock().unwrap();
                  locked_state
                    .window
                    .as_ref()
                    .unwrap()
                    .emit("new-clip-codec", (node_id, codec_string))
                    .unwrap();
                }
                let locked_state = shared_state.lock().unwrap();
                locked_state
                  .window
                  .as_ref()
                  .unwrap()
                  .emit("video-chunk-ready", (node_id, segment_number))
                  .unwrap();
              }
              Ok(Message::CompositedClipLength) => {
                let temp = networking::receive_data(&mut stream, 16).unwrap();
                let mut node_id_data = [0 as u8; 16];
                node_id_data.copy_from_slice(&temp);
                let node_id = Uuid::from_bytes(node_id_data);

                let bytes = networking::receive_data(&mut stream, 8).unwrap();
                let mut buffer = [0 as u8; 8];
                buffer.copy_from_slice(&bytes);
                let duration_ms = u64::from_le_bytes(buffer);

                let locked_state = shared_state.lock().unwrap();
                locked_state
                  .window
                  .as_ref()
                  .unwrap()
                  .emit("composited-clip-length", (node_id, duration_ms))
                  .unwrap();
              }
              _ => break,
            }
          }
          println!("All chunks done");

          let mut x = shared_state.lock().unwrap();
          x.window
            .as_ref()
            .unwrap()
            .emit(
              "generated-preview",
              VideoPreviewSend {
                output_directory_path: media_output_location(),
                segment_duration: CHUNK_LENGTH as i32,
              },
            )
            .unwrap();
          x.pipeline_executed = true;
        }
        // https://github.com/sdroege/gstreamer-rs/blob/master/examples/src/bin/events.rs
      }
    }
    thread::sleep(time::Duration::from_millis(1000));
  }
}
