use std::collections::HashMap;

use petgraph::{visit::EdgeRef, EdgeDirection};

use crate::{
    clip::{ClipIdentifier, ClipType},
    nodes::media_import_node,
    store::Store,
    ID,
};

pub struct Cache {
    pub cache_data: HashMap<ID, HashMap<String, ID>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            cache_data: HashMap::new(),
        }
    }

    pub fn get(&self, id: &ID) -> Option<&HashMap<String, ID>> {
        self.cache_data.get(id)
    }

    pub fn clear(&mut self, id: &ID) {
        self.cache_data.remove(id);
    }

    pub fn node_modified(&mut self, id: &ID, store: &Store) {
        let graph = store.pipeline.get_graph(store);
        if let Ok((graph, node_id_to_index)) = graph {
            self.clear(id);
            let mut nodes_to_clear = Vec::new();
            nodes_to_clear.push(id.clone());
            let mut nodes_cleared = Vec::new();
            while nodes_to_clear.len() > 0 {
                let node = nodes_to_clear.pop().unwrap();
                if nodes_cleared.contains(&node) {
                    continue;
                }

                self.clear(&node);

                let edges_out = graph.edges_directed(
                    *node_id_to_index.get_by_left(&node).unwrap(),
                    EdgeDirection::Outgoing,
                );

                for edge in edges_out {
                    let to_node = node_id_to_index
                        .get_by_right(&edge.target())
                        .unwrap()
                        .clone();
                    nodes_to_clear.push(to_node);
                }

                nodes_cleared.push(node);
            }
        }
    }

    pub fn clip_modified(&mut self, clip_id: &ID, clip_type: ClipType, store: &Store) {
        for (id, node) in &store.nodes {
            if node.node_type.as_str() == media_import_node::IDENTIFIER {
                if let Some(clip_data) = node.properties.get(media_import_node::inputs::CLIP) {
                    let clip_identifier =
                        serde_json::from_value::<ClipIdentifier>(clip_data.clone());
                    if let Ok(clip_identifier) = clip_identifier {
                        if clip_identifier.id == clip_id.clone()
                            && clip_identifier.clip_type == clip_type
                        {
                            self.node_modified(id, store);
                        }
                    }
                }
            }
        }
    }

    pub fn add_to_cache(&mut self, id: ID, cache_data: HashMap<String, ID>) {
        self.clear(&id);
        self.cache_data.insert(id, cache_data);
    }
}
