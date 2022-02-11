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
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use cs310_shared::{
    abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode},
    constants::{media_output_location, source_files_location, store_json_location, CHUNK_LENGTH},
    gst_editing_test,
    networking::{self, send_file, send_message, SERVER_PORT},
    node::PipeableStreamType,
    nodes::{get_node_register, NodeRegister},
    pipeline::Pipeline,
    store::Store,
};
use ges::{
    prelude::EncodingProfileBuilder,
    traits::{AssetExt, ExtractableExt, GESPipelineExt, LayerExt, ProjectExt, TimelineExt},
};
use glib::gobject_ffi::GValue;
use gst::{ffi::GstStructure, glib, prelude::*};
use simple_logger::SimpleLogger;
use state::State;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod state;

fn main() {
    SimpleLogger::new().init().unwrap();

    let current_dir = std::env::current_dir().unwrap();
    let current_dir = current_dir.to_str().unwrap();
    cs310_shared::constants::init(format!("{}/application_data", current_dir), true);

    let store = Store::from_file(store_json_location());

    let store = match store {
        Ok(store) => store,
        Err(_) => {
            let store = Store::new();

            let json = serde_json::to_string(&store).unwrap();
            std::fs::write(store_json_location(), json).unwrap();

            store
        }
    };

    let state = Arc::new(Mutex::new(State { store }));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();

    listener.set_nonblocking(true).unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting CTRL+C handler");

    log::info!("Server opened on port {}", SERVER_PORT);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(false).unwrap();
                log::info!("New connection from: {}", stream.peer_addr().unwrap());
                let state = state.clone();
                thread::spawn(move || {
                    handle_client(stream, state);
                });
            }
            Err(e) => {}
        }
        if !running.load(Ordering::SeqCst) {
            break;
        }
    }

    drop(listener);
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    log::info!("Handling client: {}", stream.peer_addr().unwrap());

    while match networking::receive_message(&mut stream) {
        Ok(message) => {
            let operation_id = &format!("{}", Uuid::new_v4())[..8];

            log::info!(
                "[{}] New operation from: {}",
                operation_id,
                stream.peer_addr().unwrap()
            );
            match message {
                networking::Message::GetStore => {
                    log::info!("[{}] Getting store ", operation_id);
                    let mut file = File::open(store_json_location()).unwrap();

                    networking::send_file(&mut stream, &mut file);
                    log::info!("[{}] Store sent ", operation_id);
                }
                networking::Message::GetFileID => {
                    log::info!("[{}] Getting unique file ID", operation_id);
                    let uuid = Uuid::new_v4();
                    networking::send_data(&mut stream, uuid.as_bytes()).unwrap();
                    log::info!("[{}] New file ID: {}", operation_id, uuid);
                }
                networking::Message::UploadFile => {
                    log::info!("[{}] Receiving file", operation_id);
                    let temp = networking::receive_data(&mut stream, 16).unwrap();
                    let mut uuid_bytes = [0 as u8; 16];
                    uuid_bytes.copy_from_slice(&temp);
                    let uuid = Uuid::from_bytes(uuid_bytes);

                    log::info!("[{}] File ID: {}", operation_id, uuid);

                    let mut output_file =
                        File::create(format!("{}/{}", source_files_location(), uuid)).unwrap();
                    networking::receive_file(&mut stream, &mut output_file);
                    let msg = networking::receive_message(&mut stream).unwrap();

                    log::info!("[{}] File received successfully", operation_id);
                }
                networking::Message::SetStore => {
                    log::info!("[{}] Receiving store", operation_id);
                    let mut output_file = File::create(store_json_location()).unwrap();
                    networking::receive_file(&mut stream, &mut output_file);

                    log::info!("[{}] Store received", operation_id);

                    let store = Store::from_file(store_json_location()).unwrap();
                    let node_register = get_node_register();
                    log::info!("[{}] Executing pipeline", operation_id);
                    execute_pipeline(&mut stream, &store, &node_register);
                    log::info!("[{}] Pipeline executed", operation_id);
                }
                _ => println!("Unknown message"),
            }

            true
        }
        Err(error) => {
            if error.kind() == ErrorKind::UnexpectedEof
                || error.kind() == ErrorKind::ConnectionReset
            {
                stream.shutdown(Shutdown::Both).unwrap();
                false
            } else {
                if error.kind() != ErrorKind::WouldBlock {
                    log::error!(
                        "Error occurred for connection {}: {:?}",
                        stream.peer_addr().unwrap(),
                        error
                    );
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

fn set_pipeline_props(pipeline: &ges::Pipeline, stream_type: &cs310_shared::node::PipeableType) {
    // Every audiostream piped into the encodebin should be encoded using opus.
    let audio_profile =
        gst_pbutils::EncodingAudioProfile::builder(&gst::Caps::builder("audio/x-vorbis").build())
            .build();

    // Every videostream piped into the encodebin should be encoded using vp8.
    let video_profile =
        gst_pbutils::EncodingVideoProfile::builder(&gst::Caps::builder("video/x-vp8").build())
            .build();

    // All streams are then finally combined into a webm container.

    let container_profile =
        gst_pbutils::EncodingContainerProfile::builder(&gst::Caps::builder("video/webm").build())
            .add_profile(&audio_profile)
            .add_profile(&video_profile)
            .build();

    // Apply the EncodingProfile to the pipeline, and set it to render mode
    let output_uri = format!("file:////home/ajsmith/cs310/server/application_data/out.webm");
    pipeline
        .set_render_settings(&output_uri, &container_profile)
        .expect("Failed to set render settings");
    pipeline
        .set_mode(ges::PipelineFlags::RENDER)
        .expect("Failed to set pipeline to render mode");
}

fn execute_pipeline(stream: &mut TcpStream, store: &Store, node_register: &NodeRegister) {
    let mut pipeline = store.pipeline.gen_graph_new(store, node_register, true);
    let clips = store.clips.clone();
    if let Ok((node_type_data, composited_clip_data, output)) = pipeline {
        if let Some(mut output) = output {
            if output.nodes.len() > 0 {
                let pipeline = gst::Pipeline::new(None);
                for (id, clip) in &clips.composited {
                    let out_type = composited_clip_data.get(id).unwrap();

                    let timeline_location = clip.get_location();

                    let timeline = out_type.stream_type.create_timeline();

                    let timeline_asset =
                        ges::UriClipAsset::request_sync(timeline_location.as_str()).unwrap();

                    let layer = timeline.append_layer();
                    layer
                        .add_asset(&timeline_asset, None, None, None, ges::TrackType::UNKNOWN)
                        .unwrap();

                    let total_duration = timeline.duration().mseconds();

                    networking::send_message(stream, networking::Message::CompositedClipLength)
                        .unwrap();

                    let node_id_bytes = id.as_bytes();
                    networking::send_data(stream, node_id_bytes).unwrap();
                    networking::send_data(stream, &total_duration.to_le_bytes());

                    pipeline.add(&timeline).unwrap();

                    let muxer = gst::ElementFactory::make(
                        "splitmuxsink",
                        Some(format!("composited-clip-{}", id.to_string()).as_str()),
                    )
                    .unwrap();
                    muxer.set_property("location", clip.get_output_location_template());
                    muxer.set_property("muxer-factory", "mp4mux");

                    std::fs::create_dir_all(clip.get_output_location()).unwrap();

                    let structure = gst::Structure::new(
                        "properties",
                        &[("streamable", &true), ("fragment-duration", &1000)],
                    );
                    muxer.set_property("muxer-properties", structure);
                    muxer.set_property("async-finalize", true);
                    let nanoseconds = (CHUNK_LENGTH as u64) * 1000000000;
                    muxer.set_property("max-size-time", nanoseconds);
                    muxer.set_property("send-keyframe-requests", true);
                    pipeline.add(&muxer).unwrap();

                    let mut i = 0;
                    for x in timeline.pads() {
                        let video = i;
                        let audio = i - out_type.stream_type.video;
                        i += 1;
                        println!("Name: {:?}", x.name());

                        if video < out_type.stream_type.video {
                            let encoder = gst::ElementFactory::make("x264enc", None).unwrap();

                            let videoconvert =
                                gst::ElementFactory::make("videoconvert", None).unwrap();
                            let queue = gst::ElementFactory::make("queue", None).unwrap();

                            pipeline.add(&encoder).unwrap();
                            pipeline.add(&videoconvert).unwrap();
                            pipeline.add(&queue).unwrap();

                            //pipeline.add(&videoconvert2).unwrap();
                            timeline
                                .link_pads(Some(x.name().as_str()), &videoconvert, None)
                                .unwrap();
                            videoconvert.link(&encoder).unwrap();
                            //encoder.link(&videoconvert2).unwrap();

                            encoder.link(&queue).unwrap();

                            if video > 0 {
                                panic!("Can only handle one video stream!");
                            }
                            queue
                                .link_pads(None, &muxer, Some(format!("video").as_str()))
                                .unwrap();
                        } else {
                            let audioconvert1 =
                                gst::ElementFactory::make("audioconvert", None).unwrap();
                            let audioresample =
                                gst::ElementFactory::make("audioresample", None).unwrap();
                            let audioconvert2 =
                                gst::ElementFactory::make("audioconvert", None).unwrap();

                            let queue = gst::ElementFactory::make("queue", None).unwrap();

                            let encoder = gst::ElementFactory::make("avenc_aac", None).unwrap();

                            pipeline.add(&audioconvert1).unwrap();
                            pipeline.add(&audioresample).unwrap();
                            pipeline.add(&audioconvert2).unwrap();
                            pipeline.add(&queue).unwrap();
                            pipeline.add(&encoder).unwrap();
                            timeline
                                .link_pads(Some(x.name().as_str()), &audioconvert1, None)
                                .unwrap();
                            audioconvert1.link(&audioresample).unwrap();
                            audioresample.link(&audioconvert2).unwrap();
                            audioconvert2.link(&encoder).unwrap();
                            encoder.link(&queue).unwrap();
                            queue
                                .link_pads(None, &muxer, Some(format!("audio_{}", audio).as_str()))
                                .unwrap();
                        }
                    }
                }

                println!("Starting pipeline...");
                pipeline
                    .set_state(gst::State::Playing)
                    .expect("Unable to set the pipeline to the `Playing` state");

                let bus = pipeline.bus().unwrap();

                for msg in bus.iter_timed(gst::ClockTime::NONE) {
                    use gst::MessageView;

                    match msg.view() {
                        MessageView::Eos(..) => break,
                        MessageView::Error(err) => {
                            println!(
                                "Error from {:?}: {} ({:?})",
                                err.src().map(|s| s.path_string()),
                                err.error(),
                                err.debug()
                            );
                            break;
                        }
                        MessageView::Element(_) => {
                            let src = msg.src();
                            let structure = msg.structure();

                            if let (Some(src), Some(structure)) = (src, structure) {
                                let event = structure.name().to_string();

                                if event == String::from("splitmuxsink-fragment-closed") {
                                    let location = structure.get::<String>("location");
                                    let running_time = structure.get::<u64>("running-time");

                                    if let (Ok(location), Ok(running_time)) =
                                        (location, running_time)
                                    {
                                        println!("Name: {}", src.name());
                                        let node_id = src.name().to_string();

                                        let mut parts: Vec<&str> = node_id.split("-").collect();
                                        parts.drain(0..(parts.len() - 5));
                                        let node_id = parts.join("-");

                                        let parts: Vec<&str> = location.split("/").collect();
                                        let filename = parts.last().unwrap();
                                        let parts: Vec<&str> = filename.split(".").collect();
                                        let number_string: String = parts
                                            .first() // the bit of the filename excluding the extension
                                            .unwrap()
                                            .chars()
                                            .filter(|c| c.is_digit(10)) // extract all the numbers
                                            .collect();
                                        let segment = number_string.parse::<u32>().unwrap();

                                        // let lock_clone = lock.clone();
                                        // let join = thread::spawn(move || {
                                        println!("New chunk: {} (segment: {})", node_id, segment);

                                        let mut parts: Vec<&str> = node_id.split("-").collect();
                                        parts.drain(0..(parts.len() - 5));
                                        let node_id = parts.join("-");
                                        let mut file = File::open(location).unwrap();

                                        networking::send_message(
                                            stream,
                                            networking::Message::NewChunk,
                                        )
                                        .unwrap();

                                        let uuid = Uuid::parse_str(&node_id).unwrap();
                                        let node_id_bytes = uuid.as_bytes();
                                        networking::send_data(stream, node_id_bytes).unwrap();
                                        let mut segment_bytes = [0 as u8; 4];
                                        segment_bytes.copy_from_slice(&segment.to_le_bytes());
                                        networking::send_data(stream, &segment_bytes).unwrap();
                                        networking::send_file(stream, &mut file);
                                    }
                                }
                            }
                        }

                        _ => (),
                    }
                }

                pipeline
                    .set_state(gst::State::Null)
                    .expect("Unable to set the pipeline to the `Null` state");
                println!("Pipeline complete!");

                println!("Composited clips done!");
            }
        }
    }
    networking::send_message(stream, networking::Message::AllChunksGenerated).unwrap();
}
