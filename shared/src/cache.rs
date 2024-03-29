use std::collections::HashMap;

use petgraph::{visit::EdgeRef, EdgeDirection};
use serde::Deserialize;

use crate::{
    clip::{ClipIdentifier, ClipType},
    nodes::media_import_node,
    store::Store,
    ID,
};
#[derive(Serialize, Deserialize, Clone, Debug)]

/**
 * The caching structure to assist with ensuring that the correct nodes of the graph are regenerated when a particular node is changed
 */
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

    /**
     * Updates the cache to purge any affected nodes' cache as a result of a node modification
     */
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
    /**
     * Updates the cache to purge any affected nodes' cache as a result of a clip modification
     */
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        global::uniq_id,
        node::{Node, Position},
        nodes::blur_node,
        pipeline::{Link, LinkEndpoint, Pipeline},
        store::{ClipStore, Store},
    };

    use super::Cache;

    #[test]
    fn test_cache_1() {
        let node1 = Node {
            id: uniq_id(),
            group: uniq_id(),
            node_type: blur_node::IDENTIFIER.to_owned(),
            position: Position::new(),
            properties: HashMap::new(),
        };

        let node2 = Node {
            id: uniq_id(),
            group: uniq_id(),
            node_type: blur_node::IDENTIFIER.to_owned(),
            position: Position::new(),
            properties: HashMap::new(),
        };

        let edge = Link {
            from: LinkEndpoint {
                node_id: node1.id.clone(),
                property: blur_node::outputs::OUTPUT.to_owned(),
            },
            to: LinkEndpoint {
                node_id: node2.id.clone(),
                property: blur_node::inputs::MEDIA.to_owned(),
            },
        };

        let mut hm = HashMap::new();
        hm.insert(node1.id.clone(), node1.clone());
        hm.insert(node2.id.clone(), node2.clone());

        let mut pipeline = Pipeline::new();
        pipeline.links.push(edge);

        let store = Store {
            nodes: hm,
            clips: ClipStore::new(),
            pipeline,
        };

        let mut cache = Cache::new();
        cache.add_to_cache(node1.id.clone(), HashMap::new());
        cache.add_to_cache(node2.id.clone(), HashMap::new());

        cache.node_modified(&node1.id, &store);
        assert!(cache.cache_data.get(&node2.id).is_none());
    }

    #[test]
    fn test_cache_2() {
        let node1 = Node {
            id: uniq_id(),
            group: uniq_id(),
            node_type: blur_node::IDENTIFIER.to_owned(),
            position: Position::new(),
            properties: HashMap::new(),
        };

        let node2 = Node {
            id: uniq_id(),
            group: uniq_id(),
            node_type: blur_node::IDENTIFIER.to_owned(),
            position: Position::new(),
            properties: HashMap::new(),
        };

        let edge = Link {
            from: LinkEndpoint {
                node_id: node1.id.clone(),
                property: blur_node::outputs::OUTPUT.to_owned(),
            },
            to: LinkEndpoint {
                node_id: node2.id.clone(),
                property: blur_node::inputs::MEDIA.to_owned(),
            },
        };

        let mut hm = HashMap::new();
        hm.insert(node1.id.clone(), node1.clone());
        hm.insert(node2.id.clone(), node2.clone());

        let mut pipeline = Pipeline::new();
        pipeline.links.push(edge);

        let store = Store {
            nodes: hm,
            clips: ClipStore::new(),
            pipeline,
        };

        let mut cache = Cache::new();
        cache.add_to_cache(node1.id.clone(), HashMap::new());
        cache.add_to_cache(node2.id.clone(), HashMap::new());

        cache.node_modified(&node2.id, &store);
        assert!(cache.cache_data.get(&node1.id).is_some());
    }
}
