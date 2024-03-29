use core::time;
use cs310_shared::{
    cache::Cache,
    clip::{ClipType, CompositedClip, SourceClip, SourceClipServerStatus},
    constants::{
        source_files_location, store_json_location, CHUNK_FILENAME_NUMBER_LENGTH, CHUNK_LENGTH,
    },
    networking::{self, SERVER_PORT},
    node::{Node, NodeTypeInput, NodeTypeOutput, PipedType},
    nodes::{get_node_register, NodeRegister},
    pipeline::Link,
    store::Store,
    task::Task,
};
use ges::traits::{GESPipelineExt, LayerExt, TimelineExt};
use gst::prelude::*;
use ipc_channel::ipc::{self, IpcOneShotServer, IpcSender};
use num_traits::cast::FromPrimitive;
use serde_json::Value;
use simple_logger::SimpleLogger;
use state::State;
use std::{
    borrow::BorrowMut,
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::ErrorKind,
    net::{Shutdown, TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    gst_process::{IPCMessage, ProcessPool},
    state::VideoChunkStatus,
};

use threadpool::ThreadPool;

mod gst_process;
mod state;
fn main() {
    SimpleLogger::new().init().unwrap();
    procspawn::init();
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

    let state = Arc::new(Mutex::new(State {
        store,
        gstreamer_processes: ProcessPool::new(8),
        video_preview_generation: HashMap::new(),
        cache: Cache::new(),
    }));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();
    listener.set_nonblocking(true).unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting CTRL+C handler");

    let state_clone = state.clone();
    let running_clone = running.clone();

    // Saves the persistent state to the filesystem periodically
    let store_saver_thread = thread::spawn(move || {
        let mut previous_hash = None;
        let mut last_executed = Instant::now();
        loop {
            let now = Instant::now();
            if (now - last_executed) > Duration::from_secs(10) {
                let lock = state_clone.lock().unwrap();
                let store = lock.store.clone();
                drop(lock);

                let bytes = serde_json::to_vec(&store).unwrap();
                let mut hash = DefaultHasher::new();
                bytes.hash(&mut hash);
                let hash = hash.finish();
                if match previous_hash {
                    None => true,
                    Some(h) => h != hash,
                } {
                    fs::write(store_json_location(), &bytes).unwrap();
                }
                previous_hash = Some(hash);

                last_executed = now;
            }

            thread::sleep(Duration::from_secs(1));

            if !running_clone.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    log::info!("Server opened on port {}", SERVER_PORT);

    let pool = ThreadPool::new(10);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(false).unwrap();
                log::info!("New connection from: {}", stream.peer_addr().unwrap());
                let state = state.clone();
                pool.execute(move || {
                    handle_client(stream, state);
                });
            }
            Err(_) => {}
        }
        if !running.load(Ordering::SeqCst) {
            break;
        }
    }

    println!("Waiting for store saver to stop...");
    store_saver_thread.join().unwrap();
    println!("Store saver stopped");
    state.lock().unwrap().gstreamer_processes.clear();

    drop(listener);
}

/**
 * Handles a particular client request
 */
fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) -> Result<(), std::io::Error> {
    log::info!("Handling client: {}", stream.peer_addr().unwrap());

    while match networking::receive_message(&mut stream) {
        Ok(message) => {
            let operation_id = &format!("{}", Uuid::new_v4())[..8];

            log::info!(
                "[{}] New operation from: {}",
                operation_id,
                stream.peer_addr().unwrap()
            );

            // The client always sends an initial message, which is the intent of the connection
            match message {
                networking::Message::GetStore => {
                    log::info!("[{}] Getting store", operation_id);
                    let store = state.lock().unwrap().store.get_client_data();
                    let data = serde_json::to_vec(&store).unwrap();
                    networking::send_as_file(&mut stream, &data);
                    log::info!("[{}] Store sent as file", operation_id);
                }
                networking::Message::GetFileID => {
                    log::info!("[{}] Getting unique file ID", operation_id);
                    let uuid = Uuid::new_v4();
                    networking::send_data(&mut stream, uuid.as_bytes()).unwrap();
                    log::info!("[{}] New file ID: {}", operation_id, uuid);
                }
                networking::Message::UploadFile => {
                    client_message_upload_file(&mut stream, state.clone(), operation_id)?;
                }
                networking::Message::CreateSourceClip => {
                    let clip_data = networking::receive_file_as_bytes(&mut stream);
                    let clip = serde_json::from_slice::<SourceClip>(&clip_data);
                    if clip.is_err() {
                        log::warn!("[{}] Client sent invalid JSON data!", operation_id);

                        return Ok(());
                    }
                    let mut clip = clip.unwrap();
                    let uuid = Uuid::new_v4();
                    clip.id = uuid.clone();
                    clip.status = SourceClipServerStatus::LocalOnly;
                    networking::send_data(&mut stream, uuid.as_bytes()).unwrap();
                    let mut lock = state.lock().unwrap();
                    lock.store.clips.source.insert(uuid, clip);
                }
                networking::Message::CreateCompositedClip => {
                    let clip_data = networking::receive_file_as_bytes(&mut stream);
                    let clip = serde_json::from_slice::<CompositedClip>(&clip_data);
                    if clip.is_err() {
                        log::warn!("[{}] Client sent invalid JSON data!", operation_id);

                        return Ok(());
                    }
                    let mut clip = clip.unwrap();
                    let uuid = Uuid::new_v4();
                    clip.id = uuid.clone();
                    networking::send_data(&mut stream, uuid.as_bytes()).unwrap();

                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::CreateCompositedClip(clip)]);
                }
                networking::Message::Checksum => {
                    let checksum = networking::receive_u64(&mut stream)?;

                    let store_checksum = state.lock().unwrap().store.get_client_checksum();
                    if store_checksum != checksum {
                        log::warn!("Checksum not the same! Updating client");
                        // update client

                        networking::send_message(&mut stream, networking::Message::ChecksumError)
                            .unwrap();

                        let data = &state.lock().unwrap().store.get_client_data();
                        let bytes = serde_json::to_vec(data).unwrap();

                        println!("Sending new JSON - byte length: {}", bytes.len());
                        networking::send_as_file(&mut stream, &bytes);
                    } else {
                        networking::send_message(&mut stream, networking::Message::ChecksumOk)
                            .unwrap();
                    }
                }
                networking::Message::CreateNode => {
                    let bytes = networking::receive_file_as_bytes(&mut stream);
                    let node = serde_json::from_slice::<Node>(&bytes);
                    if node.is_err() {
                        log::warn!("Client sent invalid JSON!");
                        return Ok(());
                    }
                    let mut node = node.unwrap();
                    let uuid = Uuid::new_v4();
                    node.id = uuid.clone();
                    networking::send_data(&mut stream, uuid.as_bytes()).unwrap();

                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::AddNode(node)]);
                }
                networking::Message::UpdateNode => {
                    let bytes = networking::receive_file_as_bytes(&mut stream);
                    let node = serde_json::from_slice::<Node>(&bytes);
                    if node.is_err() {
                        log::warn!("Client sent invalid JSON!");
                        return Ok(());
                    }
                    let node = node.unwrap();
                    let mut lock = state.lock().unwrap();
                    if lock.store.nodes.contains_key(&node.id) {
                        lock.cache_node_modified(&node.id);
                        let store = lock.store.borrow_mut();
                        Task::apply_tasks(store, vec![Task::UpdateNode(node.id.clone(), node)]);
                    }
                }
                networking::Message::AddLink => {
                    let bytes = networking::receive_file_as_bytes(&mut stream);
                    let link = serde_json::from_slice::<Link>(&bytes);
                    if link.is_err() {
                        log::warn!("Client sent invalid JSON!");
                        return Ok(());
                    }
                    let link = link.unwrap();
                    let mut lock = state.lock().unwrap();

                    if lock.store.nodes.contains_key(&link.to.node_id) {
                        lock.cache_node_modified(&link.to.node_id);
                        let store = lock.store.borrow_mut();
                        Task::apply_tasks(store, vec![Task::AddLink(link)]);
                    }
                }
                networking::Message::DeleteLinks => {
                    let uuid = networking::receive_uuid(&mut stream)?;

                    let property = networking::receive_file_as_bytes(&mut stream);
                    let property = String::from_utf8(property);
                    if property.is_err() {
                        log::warn!("Client sent invalid string!");
                    }
                    let property = property.unwrap();
                    let property = match property.as_str() {
                        "" => None,
                        s => Some(String::from(s)),
                    };
                    let mut lock = state.lock().unwrap();

                    if lock.store.nodes.contains_key(&uuid) {
                        lock.cache_node_modified(&uuid);
                        let store = lock.store.borrow_mut();
                        Task::apply_tasks(store, vec![Task::DeleteLinks(uuid, property)]);
                    }
                }
                networking::Message::UpdateClip => {
                    let clip_type_bytes = networking::receive_data(&mut stream, 1)?;
                    let clip_type = ClipType::from_u8(clip_type_bytes[0]);
                    if clip_type.is_none() {
                        log::warn!("Client sent invalid clip type!");
                        return Ok(());
                    }
                    let clip_type = clip_type.unwrap();

                    let bytes = networking::receive_file_as_bytes(&mut stream);

                    let (id, clip) = match clip_type {
                        ClipType::Source => {
                            let clip = serde_json::from_slice::<SourceClip>(&bytes);
                            if clip.is_err() {
                                log::warn!(
                                    "Client sent invalid JSON! (source) - {:?}",
                                    clip.unwrap_err()
                                );
                                let value = serde_json::from_slice::<Value>(&bytes);
                                println!("Value: {:?}", value);
                                return Ok(());
                            }
                            let clip = clip.unwrap();
                            let id = clip.id.clone();
                            (id, serde_json::to_value(clip).unwrap())
                        }
                        ClipType::Composited => {
                            let clip = serde_json::from_slice::<CompositedClip>(&bytes);
                            if clip.is_err() {
                                log::warn!("Client sent invalid JSON!");
                                return Ok(());
                            }
                            let clip = clip.unwrap();
                            let id = clip.id.clone();
                            (id, serde_json::to_value(clip).unwrap())
                        }
                    };
                    let mut lock = state.lock().unwrap();

                    lock.cache_clip_modified(&id, clip_type.clone());
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::UpdateClip(id, clip_type, clip)]);
                }
                networking::Message::DeleteNode => {
                    let uuid = networking::receive_uuid(&mut stream)?;

                    let mut lock = state.lock().unwrap();

                    if lock.store.nodes.contains_key(&uuid) {
                        lock.cache_node_modified(&uuid);
                        let store = lock.store.borrow_mut();
                        Task::apply_tasks(store, vec![Task::DeleteNode(uuid)]);
                    }
                }
                networking::Message::CompositedClipLength => {
                    client_message_composited_clip_length(
                        &mut stream,
                        state.clone(),
                        operation_id,
                    )?;
                }
                networking::Message::GetVideoPreview => {
                    client_message_get_video_preview(&mut stream, state.clone(), operation_id)?;
                }

                networking::Message::DownloadChunk => {
                    client_message_download_chunk(&mut stream, state.clone(), operation_id)?;
                }
                _ => {
                    log::error!(
                        "[{}] Unknown message received; terminating connection",
                        operation_id
                    );
                    stream.shutdown(Shutdown::Both).unwrap();
                    return Ok(());
                }
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
                }
                true
            }
        }
    } {
        thread::sleep(time::Duration::from_millis(10));
    }

    Ok(())
}

