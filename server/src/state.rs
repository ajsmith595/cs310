use std::collections::HashMap;

use cs310_shared::store::Store;
use uuid::Uuid;

use crate::gst_process::ProcessPool;

#[derive(Clone)]
pub enum VideoChunkStatus {
    NotGenerated,
    Requested,
    Generating(u32),
    Generated,
}
pub struct State {
    pub store: Store,
    pub video_preview_generation:
        HashMap<Uuid, (Option<u64>, Option<String>, Vec<VideoChunkStatus>)>,
    pub gstreamer_processes: ProcessPool,
}
