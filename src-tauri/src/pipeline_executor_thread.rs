use core::time;
use std::{
  collections::HashMap,
  fs,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use crate::{
  classes::{global::uniq_id, pipeline::Pipeline},
  state_manager::SharedState,
};

pub fn pipeline_executor_thread(shared_state: Arc<Mutex<SharedState>>) {
  loop {
    let x = shared_state.lock().unwrap();
    let rx = &x.thread_stopper;
    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        break;
      }
      Err(mpsc::TryRecvError::Empty) => {
        let pipeline = x.store.pipeline.gen_graph_new(&x.store, &x.node_register);
        let clips = x.store.clips.clone();
        drop(x);
        if let Ok((node_type_data, composited_clip_data, output)) = pipeline {
          if let Some(output) = output {
            let mut output = output.trim().to_string();
            if output.len() > 0 {
              for (id, clip) in clips.source {
                println!("Clip info: {:#?}", clip.info);
                let name = uniq_id();
                let mut new_str = format!(
                  "filesrc location=\"{}\" ! qtdemux name={}",
                  clip.file_location.replace("\\", "/"),
                  name.clone()
                );
                if let Some(info) = clip.info {
                  for i in 0..info.video_streams.len() {
                    new_str = format!(
                      "{} {}.video_{} ! h264parse ! nvh264dec ! videoconvert name=source-clip-{}-video-{}",
                      new_str,
                      name.clone(),
                      i,
                      id.clone(),
                      i
                    );
                  }
                  for i in 0..info.audio_streams.len() {
                    new_str = format!(
                      "{} {}.audio_{} ! audioconvert name=source-clip-{}-audio-{}",
                      new_str,
                      name.clone(),
                      i,
                      id.clone(),
                      i
                    );
                  }
                  for i in 0..info.subtitle_streams.len() {
                    new_str = format!(
                      "{} {}.subtitles_{} ! subparse name=source-clip-{}-subtitles-{}",
                      new_str,
                      name.clone(),
                      i,
                      id.clone(),
                      i
                    );
                  }
                }
                output = format!("{} {}", new_str, output);
              }
              println!("Executing pipeline: {} ", output);
              Pipeline::execute_pipeline(output, 60);
              println!("Pipeline executed!");
            }
          }
        }
      }
    }
    thread::sleep(time::Duration::from_millis(20000));
  }
}
