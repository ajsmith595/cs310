use std::collections::HashMap;

use cs310_shared::{cache::Cache, clip::ClipType, store::Store, ID};
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
    pub cache: Cache,
}

impl State {
    pub fn cache_node_modified(&mut self, id: &ID) {
        self.cache.node_modified(&id, &self.store);
    }

    pub fn cache_clip_modified(&mut self, id: &ID, clip_type: ClipType) {
        self.cache.clip_modified(&id, clip_type, &self.store);
    }
}