fn client_message_download_chunk(
    stream: &mut TcpStream,
    state: Arc<Mutex<State>>,
    operation_id: &str,
) -> Result<(), std::io::Error> {
    let uuid = networking::receive_uuid(stream)?;
    let chunk_id = networking::receive_u32(stream)?;

    let mut lock = state.lock().unwrap();
    let clip = lock.store.clips.composited.get(&uuid);
    let data = lock.video_preview_generation.get(&uuid);
    if let (Some((duration, codec, data)), Some(clip)) = (data, clip) {
        if let Some(data) = data.get(chunk_id as usize) {
            match data {
                VideoChunkStatus::Generated => {
                    let output_location = format!(
                        "{}/segment{:0>width$}.ts",
                        clip.get_output_location(),
                        chunk_id,
                        width = CHUNK_FILENAME_NUMBER_LENGTH as usize
                    );

                    let file = File::open(output_location);
                    if let Ok(mut file) = file {
                        networking::send_message(stream, networking::Message::Response).unwrap();
                        networking::send_file(stream, &mut file);
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    networking::send_message(stream, networking::Message::ChecksumError).unwrap(); // placeholder

    Ok(())
}

fn client_message_upload_file(
    stream: &mut TcpStream,
    state: Arc<Mutex<State>>,
    operation_id: &str,
) -> Result<(), std::io::Error> {
    log::info!("[{}] Receiving file", operation_id);
    let uuid = networking::receive_uuid(stream)?;

    log::info!("[{}] File ID: {}", operation_id, uuid);

    let mut locked_state = state.lock().unwrap();

    let store = &mut locked_state.store;
    let clip = store.clips.source.get_mut(&uuid);

    if clip.is_none() {
        log::warn!(
            "[{}] Client tried to upload file for a source clip which did not exist",
            operation_id
        );
        stream.shutdown(Shutdown::Both).unwrap();
        return Ok(());
    }
    let clip = clip.unwrap();

    match &clip.status {
        SourceClipServerStatus::LocalOnly => {}
        _ => {
            log::warn!("[{}] Client tried to upload file for a source clip which is not marked as LocalOnly", operation_id);
            stream.shutdown(Shutdown::Both).unwrap();

            return Ok(());
        }
    }
    clip.status = SourceClipServerStatus::Uploading;
    drop(locked_state);

    let mut output_file = File::create(format!("{}/{}", source_files_location(), uuid)).unwrap();
    networking::receive_file(stream, &mut output_file);
    let msg = networking::receive_message(stream).unwrap();
    let mut locked_state = state.lock().unwrap();
    let store = &mut locked_state.store;
    let clip = store.clips.source.get_mut(&uuid).unwrap();
    clip.status = SourceClipServerStatus::Uploaded;

    log::info!("[{}] File received successfully", operation_id);

    Ok(())
}

fn client_message_composited_clip_length(
    stream: &mut TcpStream,
    state: Arc<Mutex<State>>,
    operation_id: &str,
) -> Result<(), std::io::Error> {
    println!("Clip length");
    let composited_clip_id = networking::receive_uuid(stream)?;

    let mut lock = state.lock().unwrap();
    let result = generate_pipeline_in_process(lock.store.clone(), lock.cache.clone());

    if result.is_err() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGetLength).unwrap();
        return Ok(());
    }

    let (node_type_data, composited_clip_data, output) = result.unwrap();

    if !output {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGetLength).unwrap();
        return Ok(());
    }

    let clip = lock.store.clips.composited.get(&composited_clip_id);
    let output_type = composited_clip_data.get(&composited_clip_id);
    if clip.is_none() || output_type.is_none() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGetLength).unwrap();
        return Ok(());
    }
    let clip = clip.unwrap().clone();
    let output_type = output_type.unwrap().clone();

    let existing = lock.video_preview_generation.get(&composited_clip_id);
    if let Some((Some(duration), codec, chunks)) = existing {
        let num_chunks = chunks.len() as u32;

        println!("Number of chunks: {}", num_chunks);
        let duration = duration.clone();
        drop(lock);
        networking::send_message(stream, networking::Message::CompositedClipLength).unwrap();
        networking::send_data(stream, composited_clip_id.as_bytes()).unwrap();
        networking::send_data(stream, &duration.to_ne_bytes()).unwrap();
        networking::send_data(stream, &num_chunks.to_ne_bytes()).unwrap();
        return Ok(());
    }

    lock.video_preview_generation.remove(&composited_clip_id);

    lock.video_preview_generation
        .insert(composited_clip_id.clone(), (None, None, Vec::new()));

    let process = lock.gstreamer_processes.acquire_process();

    if process.is_none() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGetLength).unwrap();
        return Ok(());
    }
    let (process, sender, recv) = process.unwrap();

    drop(lock);
    sender
        .send(IPCMessage::GetLength(clip, output_type))
        .unwrap();

    let message = recv.recv().unwrap();
    match message {
        IPCMessage::CompositedClipLength(id, duration, number_of_chunks) => {
            let mut lock = state.lock().unwrap();
            let statuses = vec![VideoChunkStatus::NotGenerated; number_of_chunks as usize];
            lock.video_preview_generation
                .insert(composited_clip_id.clone(), (Some(duration), None, statuses));
            drop(lock);
            println!("Duration: {}; chunks: {}", duration, number_of_chunks);
            networking::send_message(stream, networking::Message::CompositedClipLength).unwrap();
            networking::send_data(stream, id.as_bytes()).unwrap();
            networking::send_data(stream, &duration.to_ne_bytes()).unwrap();
            networking::send_data(stream, &number_of_chunks.to_ne_bytes()).unwrap();
        }
        _ => {
            todo!();
        }
    }

    Ok(())
}

