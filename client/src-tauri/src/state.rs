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
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum VideoPreviewChunkStatus {
  NotRequested, // front end not asked for
  Requested,    // requested by front end, backend not yet asked server
  Generating,   // asked from server, awaiting response
  Generated,    // server has generated the content
  Downloading,  // currently downloading the content from the server
  Downloaded,   // downloaded, ready to be used by front end
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum VideoPreviewStatus {
  NotRequested,                                                  // Not yet requested
  LengthRequested, // The number of chunks and the duration of the composited clip have been requested
  Data(u64, Option<String>, bool, Vec<VideoPreviewChunkStatus>), // The duration, codec string and vector of chunk statuses for the composited clip
}

pub struct SharedStateWrapper(pub Arc<Mutex<SharedState>>);

pub struct SharedState {
  pub connection_status: ConnectionStatus, // The current connection status with the server
  pub store: Option<Store>,                // The persistent state
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
