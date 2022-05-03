use std::{
  collections::HashMap,
  fs::File,
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{constants::CHUNK_FILENAME_NUMBER_LENGTH, networking};
use ges::prelude::DiscovererStreamInfoExt;
use glib::Cast;
use gst_pbutils::DiscovererVideoInfo;
use uuid::Uuid;

use crate::state::{SharedState, VideoPreviewChunkStatus, VideoPreviewStatus};

/**
 * Handles the requesting of video chunks from the server
 */
pub fn video_preview_handler_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut lock = shared_state.lock().unwrap();

    // first, look at what has been requested by the front end, and set it as 'generating'
    let video_preview_status = &mut lock.video_preview_data;
    for (id, status) in video_preview_status.iter_mut() {
      match status {
        VideoPreviewStatus::Data(_, _, _, data) => {
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
          video_preview_length_requested(id.clone(), shared_state.clone());
        }
        VideoPreviewStatus::Data(length, _, _, data) => {
          video_preview_data(id.clone(), data, shared_state.clone());
        }
      }
    }

    thread::sleep(Duration::from_secs(1));
  }
}

/// Goes through the video preview, and downloads any generated but not downloaded chunks from the server
pub fn video_previewer_downloader_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let mut lock = shared_state.lock().unwrap();

    // first, look at what has been requested by the front end, and set it as 'generating'

    let video_preview_status = &mut lock.video_preview_data;

    let mut clip_ids = Vec::new();
    for (id, status) in video_preview_status.iter_mut() {
      match status {
        VideoPreviewStatus::Data(_, _, _, data) => {
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
        VideoPreviewStatus::Data(_, codec, _, data) => {
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
                  "{}/segment{:0>width$}.ts",
                  clip.get_output_location(),
                  chunk_id,
                  width = CHUNK_FILENAME_NUMBER_LENGTH as usize
                );

                std::fs::create_dir_all(clip.get_output_location()).unwrap();

                let msg = networking::receive_message(&mut stream).unwrap();
                match msg {
                  networking::Message::Response => {
                    let mut file = File::create(output_location.clone()).unwrap();
                    networking::receive_file(&mut stream, &mut file);

                    let mut lock = shared_state.lock().unwrap();
                    let existing_data = lock.video_preview_data.get_mut(&id).unwrap();

                    match existing_data {
                      VideoPreviewStatus::Data(duration, codec, is_vid, data) => {
                        data[i] = VideoPreviewChunkStatus::Downloaded;
                        println!("File {} received", output_location.clone());
                        if codec.is_none() {
                          if let Ok((codec_string, is_video)) =
                            get_codec_string(output_location.clone())
                          // obtains the codec by inspecting the file
                          {
                            *codec = Some(codec_string.clone());
                            *is_vid = is_video;
                          }
                        }
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
                  _ => {
                    println!("Chunk could not be downloaded!");
                    return;
                  }
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

pub fn video_preview_length_requested(
  id: Uuid,
  shared_state: Arc<Mutex<SharedState>>,
) -> Result<(), std::io::Error> {
  let mut stream = networking::connect_to_server()?;

  networking::send_message(&mut stream, networking::Message::CompositedClipLength)?;
  let uuid_bytes = id.as_bytes();
  networking::send_data(&mut stream, uuid_bytes)?;

  let msg = networking::receive_message(&mut stream)?;

  match msg {
    networking::Message::CompositedClipLength => {
      let uuid = networking::receive_uuid(&mut stream)?;
      let duration = networking::receive_u64(&mut stream)?;
      let number_of_chunks = networking::receive_u32(&mut stream)?;

      let data_statuses = vec![VideoPreviewChunkStatus::NotRequested; number_of_chunks as usize];
      let status = VideoPreviewStatus::Data(duration, None, false, data_statuses);

      let mut lock = shared_state.lock().unwrap();
      lock.video_preview_data.insert(id.clone(), status);

      lock
        .window
        .as_ref()
        .unwrap()
        .emit("video-preview-data-update", lock.video_preview_data.clone())
        .unwrap();
    }
    networking::Message::CouldNotGetLength => {}
    _ => {
      panic!("Unknown message!: {:?}", msg);
    }
  }

  Ok(())
}

pub fn video_preview_data(
  id: Uuid,
  data: Vec<VideoPreviewChunkStatus>,
  shared_state: Arc<Mutex<SharedState>>,
) -> Result<(), std::io::Error> {
  let mut start_chunk = None;
  let mut end_chunk = None;
  for i in 0..data.len() {
    let status = &data[i];
    match status {
      VideoPreviewChunkStatus::Generating => {
        if start_chunk.is_none() {
          start_chunk = Some(i);
        }
        end_chunk = Some(i);
      }
      _ => {
        if start_chunk.is_some() {
          break;
        }
      }
    }
  }

  if let (Some(start_chunk), Some(end_chunk)) = (start_chunk, end_chunk) {
    let mut stream = networking::connect_to_server()?;

    println!(
      "Getting video preview for {} between chunks {} and {}",
      id, start_chunk, end_chunk
    );
    networking::send_message(&mut stream, networking::Message::GetVideoPreview)?;
    networking::send_data(&mut stream, id.as_bytes())?;
    networking::send_data(&mut stream, &start_chunk.to_ne_bytes())?;
    networking::send_data(&mut stream, &end_chunk.to_ne_bytes())?;

    loop {
      let message = networking::receive_message(&mut stream)?;
      match message {
        networking::Message::CouldNotGeneratePreview => {
          return Err(std::io::Error::from_raw_os_error(22));
        }
        networking::Message::NewChunk => {
          let chunk_id = networking::receive_u32(&mut stream)?;
          println!("Got new chunk: {}", chunk_id);

          let mut lock = shared_state.lock().unwrap();
          let entry = lock.video_preview_data.get_mut(&id).unwrap();

          match entry {
            VideoPreviewStatus::Data(duration, _, _, data) => {
              if data.get(chunk_id as usize).is_none() {
                println!("Received chunk out of range");
              }
              data[chunk_id as usize] = VideoPreviewChunkStatus::Generated;
              // notify to the downloader thread that the chunk is ready to be downloaded

              lock
                .window
                .as_ref()
                .unwrap()
                .emit("video-preview-data-update", lock.video_preview_data.clone())
                .unwrap();
            }
            _ => {
              println!("Received chunk for a section which no longer has data");
            }
          }
        }
        networking::Message::AllChunksGenerated => {
          println!("All chunks generated");
          break;
        }
        _ => {}
      }
    }
  }

  Ok(())
}

/// Looks at the supplied file, and will return the codec string that should be supplied to MSE when creating the source buffer.
fn get_codec_string(filename: String) -> Result<(String, bool), glib::Error> {
  let discoverer = gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(10)).unwrap();

  let info = discoverer.discover_uri(format!("file:///{}", filename).as_str())?;

  let video_streams = info.video_streams();

  let mut codec_string = String::from("");
  for video_stream in video_streams.clone() {
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

    if codec_string.len() > 0 {
      codec_string = format!("{},", codec_string);
    }
    codec_string = format!("{}avc1.{}", codec_string, string);
  }
  for audio_stream in info.audio_streams() {
    if codec_string.len() > 0 {
      codec_string = format!("{},", codec_string);
    }
    codec_string = format!("{}mp4a.40.2", codec_string);
  }

  Ok((
    format!("video/mp2t; codecs=\"{}\"", codec_string),
    video_streams.len() > 0,
  ))
}