fn client_message_get_video_preview(
    stream: &mut TcpStream,
    state: Arc<Mutex<State>>,
    operation_id: &str,
) -> Result<(), std::io::Error> {
    println!("Video preview");
    let composited_clip_id = networking::receive_uuid(stream)?;
    let starting_segment = networking::receive_u64(stream)? as u32;
    let ending_segment = networking::receive_u64(stream)? as u32;

    log::warn!(
        "Getting video preview from {} to {} ({})",
        starting_segment,
        ending_segment,
        operation_id
    );

    let mut lock = state.lock().unwrap();
    let result = generate_pipeline_in_process(lock.store.clone(), lock.cache.clone());
    if result.is_err() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGeneratePreview).unwrap();
        return Ok(());
    }

    let (node_type_data, composited_clip_data, output) = result.unwrap();

    if !output {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGeneratePreview).unwrap();
        return Ok(());
    }
    let clip = lock.store.clips.composited.get(&composited_clip_id);
    let output_type = composited_clip_data.get(&composited_clip_id);
    if clip.is_none() || output_type.is_none() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGeneratePreview).unwrap();
        return Ok(());
    }
    let clip = clip.unwrap().clone();
    let output_type = output_type.unwrap().clone();

    let existing = lock.video_preview_generation.get(&composited_clip_id);
    if let Some((Some(duration), codec, chunks)) = existing {
        let chunks = chunks.clone();
        let mut ok = true;
        for i in starting_segment..(ending_segment + 1) {
            let status = chunks.get(i as usize);
            if let Some(status) = status {
                match status {
                    VideoChunkStatus::Generated => {}
                    _ => {
                        ok = false;
                    }
                }
            } else {
                ok = false;
            }
        }

        if ok {
            drop(lock);
            log::warn!(
                "All chunks are already present, so will simply push them all to the client ({})",
                operation_id
            );
            for i in starting_segment..(ending_segment + 1) {
                networking::send_message(stream, networking::Message::NewChunk).unwrap();

                networking::send_data(stream, &i.to_ne_bytes()).unwrap();
            }

            networking::send_message(stream, networking::Message::AllChunksGenerated).unwrap();
            return Ok(());
        }
    }

    lock.video_preview_generation.remove(&composited_clip_id);

    lock.video_preview_generation
        .insert(composited_clip_id.clone(), (None, None, Vec::new()));

    let process = lock.gstreamer_processes.acquire_process();
    if process.is_none() {
        drop(lock);
        networking::send_message(stream, networking::Message::CouldNotGeneratePreview).unwrap();
        return Ok(());
    }
    let (process, sender, recv) = process.unwrap();

    drop(lock);

    sender
        .send(IPCMessage::GeneratePreview(
            clip,
            output_type,
            starting_segment,
            ending_segment,
        ))
        .unwrap();

    let message = recv.recv();
    match message {
        Ok(IPCMessage::CompositedClipLength(id, duration, number_of_chunks)) => {
            let mut lock = state.lock().unwrap();
            let mut statuses = vec![VideoChunkStatus::NotGenerated; number_of_chunks as usize];

            for i in starting_segment..(ending_segment + 1) {
                statuses[i as usize] = VideoChunkStatus::Generating(process.pid().unwrap())
            }

            lock.video_preview_generation
                .insert(composited_clip_id.clone(), (Some(duration), None, statuses));
        }
        Err(e) => {
            panic!("Error encountered!: {:?}", e);
        }
        _ => {
            todo!();
        }
    }

    loop {
        let message = recv.recv();
        match message {
            Ok(IPCMessage::ChunkCompleted(id, chunk)) => {
                let mut lock = state.lock().unwrap();
                let (duration, codec, statuses) =
                    lock.video_preview_generation.get_mut(&id).unwrap();
                statuses[chunk as usize] = VideoChunkStatus::Generated;

                drop(lock);

                networking::send_message(stream, networking::Message::NewChunk).unwrap();

                log::warn!("Chunk {} ready, sending to user ({})", chunk, operation_id);
                networking::send_data(stream, &chunk.to_ne_bytes()).unwrap();
            }
            Ok(IPCMessage::ChunksCompleted(id, start_chunk, end_chunk)) => {
                networking::send_message(stream, networking::Message::AllChunksGenerated).unwrap();
                break;
            }
            Err(e) => {
                panic!("Error encountered!: {:?}", e);
            }
            _ => {
                println!("Invalid message received: ({:?})", message);
                return Ok(());
            }
        }
    }

    let mut lock = state.lock().unwrap();

    lock.gstreamer_processes
        .add_process_to_pool((process, sender, recv));

    // possibly keep track of clips being generated so we don't create the same one twice or whatever

    Ok(())
}

