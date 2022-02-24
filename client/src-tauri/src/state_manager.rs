use std::sync::{
  mpsc::{Receiver, Sender},
  Arc, Mutex,
};

use tauri::{Window, Wry};

use cs310_shared::{
  nodes::NodeRegister,
  store::Store,
  task::{NetworkTask, Task},
};

pub struct SharedStateWrapper(pub Arc<Mutex<SharedState>>);

pub struct SharedState {
  pub file_written: bool,
  pub connection_status: ConnectionStatus,
  pub store: Option<Store>,
  pub thread_stopper: Receiver<()>,
  pub task_manager_notifier: Option<Sender<bool>>,
  pub window: Option<Window<Wry>>,
  pub node_register: NodeRegister,
  pub tasks: Vec<Task>,
  pub network_jobs: Vec<NetworkTask>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionStatus {
  InitialisingConnection,
  InitialConnectionFailed(String),
  Connected,
  ConnectionFailed(String),
}
