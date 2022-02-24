use std::{
  collections::HashMap,
  sync::{mpsc::Receiver, Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{
  clip::{ClipIdentifier, ClipType, CompositedClip, SourceClip},
  node::{Node, Position},
  pipeline::Link,
  ID,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{network_task_manager, state_manager::SharedState};
#[derive(Clone)]
pub enum Task {
  UpdateNode(ID, Node),
  AddNode(Node),
  AddLink(Link),
  DeleteLinks(ID, Option<String>),
  DeleteNode(ID),
  UpdateClip(ID, ClipType, Value),
  CreateSourceClip(SourceClip),
  CreateCompositedClip(CompositedClip),
}

pub fn task_manager_thread(shared_state: Arc<Mutex<SharedState>>, rx: Receiver<bool>) {
  loop {
    let response = rx.recv().unwrap();
    if !response {
      return;
    }
    thread::sleep(Duration::from_millis(10)); // wait a tiny bit so that if we're doing lots of things at once, we allow that to happen
    let mut lock = shared_state.lock().unwrap();
    let tasks = lock.tasks.clone();
    lock.tasks.clear();
    let mut network_jobs = Vec::new();

    let mutable_store = lock.store.as_mut().unwrap();
    for task in tasks {
      match task {
        Task::UpdateNode(id, node) => {
          mutable_store.nodes.insert(id, node);
          network_jobs.push(network_task_manager::NetworkTask::UpdateNode(id));
        }
        Task::AddNode(node) => {
          let id = node.id.clone();
          network_jobs.push(network_task_manager::NetworkTask::GetNodeID(id.clone()));
          mutable_store.nodes.insert(id, node);
        }
        Task::AddLink(link) => {
          let mut new_links = mutable_store.pipeline.links.clone();
          new_links = new_links
            .into_iter()
            .filter(|x| x.to.get_id() != link.to.get_id())
            .collect();

          network_jobs.push(network_task_manager::NetworkTask::AddLink(link.clone()));
          new_links.push(link);

          mutable_store.pipeline.links = new_links;
        }
        Task::DeleteLinks(id, property) => {
          let mut new_links = mutable_store.pipeline.links.clone();

          new_links = match property.clone() {
            None => new_links
              .into_iter()
              .filter(|x: &Link| x.to.node_id != id)
              .collect(),
            Some(prop) => new_links
              .into_iter()
              .filter(|x: &Link| x.to.node_id != id && x.to.property != prop)
              .collect(),
          };
          mutable_store.pipeline.links = new_links;
          network_jobs.push(network_task_manager::NetworkTask::DeleteLinks(id, property));
        }
        Task::DeleteNode(id) => {
          let mut new_links = mutable_store.pipeline.links.clone();

          new_links = new_links
            .into_iter()
            .filter(|x: &Link| x.to.node_id != id && x.from.node_id != id)
            .collect();
          mutable_store.pipeline.links = new_links;
          mutable_store.nodes.remove(&id);

          network_jobs.push(network_task_manager::NetworkTask::DeleteNode(id));
        }
        Task::CreateSourceClip(clip) => {
          network_jobs.push(network_task_manager::NetworkTask::GetSourceClipID(
            clip.id.clone(),
          ));
          mutable_store.clips.source.insert(clip.id.clone(), clip);
        }
        Task::CreateCompositedClip(clip) => {
          let mut output_node = Node::new(String::from("output"), Some(Uuid::new_v4()));
          let clip_identifier = ClipIdentifier {
            id: clip.id,
            clip_type: ClipType::Composited,
          };
          output_node.properties.insert(
            String::from("clip"),
            serde_json::to_value(&clip_identifier).unwrap(),
          );

          network_jobs.push(network_task_manager::NetworkTask::GetCompositedClipID(
            clip.id.clone(),
          ));
          mutable_store.clips.composited.insert(clip.id.clone(), clip);
        }
        Task::UpdateClip(id, clip_type, clip) => {
          match clip_type {
            ClipType::Source => {
              let clip = serde_json::from_value::<SourceClip>(clip);
              if clip.is_err() {
                continue;
              }
              let mut clip = clip.unwrap();

              clip.id = id.clone();
              let existing_clip = mutable_store.clips.source.get_mut(&id);
              if existing_clip.is_none() {
                continue;
              }
              let existing_clip = existing_clip.unwrap();
              *existing_clip = clip;
            }
            ClipType::Composited => {
              let clip = serde_json::from_value::<CompositedClip>(clip);
              if clip.is_err() {
                continue;
              }
              let mut clip = clip.unwrap();

              clip.id = id.clone();
              let existing_clip = mutable_store.clips.composited.get_mut(&id);
              if existing_clip.is_none() {
                continue;
              }
              let existing_clip = existing_clip.unwrap();
              *existing_clip = clip;
            }
          }
          network_jobs.push(network_task_manager::NetworkTask::UpdateClip(id, clip_type));
        }
      }
    }
    lock.network_jobs.append(&mut network_jobs);

    if lock.window.is_some() && lock.store.is_some() {
      lock
        .window
        .as_ref()
        .unwrap()
        .emit("store-update", lock.store.as_ref().unwrap().clone())
        .unwrap();
    }
  }
}
