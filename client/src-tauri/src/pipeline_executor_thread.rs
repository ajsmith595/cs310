use core::time;
use std::{
  collections::HashMap,
  fs,
  path::Path,
  process::Command,
  sync::{mpsc, Arc, Mutex},
  thread,
};

use crate::state_manager::SharedState;
use cs310_shared::{
  abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode},
  constants::{media_output_location, CHUNK_LENGTH},
  global::uniq_id,
  node::PipeableStreamType,
  pipeline::Pipeline,
};

#[derive(Serialize)]
struct VideoPreviewSend {
  pub output_directory_path: String,
  pub segment_duration: i32,
}

pub fn pipeline_executor_thread(shared_state: Arc<Mutex<SharedState>>) {
  let mut path = None;
  match dirs::data_dir() {
    Some(p) => {
      path = Some(p.join(media_output_location()));
    }
    None => println!("Cannot get data directory!"),
  }
  let path = path.unwrap();

  loop {
    let x = shared_state.lock().unwrap();
    let rx = &x.thread_stopper;

    match rx.try_recv() {
      Ok(_) | Err(mpsc::TryRecvError::Disconnected) => {
        println!("Thread terminating");
        //break;
        return;
      }
      Err(mpsc::TryRecvError::Empty) => {
        if !x.pipeline_executed {
          let mut pipeline = x.store.pipeline.gen_graph_new(&x.store, &x.node_register);
          let clips = x.store.clips.clone();
          drop(x);
          if let Ok((node_type_data, composited_clip_data, output)) = pipeline {
            if let Some(mut output) = output {
              if output.nodes.len() > 0 {
                for (id, clip) in clips.source {
                  let mut props = HashMap::new();
                  props.insert(
                    "location".to_string(),
                    format!("\"{}\"", clip.file_location.replace("\\", "/")),
                  );
                  let filesrc_node = AbstractNode::new_with_props("filesrc", None, props);

                  let qtdemux_node = AbstractNode::new("qtdemux", None);

                  output.link(&filesrc_node, &qtdemux_node);

                  if let Some(info) = clip.info {
                    for i in 0..info.video_streams.len() {
                      let decoder_node = AbstractNode::new_decoder(&PipeableStreamType::Video);
                      let videoconvert_node = AbstractNode::new(
                        "videoconvert",
                        Some(format!("source-clip-{}-video-{}", id.clone(), i)),
                      );

                      output.link_abstract(AbstractLink {
                        from: AbstractLinkEndpoint::new_with_property(
                          qtdemux_node.id.clone(),
                          format!("video_{}", i),
                        ),
                        to: AbstractLinkEndpoint::new(decoder_node.id.clone()),
                      });
                      output.link(&decoder_node, &videoconvert_node);

                      output.add_node(decoder_node);
                      output.add_node(videoconvert_node);
                    }
                    for i in 0..info.audio_streams.len() {
                      let decoder_node = AbstractNode::new_decoder(&PipeableStreamType::Audio);
                      let audioconvert_node = AbstractNode::new(
                        "audioconvert",
                        Some(format!("source-clip-{}-audio-{}", id.clone(), i)),
                      );

                      output.link_abstract(AbstractLink {
                        from: AbstractLinkEndpoint::new_with_property(
                          qtdemux_node.id.clone(),
                          format!("audio_{}", i),
                        ),
                        to: AbstractLinkEndpoint::new(decoder_node.id.clone()),
                      });
                      output.link(&decoder_node, &audioconvert_node);

                      output.add_node(decoder_node);
                      output.add_node(audioconvert_node);
                    }
                    for i in 0..info.subtitle_streams.len() {
                      let decoder_node = AbstractNode::new_decoder(&PipeableStreamType::Subtitles);
                      let subparse_node = AbstractNode::new(
                        "subparse",
                        Some(format!("source-clip-{}-subtitles-{}", id.clone(), i)),
                      );

                      output.link_abstract(AbstractLink {
                        from: AbstractLinkEndpoint::new_with_property(
                          qtdemux_node.id.clone(),
                          format!("subtitles_{}", i),
                        ),
                        to: AbstractLinkEndpoint::new(decoder_node.id.clone()),
                      });
                      output.link(&decoder_node, &subparse_node);

                      output.add_node(decoder_node);
                      output.add_node(subparse_node);
                    }
                  }

                  output.add_node(filesrc_node);
                  output.add_node(qtdemux_node);
                }

                for (id, clip) in &clips.composited {
                  let directory = clip.get_output_location();
                  if !Path::new(&directory).exists() {
                    fs::create_dir_all(directory).unwrap();
                  }
                }

                let output = output.to_gstreamer_pipeline();
                println!("Executing pipeline: {} ", output);
                let shared_state_clone = shared_state.clone();
                Pipeline::execute_pipeline(
                  output,
                  180,
                  Some(Box::new(move |node_id, segment, location| {
                    let shared_state_clone = shared_state_clone.clone();
                    let shared_state_clone = shared_state_clone.lock().unwrap();
                    let window = shared_state_clone.window.as_ref().unwrap();
                    window
                      .emit("video-chunk-ready", (node_id, segment))
                      .unwrap();
                  })),
                )
                .unwrap();
                println!("Pipeline executed!");

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
                drop(x);
              }
            }
          }
        } else {
          drop(x);
        }
      }
    }
    thread::sleep(time::Duration::from_millis(1000));
  }
}
