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
  store::Store,
  task::Task,
  ID,
};
use serde_json::Value;
use uuid::Uuid;

use crate::state::SharedState;

pub fn task_manager_thread(shared_state: Arc<Mutex<SharedState>>, rx: Receiver<bool>) {
  loop {
    let response = rx.recv().unwrap(); // Wait for someone to notify you of a change made to the queue
    if !response {
      return;
    }

    // Get all the tasks off the queue, and modify the persistent state accordingly.
    let mut lock = shared_state.lock().unwrap();
    let tasks = lock.tasks.clone();
    lock.tasks.clear();
    let mutable_store = lock.store.as_mut().unwrap();
    let mut network_jobs = Task::apply_tasks(mutable_store, tasks); // This returns the set of network tasks that are expected to be executed by the network task manager.
    lock.network_jobs.append(&mut network_jobs);

    if lock.window.is_some() && lock.store.is_some() {
      lock
        .window
        .as_ref()
        .unwrap()
        .emit("store-update", lock.store.as_ref().unwrap().clone())
        .unwrap();
      // Update the UI with the new store when complete
    }
  }
}
