use std::{
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipType, SourceClipServerStatus},
  networking,
  store::Store,
  task::NetworkTask,
};
use uuid::Uuid;

use crate::state::{SharedState, VideoPreviewStatus};

/**
 * Handles all network tasks that need to be sent to the server
 */
pub fn network_task_manager_thread(shared_state: Arc<Mutex<SharedState>>) {
  let mut should_checksum = false;
  loop {
    let mut lock = shared_state.lock().unwrap();
    if !lock.network_jobs.is_empty() {
      let jobs = lock.network_jobs.clone();
      lock.network_jobs.clear();

      // We perform all the currently queued network jobs in one go
      drop(lock);
      for job in jobs {
        match job {
          NetworkTask::GetSourceClipID(source_clip_id) => {
            // Creates a source clip, and gets the server's generated ID for that source clip, and updates the state to move the ID of the clip to that new ID

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
                // Ready to be uploaded
              }
            }
          }
          NetworkTask::GetCompositedClipID(composited_clip_id) => {
            // Same situation as the source clip; get a new ID from the server, and move the clip

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
                .video_preview_data
                .insert(uuid.clone(), VideoPreviewStatus::NotRequested);

              lock
                .window
                .as_ref()
                .unwrap()
                .emit("store-update", lock.store.as_ref().unwrap().clone())
                .unwrap();
            }
          }
          NetworkTask::GetNodeID(node_id) => {
            // Same concept as the clips; create a node, and get the new ID from the server

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

              let composited_clip_id = lock
                .store
                .as_ref()
                .unwrap()
                .get_clip_from_group(node.group.clone());

              // We then need to reset the video preview data for that node's group's composited clip, since its pipeline has now been changed

              if let Some(composited_clip_id) = composited_clip_id {
                // Reset the composited clip
                lock
                  .video_preview_data
                  .insert(composited_clip_id, VideoPreviewStatus::LengthRequested);
              }

              let window = lock.window.as_ref().unwrap();

              window
                .emit("move-node", (node_id.clone(), uuid.clone()))
                .unwrap();

              window
                .emit("store-update", lock.store.as_ref().unwrap().clone())
                .unwrap();

              if composited_clip_id.is_some() {
                window
                  .emit("video-preview-data-update", lock.video_preview_data.clone())
                  .unwrap();
              }
            }
          }
          NetworkTask::UpdateNode(node_id) => {
            // Simply update a node, and then, like in the node creation, we reset the video preview of the relevant composited clip

            let lock = shared_state.lock().unwrap();
            let node = lock.store.as_ref().unwrap().nodes.get(&node_id);
            if let Some(node) = node {
              let node = node.clone();
              drop(lock);
              let bytes = serde_json::to_vec(&node).unwrap();
              let mut stream = networking::connect_to_server().unwrap();
              networking::send_message(&mut stream, networking::Message::UpdateNode).unwrap();
              networking::send_as_file(&mut stream, &bytes);

              let mut lock = shared_state.lock().unwrap();
              let composited_clip_id = lock
                .store
                .as_ref()
                .unwrap()
                .get_clip_from_group(node.group.clone());

              if let Some(composited_clip_id) = composited_clip_id {
                // Reset the composited clip
                lock
                  .video_preview_data
                  .insert(composited_clip_id, VideoPreviewStatus::LengthRequested);

                let window = lock.window.as_ref().unwrap();
                window
                  .emit("video-preview-data-update", lock.video_preview_data.clone())
                  .unwrap();
              }
            }
          }
          NetworkTask::AddLink(link) => {
            // Add a link between two nodes, and again, reset the video preview data for the relevant composited clip

            let bytes = serde_json::to_vec(&link).unwrap();
            let mut stream = networking::connect_to_server().unwrap();
            networking::send_message(&mut stream, networking::Message::AddLink).unwrap();
            networking::send_as_file(&mut stream, &bytes);

            let mut lock = shared_state.lock().unwrap();

            let group = lock
              .store
              .as_ref()
              .unwrap()
              .nodes
              .get(&link.to.node_id)
              .unwrap()
              .group
              .clone();

            let composited_clip_id = lock.store.as_ref().unwrap().get_clip_from_group(group);

            if let Some(composited_clip_id) = composited_clip_id {
              lock
                .video_preview_data
                .insert(composited_clip_id, VideoPreviewStatus::LengthRequested);
            }
          }
          NetworkTask::DeleteLinks(node_id, property) => {
            // Delete all links for a particular node (if property specified, we only remove links linked to that particular input/output), and again, reset the video preview data for the relevant composited clip

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

            let mut lock = shared_state.lock().unwrap();

            let group = lock
              .store
              .as_ref()
              .unwrap()
              .nodes
              .get(&node_id)
              .unwrap()
              .group
              .clone();

            let composited_clip_id = lock.store.as_ref().unwrap().get_clip_from_group(group);

            if let Some(composited_clip_id) = composited_clip_id {
              lock
                .video_preview_data
                .insert(composited_clip_id, VideoPreviewStatus::LengthRequested);
            }
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
      // If we have a situation whereby some jobs were performed in one loop, and in the subsequent loop no new jobs have been added, we perform a checksum.

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
