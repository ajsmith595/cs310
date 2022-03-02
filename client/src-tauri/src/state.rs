use std::{
  collections::HashMap,
  sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
  },
};

use tauri::{Window, Wry};

use cs310_shared::{
  nodes::NodeRegister,
  store::Store,
  task::{NetworkTask, Task},
};
use uuid::Uuid;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPreviewChunkStatus {
  NotRequested, // front end not asked for
  Requested,    // requested by front end, backend not yet asked server
  Generating,   // asked from server, awaiting response
  Generated,    // server has generated the content
  Downloading,  // currently downloading the content from the server
  Downloaded,   // downloaded, ready to be used by front end
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPreviewStatus {
  NotRequested,
  LengthRequested,
  Data(u64, Vec<VideoPreviewChunkStatus>),
}

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
  pub video_preview_data: HashMap<Uuid, VideoPreviewStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionStatus {
  InitialisingConnection,
  InitialConnectionFailed(String),
  Connected,
  ConnectionFailed(String),
}
