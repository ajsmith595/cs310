use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use super::{
    clip::{CompositedClip, SourceClip},
    node::Node,
    pipeline::Pipeline,
    ID,
};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipStore {
    pub source: HashMap<ID, SourceClip>,
    pub composited: HashMap<ID, CompositedClip>,
}
impl ClipStore {
    pub fn new() -> Self {
        Self {
            source: HashMap::new(),
            composited: HashMap::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Store {
    pub nodes: HashMap<ID, Node>,
    pub clips: ClipStore,
    pub pipeline: Pipeline,
}
impl Store {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            clips: ClipStore::new(),
            pipeline: Pipeline::new(),
        }
    }

    pub fn from_file(filename: String) -> Result<Self, String> {
        let f = std::fs::read(filename);

        if f.is_err() {
            return Err(f.unwrap_err().to_string());
        }
        let data = f.unwrap();

        let store = serde_json::from_slice(&data);

        if store.is_err() {
            return Err(store.unwrap_err().to_string());
        }
        Ok(store.unwrap())
    }

    pub fn merge_client_data(&mut self, other: &Self) {
        self.nodes = other.nodes.clone();
        self.pipeline = other.pipeline.clone();

        let other_clips = other.clips.clone();
        self.clips.composited = other_clips.composited;
        let mut source_clips = HashMap::new();
        for (id, source_clip) in &other_clips.source {
            let original_clip = self.clips.source.get(id);
            if let Some(original_clip) = original_clip {
                let mut clip = original_clip.clone();
                clip.name = source_clip.name.clone();
                source_clips.insert(id, clip);
            } else {
                source_clips.insert(id, source_clip.clone());
            }
        }
        self.clips = other.clips.clone();
    }

    pub fn get_client_checksum(&self) -> u64 {
        let mut self_clone = self.clone();

        // perform client transformations
        for (_, source_clip) in &mut self_clone.clips.source {
            source_clip.file_location = None;
            source_clip.original_device_id = None;
            source_clip.original_file_location = None;
            source_clip.thumbnail_location = None;
        }

        // hash the result
        let bytes = serde_json::to_vec(&self_clone).unwrap();
        let mut hash = DefaultHasher::new();
        bytes.hash(&mut hash);
        hash.finish()
    }
}
