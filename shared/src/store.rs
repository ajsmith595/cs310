use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap},
    hash::{Hash, Hasher},
};

use serde::{Serialize, Serializer};
use uuid::Uuid;

use crate::{
    clip::{ClipIdentifier, ClipType},
    nodes::output_node,
};

/// ---------------------------------------------------------------------------------------
/// Code from: https://stackoverflow.com/a/42723390
/// ---------------------------------------------------------------------------------------

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
    // we serialise with this to ensure that the clips do not spontaneously change position in the UI
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

    /**
     * Utility function for obtaining a store via a JSON file
     */
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

    /**
     * Returns a new struct with client-only data
     */
    pub fn get_client_data(&self) -> Self {
        let mut self_clone = self.clone();

        // perform client transformations
        for (_, source_clip) in &mut self_clone.clips.source {
            source_clip.file_location = None;
            source_clip.original_device_id = None;
            // source_clip.original_file_location = None; // TODO: get client's device ID, pass to this function
            source_clip.thumbnail_location = None;
        }
        self_clone
    }

    /**
     * Gets a checksum value of the client data of this store. Used to compare between server and client
     */
    pub fn get_client_checksum(&self) -> u64 {
        let self_clone = self.get_client_data();

        // hash the result
        let bytes = serde_json::to_vec(&self_clone).unwrap();
        let mut hash = DefaultHasher::new();
        bytes.hash(&mut hash);
        hash.finish()
    }

    /**
     * Moves a particular clip from one ID to another
     */
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
            // Performed if the clip actually did exist
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

    /**
     * Moves a particular node from one ID to another
     */
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

    /**
     * Finds the composited clip ID from a particular node's ID - will find the composited clip associated with its group
     */
    pub fn get_clip_from_group(&self, group: Uuid) -> Option<ID> {
        for (id, n) in &self.nodes {
            if group == n.group && n.node_type == output_node::IDENTIFIER {
                let prop = n.properties.get(output_node::inputs::CLIP);
                if let Some(prop) = prop {
                    let clip_identifier = serde_json::from_value::<ClipIdentifier>(prop.clone());
                    if let Ok(clip_identifier) = clip_identifier {
                        return Some(clip_identifier.id);
                    }
                }
            }
        }

        None
    }
}
