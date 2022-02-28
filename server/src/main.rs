use core::time;
use cs310_shared::{
    clip::{ClipType, CompositedClip, SourceClip, SourceClipServerStatus},
    constants::{source_files_location, store_json_location, CHUNK_LENGTH},
    networking::{self, SERVER_PORT},
    node::Node,
    nodes::{get_node_register, NodeRegister},
    pipeline::Link,
    store::Store,
    task::Task,
};
use ges::traits::{GESPipelineExt, LayerExt, TimelineExt};
use gst::prelude::*;
use num_traits::cast::FromPrimitive;
use serde_json::Value;
use simple_logger::SimpleLogger;
use state::State;
use std::sync::{Arc, Mutex};
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
use uuid::Uuid;

use crate::gst_process::{IPCMessage, ProcessPool};

mod gst_process;
mod state;
mod utility;

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

    let state = Arc::new(Mutex::new(State {
        store,
        gstreamer_processes: ProcessPool::new(8),
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
            Err(_) => {}
        }
        if !running.load(Ordering::SeqCst) {
            break;
        }
    }

    store_saver_thread.join().unwrap();

    drop(listener);
}

fn handle_client(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    log::info!("Handling client: {}", stream.peer_addr().unwrap());

    while match networking::receive_message(&mut stream) {
        Ok(message) => {
            let operation_id = &format!("{}", Uuid::new_v4())[..8];

            println!("Message: {:?}", message);
            log::info!(
                "[{}] New operation from: {}",
                operation_id,
                stream.peer_addr().unwrap()
            );

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
                    log::info!("[{}] Receiving file", operation_id);
                    let uuid = utility::receive_uuid(&mut stream);

                    log::info!("[{}] File ID: {}", operation_id, uuid);

                    let mut locked_state = state.lock().unwrap();

                    let store = &mut locked_state.store;
                    let clip = store.clips.source.get_mut(&uuid);

                    if clip.is_none() {
                        log::warn!("[{}] Client tried to upload file for a source clip which did not exist", operation_id);
                        stream.shutdown(Shutdown::Both).unwrap();
                        return;
                    }
                    let clip = clip.unwrap();

                    match &clip.status {
                        SourceClipServerStatus::LocalOnly => {}
                        _ => {
                            log::warn!("[{}] Client tried to upload file for a source clip which is not marked as LocalOnly", operation_id);
                            stream.shutdown(Shutdown::Both).unwrap();
                            return;
                        }
                    }
                    clip.status = SourceClipServerStatus::Uploading;
                    drop(locked_state);

                    let mut output_file =
                        File::create(format!("{}/{}", source_files_location(), uuid)).unwrap();
                    networking::receive_file(&mut stream, &mut output_file);
                    let msg = networking::receive_message(&mut stream).unwrap();
                    let mut locked_state = state.lock().unwrap();
                    let store = &mut locked_state.store;
                    let clip = store.clips.source.get_mut(&uuid).unwrap();
                    clip.status = SourceClipServerStatus::Uploaded;

                    log::info!("[{}] File received successfully", operation_id);
                }
                networking::Message::CreateSourceClip => {
                    let clip_data = networking::receive_file_as_bytes(&mut stream);
                    let clip = serde_json::from_slice::<SourceClip>(&clip_data);
                    if clip.is_err() {
                        log::warn!("[{}] Client sent invalid JSON data!", operation_id);
                        return;
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
                        return;
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
                    let checksum = utility::receive_u64(&mut stream);

                    let store_checksum = state.lock().unwrap().store.get_client_checksum();
                    if store_checksum != checksum {
                        log::warn!("Checksum not the same! Updating client");
                        // update client

                        networking::send_message(&mut stream, networking::Message::ChecksumError)
                            .unwrap(); // TODO: change to error type

                        let data = &state.lock().unwrap().store.get_client_data();
                        let string = serde_json::to_string(data).unwrap();

                        println!("Sending new JSON: {}", string);

                        let bytes = string.as_bytes();
                        networking::send_as_file(&mut stream, bytes);
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
                        return;
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
                        return;
                    }
                    let node = node.unwrap();
                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::UpdateNode(node.id.clone(), node)]);
                }
                networking::Message::AddLink => {
                    let bytes = networking::receive_file_as_bytes(&mut stream);
                    let link = serde_json::from_slice::<Link>(&bytes);
                    if link.is_err() {
                        log::warn!("Client sent invalid JSON!");
                        return;
                    }
                    let link = link.unwrap();
                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::AddLink(link)]);
                }
                networking::Message::DeleteLinks => {
                    let uuid = utility::receive_uuid(&mut stream);

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
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::DeleteLinks(uuid, property)]);
                }
                networking::Message::UpdateClip => {
                    let clip_type_bytes = networking::receive_data(&mut stream, 1).unwrap();
                    let clip_type = ClipType::from_u8(clip_type_bytes[0]);
                    if clip_type.is_none() {
                        log::warn!("Client sent invalid clip type!");
                        return;
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
                                return;
                            }
                            let clip = clip.unwrap();
                            let id = clip.id.clone();
                            (id, serde_json::to_value(clip).unwrap())
                        }
                        ClipType::Composited => {
                            let clip = serde_json::from_slice::<CompositedClip>(&bytes);
                            if clip.is_err() {
                                log::warn!("Client sent invalid JSON!");
                                return;
                            }
                            let clip = clip.unwrap();
                            let id = clip.id.clone();
                            (id, serde_json::to_value(clip).unwrap())
                        }
                    };
                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::UpdateClip(id, clip_type, clip)]);
                }
                networking::Message::DeleteNode => {
                    let uuid = utility::receive_uuid(&mut stream);

                    let mut lock = state.lock().unwrap();
                    let store = lock.store.borrow_mut();
                    Task::apply_tasks(store, vec![Task::DeleteNode(uuid)]);
                }
                networking::Message::GetVideoPreview => {
                    let composited_clip_id = utility::receive_uuid(&mut stream);
                    let starting_segment = utility::receive_u64(&mut stream) as u32;
                    let ending_segment = utility::receive_u64(&mut stream) as u32;

                    let mut lock = state.lock().unwrap();
                    let process = lock.gstreamer_processes.acquire_process();
                    if process.is_none() {
                        networking::send_message(
                            &mut stream,
                            networking::Message::CouldNotGeneratePreview,
                        )
                        .unwrap();
                        return;
                    }
                    let (process, sender, recv) = process.unwrap();

                    let result =
                        lock.store
                            .pipeline
                            .gen_graph_new(&lock.store, &get_node_register(), true);
                    if result.is_err() {
                        networking::send_message(
                            &mut stream,
                            networking::Message::CouldNotGeneratePreview,
                        )
                        .unwrap();
                        return;
                    }

                    let (node_type_data, composited_clip_data, output) = result.unwrap();

                    if !output {
                        networking::send_message(
                            &mut stream,
                            networking::Message::CouldNotGeneratePreview,
                        )
                        .unwrap();
                        return;
                    }
                    let clip = lock.store.clips.composited.get(&composited_clip_id);
                    let output_type = composited_clip_data.get(&composited_clip_id);
                    if clip.is_none() || output_type.is_none() {
                        networking::send_message(
                            &mut stream,
                            networking::Message::CouldNotGeneratePreview,
                        )
                        .unwrap();
                        return;
                    }
                    let clip = clip.unwrap().clone();
                    let output_type = output_type.unwrap().clone();

                    drop(lock);

                    sender
                        .send(IPCMessage::GeneratePreview(
                            clip,
                            output_type,
                            starting_segment,
                            ending_segment,
                        ))
                        .unwrap();

                    // receive messages for generated chunks
                    // readd the process back to the pool
                    // possibly keep track of clips being generated so we don't create the same one twice or whatever
                }
                _ => {
                    log::error!(
                        "[{}] Unknown message received; terminating connection",
                        operation_id
                    );
                    stream.shutdown(Shutdown::Both).unwrap();
                    return;
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
