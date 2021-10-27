use std::sync::{Arc, Mutex};

use tauri::{Window, Wry};

use crate::classes::store::Store;

pub struct SharedStateWrapper(pub Arc<Mutex<SharedState>>);

#[derive(Serialize, Deserialize, Debug)]

pub struct StoredState {
  pub store: Store,
  pub file_written: bool,
}
pub struct SharedState {
  pub stored_state: StoredState,
  // thread_stopper: Receiver<()>,
  pub window: Option<Window<Wry>>,
}
