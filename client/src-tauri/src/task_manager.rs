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

use crate::state_manager::SharedState;

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
    let mutable_store = lock.store.as_mut().unwrap();
    let mut network_jobs = Task::apply_tasks(mutable_store, tasks);
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