fn generate_pipeline_in_process(
    store: Store,
    cache: Cache,
) -> Result<
    (
        HashMap<
            Uuid,
            (
                HashMap<String, PipedType>,
                HashMap<String, NodeTypeInput>,
                HashMap<String, NodeTypeOutput>,
            ),
        >,
        HashMap<Uuid, PipedType>,
        bool,
    ),
    String,
> {
    // This is quite an unsightly way to do it, however we are limited in options since segmentation faults need to be dealt with.
    let mut i = 0;
    loop {
        i += 1;
        let serialised_store = serde_json::to_string(&store).unwrap();
        let serialised_cache = serde_json::to_string(&cache).unwrap();

        let handle = procspawn::spawn((serialised_store, serialised_cache), |(store, cache)| {
            gst::init().unwrap();
            ges::init().unwrap();

            let current_dir = std::env::current_dir().unwrap();
            let current_dir = current_dir.to_str().unwrap();
            cs310_shared::constants::init(format!("{}/application_data", current_dir), true);

            let store = serde_json::from_str::<Store>(&store).unwrap();
            let cache = serde_json::from_str::<Cache>(&cache).unwrap();

            let node_register = get_node_register();
            let res = store
                .pipeline
                .generate_pipeline(&store, &node_register, true, &cache);

            res
        });

        let res = handle.join();

        if let Ok(res) = res {
            return res;
        }

        if i > 9 {
            return Err(String::from("Limited reached"));
        }
    }
}
