use core::time;
use std::{
    cell::RefCell,
    collections::HashMap,
    convert::Infallible,
    fs::{self, File},
    io::{ErrorKind, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::Path,
    rc::Rc,
    thread,
};

use cs310_shared::{
    abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode},
    constants::{media_output_location, store_json_location},
    networking::{self, send_file, send_message, SERVER_PORT},
    node::PipeableStreamType,
    nodes::{get_node_register, NodeRegister},
    pipeline::Pipeline,
    store::Store,
};
use gstreamer::{glib, prelude::*};
use state::State;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod state;
const OUTPUT_DIR: &str = "output";

fn main() {
    let current_dir = std::env::current_dir().unwrap();
    let current_dir = current_dir.to_str().unwrap();
    cs310_shared::constants::init(format!("{}/application_data", current_dir));
    gstreamer::init().expect("GStreamer could not be initialised");

    let store = Store::from_file(String::from("state.json"));

    let store = match store {
        Ok(store) => store,
        Err(_) => Store::new(String::from(OUTPUT_DIR)),
    };

    let state = Arc::new(Mutex::new(State { store }));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                let state = state.clone();
                thread::spawn(move || {
                    handle_client(stream, state);
                });
            }
            Err(e) => {}
        }
    }

    drop(listener);
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    while match networking::receive_message(&mut stream) {
        Ok(message) => {
            println!("Valid message received: {:?}", message);

            match message {
                networking::Message::GetStore => {
                    let store = Store::new(String::from(""));

                    let store_json = serde_json::to_string(&store).unwrap();
                    let bytes = store_json.as_bytes();

                    let length = (bytes.len() as u64).to_ne_bytes();
                    networking::send_message_with_data(
                        &mut stream,
                        networking::Message::Response,
                        &length,
                    )
                    .unwrap();
                    // first send the length of the data itself

                    networking::send_data(&mut stream, bytes).unwrap();
                    // then send the data
                }
                networking::Message::UploadFile => {
                    println!("Receiving file...");
                    let mut output_file = File::create("output-test-file.txt").unwrap();
                    networking::receive_file(&mut stream, &mut output_file);
                    let msg = networking::receive_message(&mut stream).unwrap();

                    println!("Received file! End message: {:?}", msg);
                }
                networking::Message::SetStore => {
                    println!("Receiving store...");
                    let mut output_file = File::create(store_json_location()).unwrap();
                    networking::receive_file(&mut stream, &mut output_file);

                    println!("store received. playing pipeline...");

                    let store = Store::from_file(store_json_location()).unwrap();
                    let node_register = get_node_register();
                    execute_pipeline(&mut stream, &store, &node_register);
                }
                _ => println!("Unknown message"),
            }

            true
        }
        Err(error) => {
            if error.kind() == ErrorKind::UnexpectedEof {
                stream.shutdown(Shutdown::Both).unwrap();
                false
            } else {
                if error.kind() != ErrorKind::WouldBlock {
                    println!("Error type: {:?}", error.kind());
                    println!("Error description: {}", error.to_string());
                    // println!(
                    //     "Error encountered whilst reading from client: {}; shutting down stream",
                    //     error
                    // );
                    // stream.shutdown(Shutdown::Both).unwrap();
                }
                true
            }
        }
    } {
        thread::sleep(time::Duration::from_millis(10));
    }
}

fn execute_pipeline(stream: &mut TcpStream, store: &Store, node_register: &NodeRegister) {
    let mut pipeline = store.pipeline.gen_graph_new(store, node_register);
    let clips = store.clips.clone();
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
                            let decoder_node =
                                AbstractNode::new_decoder(&PipeableStreamType::Video);
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
                            let decoder_node =
                                AbstractNode::new_decoder(&PipeableStreamType::Audio);
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
                            let decoder_node =
                                AbstractNode::new_decoder(&PipeableStreamType::Subtitles);
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
                    let directory = clip.get_output_location_ext(false);
                    if !Path::new(&directory).exists() {
                        fs::create_dir_all(directory).unwrap();
                    }
                }

                let output = output.to_gstreamer_pipeline();
                //println!("Executing pipeline: {} ", output);
                println!("Media output: {}", media_output_location());
                // let lock = Rc::new(RefCell::new(stream));
                let lock = Arc::new(Mutex::new(stream));
                let lock_clone = lock.clone();
                // let shared_state_clone = shared_state.clone();
                Pipeline::execute_pipeline(
                    output,
                    180,
                    Some(Box::new(move |node_id, segment, location| {
                        println!("New chunk: {} (segment: {})", node_id, segment);

                        let mut parts: Vec<&str> = node_id.split("-").collect();
                        parts.drain(0..(parts.len() - 5));
                        let node_id = parts.join("-");

                        // example:
                        /*
                            composited-clip-file-acee7713-1ea9-46d4-af05-92a029a1aa78
                            =>
                            acee7713-1ea9-46d4-af05-92a029a1aa78
                        */
                        let mut stream = lock_clone.lock().unwrap();
                        let mut file = File::open(location).unwrap();

                        networking::send_message(&mut stream, networking::Message::NewChunk)
                            .unwrap();

                        let uuid = Uuid::parse_str(&node_id).unwrap();
                        let node_id_bytes = uuid.as_bytes();
                        networking::send_data(&mut stream, node_id_bytes).unwrap();
                        // let node_id_bytes = node_id.as_bytes();
                        // networking::send_data(&mut stream, node_id_bytes).unwrap();
                        let mut segment_bytes = [0 as u8; 4];
                        segment_bytes.copy_from_slice(&segment.to_le_bytes());
                        networking::send_data(&mut stream, &segment_bytes).unwrap();
                        networking::send_file(&mut stream, &mut file);

                        // let shared_state_clone = shared_state_clone.clone();
                        // let shared_state_clone = shared_state_clone.lock().unwrap();
                        // let window = shared_state_clone.window.as_ref().unwrap();
                        // window
                        //     .emit("video-chunk-ready", (node_id, segment))
                        //     .unwrap();
                    })),
                )
                .unwrap();
                let mut stream = lock.lock().unwrap();
                networking::send_message(&mut stream, networking::Message::AllChunksGenerated)
                    .unwrap();
                println!("Pipeline executed!");

                const SEGMENT_DURATION: i32 = 10;
                // let mut x = shared_state.lock().unwrap();
                // x.window
                //     .as_ref()
                //     .unwrap()
                //     .emit(
                //         "generated-preview",
                //         VideoPreviewSend {
                //             output_directory_path: APPLICATION_MEDIA_OUTPUT(),
                //             segment_duration: SEGMENT_DURATION,
                //         },
                //     )
                //     .unwrap();
                // x.pipeline_executed = true;
                // drop(x);
            }
        }
    }
}
