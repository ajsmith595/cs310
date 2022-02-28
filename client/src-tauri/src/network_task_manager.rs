use std::{
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipType, SourceClipServerStatus},
  networking,
  pipeline::Link,
  store::Store,
  task::NetworkTask,
  ID,
};
use uuid::Uuid;

use crate::state::SharedState;

pub fn network_task_manager_thread(shared_state: Arc<Mutex<SharedState>>) {
  let mut should_checksum = false;
  loop {
    let mut lock = shared_state.lock().unwrap();
    if !lock.network_jobs.is_empty() {
      let jobs = lock.network_jobs.clone();
      lock.network_jobs.clear();
      drop(lock);
      for job in jobs {
        match job {
          NetworkTask::GetSourceClipID(source_clip_id) => {
            let lock = shared_state.lock().unwrap();
            let clip = lock
              .store
              .as_ref()
              .unwrap()
              .clips
              .source
              .get(&source_clip_id);
            if let Some(clip) = clip {
              let clip = clip.clone();
              drop(lock);

              let bytes = serde_json::to_vec(&clip).unwrap();
              let mut stream = networking::connect_to_server().unwrap();

              println!(
                "Sending 'CreateSourceClip' as: {}",
                networking::Message::CreateSourceClip as u8
              );
              networking::send_message(&mut stream, networking::Message::CreateSourceClip).unwrap();
              networking::send_as_file(&mut stream, &bytes);

              let temp = networking::receive_data(&mut stream, 16).unwrap();
              let mut uuid_bytes = [0 as u8; 16];
              uuid_bytes.copy_from_slice(&temp);
              let uuid = Uuid::from_bytes(uuid_bytes);

              let mut lock = shared_state.lock().unwrap();
              lock
                .store
                .as_mut()
                .unwrap()
                .move_clip(&source_clip_id, &uuid, ClipType::Source);
              let clip = lock
                .store
                .as_mut()
                .unwrap()
                .clips
                .source
                .get_mut(&uuid)
                .unwrap();
              if clip.status == SourceClipServerStatus::NeedsNewID {
                clip.status = SourceClipServerStatus::LocalOnly;
              }
            }
          }
          NetworkTask::GetCompositedClipID(composited_clip_id) => {
            let lock = shared_state.lock().unwrap();
            let clip = lock
              .store
              .as_ref()
              .unwrap()
              .clips
              .composited
              .get(&composited_clip_id);
            if let Some(clip) = clip {
              let clip = clip.clone();
              drop(lock);

              let bytes = serde_json::to_vec(&clip).unwrap();
              let mut stream = networking::connect_to_server().unwrap();
              networking::send_message(&mut stream, networking::Message::CreateCompositedClip)
                .unwrap();
              networking::send_as_file(&mut stream, &bytes);

              let temp = networking::receive_data(&mut stream, 16).unwrap();
              let mut uuid_bytes = [0 as u8; 16];
              uuid_bytes.copy_from_slice(&temp);
              let uuid = Uuid::from_bytes(uuid_bytes);

              let mut lock = shared_state.lock().unwrap();
              lock.store.as_mut().unwrap().move_clip(
                &composited_clip_id,
                &uuid,
                ClipType::Composited,
              );

              lock
                .window
                .as_ref()
                .unwrap()
                .emit("store-update", lock.store.as_ref().unwrap().clone())
                .unwrap();
            }
          }
          NetworkTask::GetNodeID(node_id) => {
            let lock = shared_state.lock().unwrap();
            let node = lock.store.as_ref().unwrap().nodes.get(&node_id);
            if let Some(node) = node {
              let node = node.clone();
              drop(lock);
              let bytes = serde_json::to_vec(&node).unwrap();
              let mut stream = networking::connect_to_server().unwrap();
              networking::send_message(&mut stream, networking::Message::CreateNode).unwrap();
              networking::send_as_file(&mut stream, &bytes);

              let temp = networking::receive_data(&mut stream, 16).unwrap();
              let mut uuid_bytes = [0 as u8; 16];
              uuid_bytes.copy_from_slice(&temp);
              let uuid = Uuid::from_bytes(uuid_bytes);

              let mut lock = shared_state.lock().unwrap();
              lock.store.as_mut().unwrap().move_node(&node_id, &uuid);

              let window = lock.window.as_ref().unwrap();

              window
                .emit("move-node", (node_id.clone(), uuid.clone()))
                .unwrap();

              window
                .emit("store-update", lock.store.as_ref().unwrap().clone())
                .unwrap();
            }
          }
          NetworkTask::UpdateNode(node_id) => {
            let lock = shared_state.lock().unwrap();
            let node = lock.store.as_ref().unwrap().nodes.get(&node_id);
            if let Some(node) = node {
              let node = node.clone();
              drop(lock);
              let bytes = serde_json::to_vec(&node).unwrap();
              let mut stream = networking::connect_to_server().unwrap();
              networking::send_message(&mut stream, networking::Message::UpdateNode).unwrap();
              networking::send_as_file(&mut stream, &bytes);
            }
          }
          NetworkTask::AddLink(link) => {
            let bytes = serde_json::to_vec(&link).unwrap();
            let mut stream = networking::connect_to_server().unwrap();
            networking::send_message(&mut stream, networking::Message::AddLink).unwrap();
            networking::send_as_file(&mut stream, &bytes);
          }
          NetworkTask::DeleteLinks(node_id, property) => {
            let bytes = node_id.as_bytes();
            let mut stream = networking::connect_to_server().unwrap();
            networking::send_message(&mut stream, networking::Message::DeleteLinks).unwrap();
            networking::send_data(&mut stream, bytes).unwrap();
            let property = match property {
              Some(prop) => prop,
              None => String::from(""),
            };
            let bytes = property.as_bytes();
            networking::send_as_file(&mut stream, bytes);
          }
          NetworkTask::DeleteNode(node_id) => {
            let bytes = node_id.as_bytes();
            let mut stream = networking::connect_to_server().unwrap();
            networking::send_message(&mut stream, networking::Message::DeleteNode).unwrap();
            networking::send_data(&mut stream, bytes).unwrap();
          }
          NetworkTask::UpdateClip(clip_id, clip_type) => {
            let lock = shared_state.lock().unwrap();

            let clip = match clip_type {
              ClipType::Source => {
                let clip = lock.store.as_ref().unwrap().clips.source.get(&clip_id);
                if let Some(clip) = clip {
                  let clip = clip.clone();

                  Some(serde_json::to_vec(&clip).unwrap())
                } else {
                  None
                }
              }
              ClipType::Composited => {
                let clip = lock.store.as_ref().unwrap().clips.composited.get(&clip_id);
                if let Some(clip) = clip {
                  let clip = clip.clone();

                  Some(serde_json::to_vec(&clip).unwrap())
                } else {
                  None
                }
              }
            };
            if clip.is_none() {
              return;
            }
            let clip = clip.unwrap();

            drop(lock);
            let mut stream = networking::connect_to_server().unwrap();
            let clip_type_bytes = (clip_type as u8).to_ne_bytes();
            networking::send_message(&mut stream, networking::Message::UpdateClip).unwrap();
            networking::send_data(&mut stream, &clip_type_bytes).unwrap();
            networking::send_as_file(&mut stream, &clip);
          }
        }
      }
      should_checksum = true;
    } else if should_checksum {
      // do checksum

      let checksum = lock.store.as_ref().unwrap().get_client_checksum();
      drop(lock);
      let mut stream = networking::connect_to_server().unwrap();
      networking::send_message(&mut stream, networking::Message::Checksum).unwrap();

      let bytes = checksum.to_ne_bytes();
      networking::send_data(&mut stream, &bytes).unwrap();
      let response = networking::receive_message(&mut stream).unwrap();
      match response {
        networking::Message::ChecksumError => {
          let new_store_bytes = networking::receive_file_as_bytes(&mut stream);
          let store_str = String::from_utf8(new_store_bytes).unwrap();
          println!("Decoding string: {}", store_str);
          let store = serde_json::from_str::<Store>(&store_str).unwrap();

          let mut lock = shared_state.lock().unwrap();
          lock.store = Some(store);
          println!("NOTE: checksum does not match so store has been updated!");

          lock
            .window
            .as_ref()
            .unwrap()
            .emit("store-update", lock.store.as_ref().unwrap().clone())
            .unwrap();
        }
        networking::Message::ChecksumOk => {}
        _ => panic!("Invalid response from server!"),
      }

      should_checksum = false;
    } else {
      drop(lock);
    }

    thread::sleep(Duration::from_secs(2));
  }
}
