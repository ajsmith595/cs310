use std::{
    fs::File,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use cs310_shared::{
    clip::CompositedClip, constants::CHUNK_LENGTH, networking, node::PipedType,
    nodes::NodeRegister, store::Store,
};
use ges::traits::{LayerExt, TimelineExt, UriClipAssetExt};
use glib::{ObjectExt, StaticType};
use gst::prelude::{ElementExt, ElementExtManual, GstBinExt, GstObjectExt};
use ipc_channel::ipc::{self, IpcOneShotServer, IpcReceiver, IpcSender};
use procspawn::JoinHandle;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum IPCMessage {
    Sender(IpcSender<IPCMessage>),
    GeneratePreview(CompositedClip, PipedType, u32, u32),
    CompositedClipLength(Uuid, u64),
    ChunkCompleted(Uuid, u32),
    ChunksCompleted(Uuid, u32, u32),
    OperationFinished,
    EndProcess,
}

type Process = (
    JoinHandle<()>,
    IpcSender<IPCMessage>,
    IpcReceiver<IPCMessage>,
);
pub struct ProcessPool {
    processes: Vec<Process>,
}

impl ProcessPool {
    pub fn new(number_of_processes: u32) -> Self {
        let mut processes = Vec::new();

        for _ in 0..number_of_processes {
            let (server, server_name) = IpcOneShotServer::new().unwrap();

            let handle = procspawn::spawn(server_name, process_func);
            let (_, child_send): (_, IpcSender<IPCMessage>) = server.accept().unwrap();

            let (parent_send, parent_recv): (IpcSender<IPCMessage>, IpcReceiver<IPCMessage>) =
                ipc::channel().unwrap();

            child_send.send(IPCMessage::Sender(parent_send)).unwrap();
            processes.push((handle, child_send, parent_recv));
        }

        Self { processes }
    }

    pub fn acquire_process(&mut self) -> Option<Process> {
        self.processes.pop()
    }

    pub fn add_process_to_pool(&mut self, process: Process) {
        self.processes.push(process);
    }
}

fn process_func(server_name: String) {
    gst::init().unwrap();
    ges::init().unwrap();

    let (child_send, child_recv): (IpcSender<IPCMessage>, IpcReceiver<IPCMessage>) =
        ipc::channel().unwrap();
    let server1_send = IpcSender::connect(server_name).unwrap();
    server1_send.send(child_send).unwrap();

    let message = child_recv.recv().unwrap();
    let parent_send = match message {
        IPCMessage::Sender(sender) => sender,
        _ => {
            panic!("Invalid message!")
        }
    };

    loop {
        match child_recv.recv() {
            Ok(msg) => match msg {
                IPCMessage::GeneratePreview(clip, output_type, start_chunk, end_chunk) => {
                    execute_pipeline(clip, output_type, start_chunk, end_chunk, &parent_send);
                }
                IPCMessage::EndProcess => {
                    break;
                }
                _ => {
                    println!("Invalid message!");
                }
            },
            Err(error) => {
                println!("Error encountered by receiving?");
            }
        }
        parent_send.send(IPCMessage::OperationFinished).unwrap();
    }

    // use parent_send to send messages to the parent process
    // use child_recv to receive messages from the parent process
}

fn execute_pipeline(
    clip: CompositedClip,
    output_type: PipedType,
    start_chunk: u32,
    end_chunk: u32,
    parent_send: &IpcSender<IPCMessage>,
) {
    let id = clip.id.clone();

    let pipeline = gst::Pipeline::new(None);

    let out_type = output_type;

    let timeline_location = clip.get_location();

    let timeline = out_type.stream_type.create_timeline();

    ges::Asset::needs_reload(
        ges::UriClip::static_type(),
        Some(timeline_location.as_str()),
    );
    let timeline_asset = ges::UriClipAsset::request_sync(timeline_location.as_str()).unwrap();

    let total_duration = timeline_asset.duration().unwrap().mseconds();
    let number_of_chunks =
        f64::ceil((total_duration as f64) / ((CHUNK_LENGTH as f64) * (1000 as f64)) as f64) as u32;

    let layer = timeline.append_layer();

    let inpoint = if start_chunk > 0 {
        let inpoint = ((CHUNK_LENGTH as u32) * start_chunk) as u64;
        Some(gst::ClockTime::from_seconds(inpoint))
    } else {
        None
    };

    let duration = if end_chunk == number_of_chunks - 1 && start_chunk == 0 {
        None
    } else if end_chunk < number_of_chunks - 1 {
        let num_chunks = end_chunk - start_chunk + 1;
        let duration = num_chunks * (CHUNK_LENGTH as u32);
        Some(gst::ClockTime::from_seconds(duration as u64))
    } else {
        let num_full_chunks = end_chunk - start_chunk;
        let duration = (num_full_chunks * (CHUNK_LENGTH as u32) * 1000) as u64
            + (total_duration % (CHUNK_LENGTH as u64));

        Some(gst::ClockTime::from_seconds(duration as u64))
    };

    layer
        .add_asset(
            &timeline_asset,
            None,
            inpoint,
            duration,
            ges::TrackType::UNKNOWN,
        )
        .unwrap();

    let total_duration = timeline.duration().mseconds();

    parent_send
        .send(IPCMessage::CompositedClipLength(
            clip.id.clone(),
            total_duration,
        ))
        .unwrap();

    pipeline.add(&timeline).unwrap();

    let muxer = gst::ElementFactory::make(
        "splitmuxsink",
        Some(format!("composited-clip-{}", id.to_string()).as_str()),
    )
    .unwrap();
    muxer.set_property("location", clip.get_output_location_template());
    muxer.set_property("muxer-factory", "mp4mux");
    muxer.set_property("start-index", start_chunk);

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

            let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
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
            let audioconvert1 = gst::ElementFactory::make("audioconvert", None).unwrap();
            let audioresample = gst::ElementFactory::make("audioresample", None).unwrap();
            let audioconvert2 = gst::ElementFactory::make("audioconvert", None).unwrap();

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

                        if let (Ok(location), Ok(running_time)) = (location, running_time) {
                            println!("Name: {}", src.name());
                            let node_id = src.name().to_string();

                            let mut parts: Vec<&str> = node_id.split("-").collect();
                            parts.drain(0..(parts.len() - 5));

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

                            parent_send
                                .send(IPCMessage::ChunkCompleted(id.clone(), segment))
                                .unwrap();
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
    parent_send
        .send(IPCMessage::ChunksCompleted(
            id.clone(),
            start_chunk,
            end_chunk,
        ))
        .unwrap();
}
