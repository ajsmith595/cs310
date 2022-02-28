use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap},
    hash::{Hash, Hasher},
};

use serde::{Serialize, Serializer};
use uuid::Uuid;

use crate::clip::{ClipIdentifier, ClipType};

fn ordered_map<S, T, X>(value: &HashMap<T, X>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Ord,
    T: Serialize,
    X: Serialize,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

use super::{
    clip::{CompositedClip, SourceClip},
    node::Node,
    pipeline::Pipeline,
    ID,
};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClipStore {
    #[serde(serialize_with = "ordered_map")]
    pub source: HashMap<ID, SourceClip>,
    #[serde(serialize_with = "ordered_map")]
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
    #[serde(serialize_with = "ordered_map")]
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

    pub fn get_client_data(&self) -> Self {
        let mut self_clone = self.clone();

        // perform client transformations
        for (_, source_clip) in &mut self_clone.clips.source {
            source_clip.file_location = None;
            source_clip.original_device_id = None;
            // source_clip.original_file_location = None;
            source_clip.thumbnail_location = None;
        }
        self_clone
    }

    pub fn get_client_checksum(&self) -> u64 {
        let self_clone = self.get_client_data();

        // hash the result
        let bytes = serde_json::to_vec(&self_clone).unwrap();
        let mut hash = DefaultHasher::new();
        bytes.hash(&mut hash);
        hash.finish()
    }

    pub fn move_clip(&mut self, original_clip_id: &Uuid, new_clip_id: &Uuid, clip_type: ClipType) {
        let do_other_transforms = match clip_type {
            ClipType::Source => {
                let clip = self.clips.source.remove(original_clip_id);
                if let Some(mut clip) = clip {
                    clip.id = new_clip_id.clone();
                    self.clips.source.insert(new_clip_id.clone(), clip);
                    true
                } else {
                    false
                }
            }
            ClipType::Composited => {
                let clip = self.clips.composited.remove(original_clip_id);
                if let Some(mut clip) = clip {
                    clip.id = new_clip_id.clone();
                    self.clips.composited.insert(new_clip_id.clone(), clip);
                    true
                } else {
                    false
                }
            }
        };

        if do_other_transforms {
            for (_, node) in &mut self.nodes {
                for (_, value) in &mut node.properties {
                    let decoded_value = serde_json::from_value::<ClipIdentifier>(value.clone());
                    if let Ok(clip_identifier) = decoded_value {
                        if clip_identifier.clip_type == clip_type.clone()
                            && clip_identifier.id == *original_clip_id
                        {
                            *value = serde_json::to_value(&ClipIdentifier {
                                id: new_clip_id.clone(),
                                clip_type: clip_type.clone(),
                            })
                            .unwrap()
                        }
                    }
                }
            }
        }
    }

    pub fn move_node(&mut self, original_node_id: &Uuid, new_node_id: &Uuid) {
        let node = self.nodes.remove(original_node_id);
        let do_other_transforms = if let Some(mut node) = node {
            node.id = new_node_id.clone();
            self.nodes.insert(new_node_id.clone(), node);
            true
        } else {
            false
        };

        if do_other_transforms {
            for link in &mut self.pipeline.links {
                if link.from.node_id == original_node_id.clone() {
                    link.from.node_id = new_node_id.clone();
                }
                if link.to.node_id == original_node_id.clone() {
                    link.to.node_id = new_node_id.clone();
                }
            }
        }
    }
}
