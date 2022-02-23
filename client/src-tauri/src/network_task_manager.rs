use std::{
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipType, SourceClipServerStatus},
  networking,
  store::Store,
  ID,
};
use uuid::Uuid;

use crate::state_manager::SharedState;

#[derive(Clone)]
pub enum NetworkTask {
  GetSourceClipID(ID),
  GetCompositedClipID(ID),
  GetNodeID(ID),
}

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
            }
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
          let store = serde_json::from_slice::<Store>(&new_store_bytes).unwrap();

          let mut lock = shared_state.lock().unwrap();
          lock.store = Some(store);
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
