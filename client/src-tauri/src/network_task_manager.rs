use std::{
  sync::{Arc, Mutex},
  thread,
  time::Duration,
};

use cs310_shared::{networking, store::Store, ID};

use crate::state_manager::SharedState;

#[derive(Clone)]
pub enum Task {
  GetSourceClipID(ID),
  AddCompositedClip(ID),
  UpdateStore(Store),
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
          Task::GetSourceClipID(source_clip_id) => {
            let mut lock = shared_state.lock().unwrap();
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
            }
          }
          Task::AddCompositedClip(_) => todo!(),
          Task::UpdateStore(_) => todo!(),
        }
      }
      should_checksum = true;
    } else if should_checksum {
      // do checksum
      should_checksum = false;
    }

    thread::sleep(Duration::from_secs(2));
  }
}
