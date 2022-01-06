use std::sync::{mpsc::Receiver, Arc, Mutex};

use tauri::{Window, Wry};

use crate::classes::{nodes::NodeRegister, store::Store};
use serde_json::Value;

pub struct SharedStateWrapper(pub Arc<Mutex<SharedState>>);

pub struct SharedState {
  pub file_written: bool,
  pub pipeline_executed: bool,
  pub store: Store,
  pub thread_stopper: Receiver<()>,
  pub window: Option<Window<Wry>>,
  pub node_register: NodeRegister,
}
