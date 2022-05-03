use std::{collections::HashMap, fs, sync::mpsc, thread};

use ges::traits::TimelineExt;
use gst::{glib, prelude::*};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{graph::DiGraph, EdgeDirection};

use bimap::BiMap;
use uuid::Uuid;

use crate::cache::Cache;
use crate::{
    clip::{ClipIdentifier, ClipType},
    node::{InputOrOutput, PipedType},
    nodes::NodeRegister,
};

use super::{
    node::{Node, NodeTypeInput, NodeTypeOutput},
    nodes::{media_import_node, output_node},
    store::Store,
    ID,
};

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct LinkEndpoint {
    pub node_id: ID,
    pub property: String,
}
impl LinkEndpoint {
    pub fn get_id(&self) -> String {
        return String::from(self.node_id.to_string() + "." + &self.property);
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Link {
    pub from: LinkEndpoint,
    pub to: LinkEndpoint,
}
impl Link {
    pub fn get_id(&self) -> String {
        return String::from(self.from.get_id() + "-" + &self.to.get_id());
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pipeline {
    pub links: Vec<Link>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    /**
     * Generates a directed graph representation; no data will be supplied to the nodes or edges
     * Also returns a `BiMap` indicating what node IDs correspond to the `NodeIndex` in the graph
     */
    pub fn get_graph(
        &self,
        store: &Store,
    ) -> Result<
        (
            DiGraph<HashMap<String, PipedType>, Option<(String, String)>>,
            BiMap<Uuid, NodeIndex>,
        ),
        String,
    > {
        let mut graph = DiGraph::new();

        let mut node_id_to_index = BiMap::new();
        let mut composited_clip_to_index = BiMap::new();

        // populate nodes, and keep track of both the node ID assignments to indexes in the graph, as well as indexes of composited clip output nodes, so we can later link them to media import nodes
        for (id, node) in &store.nodes {
            let node_idx = graph.add_node(HashMap::new());
            node_id_to_index.insert(id.to_owned(), node_idx);

            if node.node_type == output_node::IDENTIFIER {
                let clip = node.properties.get(output_node::inputs::CLIP);
                if clip.is_none() {
                    return Err(format!("Output node with no clip detected!"));
                }
                let clip = clip.unwrap().to_owned();
                let clip = serde_json::from_value::<ClipIdentifier>(clip);

                if clip.is_err() {
                    return Err(format!("Clip identifier for output node not valid!"));
                }
                let clip = clip.unwrap();

                if clip.clip_type != ClipType::Composited {
                    return Err(format!(
                        "Clip identifier for output node not valid (must be composited clip)!"
                    ));
                }
                composited_clip_to_index.insert(clip.id, node_idx);
            }
        }

        // Connect media import nodes of composited clips to the relevant output nodes
        for (id, node) in &store.nodes {
            if node.node_type == media_import_node::IDENTIFIER {
                let clip = node.properties.get(media_import_node::inputs::CLIP);
                if clip.is_none() {
                    return Err(format!("Input node with no clip detected!"));
                }

                let clip = clip.unwrap().to_owned();
                let clip = serde_json::from_value::<ClipIdentifier>(clip);

                if clip.is_err() {
                    return Err(format!("Clip identifier for input node not valid!"));
                }
                let clip = clip.unwrap();

                if clip.clip_type == ClipType::Composited {
                    let composited_clip_idx = composited_clip_to_index.get_by_left(&clip.id);
                    if composited_clip_idx.is_none() {
                        return Err(format!("Reference to composited clip with no output!"));
                    }
                    let composited_clip_idx = composited_clip_idx.unwrap();

                    let node_idx = node_id_to_index.get_by_left(id).unwrap(); // we have already gone through all nodes, so guaranteed to be there
                    graph.add_edge(*composited_clip_idx, *node_idx, None);
                }
            }
        }

        // go through all the links, and add the relevant edges between the nodes.
        for Link { from, to } in &self.links {
            let (from_node_idx, to_node_idx) = (
                node_id_to_index.get_by_left(&from.node_id),
                node_id_to_index.get_by_left(&to.node_id),
            );

            if from_node_idx.is_none() || to_node_idx.is_none() {
                return Err(format!("Link contains reference to non-existent node"));
            }
            let (from_node_idx, to_node_idx) = (from_node_idx.unwrap(), to_node_idx.unwrap());

            graph.add_edge(
                *from_node_idx,
                *to_node_idx,
                Some((from.property.clone(), to.property.clone())),
            );
        }

        Ok((graph, node_id_to_index))
    }

    /**
     * Generates the pipeline; will not generate timeline files if `get_output` = `false`
     * Will utilise the cache when possible
     */
    pub fn generate_pipeline(
        &self,
        store: &Store,
        node_register: &NodeRegister,
        get_output: bool,
        cache: &Cache,
    ) -> Result<
        (
            HashMap<
                Uuid,
                (
                    HashMap<String, PipedType>,
                    HashMap<String, NodeTypeInput>,
                    HashMap<String, NodeTypeOutput>,
                ),
            >,
            HashMap<Uuid, PipedType>,
            bool,
        ),
        String,
    > {
        let (mut graph, node_id_to_index) = self.get_graph(store)?;

        // topologically sort graph
        let sorted = petgraph::algo::toposort(&graph, None);
        // if there's a cycle, it's an invalid pipeline anyway
        if sorted.is_err() {
            return Err(format!("Found cycle in the graph!"));
        }

        let mut node_type_data = HashMap::new();
        let mut composited_clip_data = HashMap::new();

        let sorted = sorted.unwrap();

        let mut do_return = true;

        // we can then iterate through the nodes in this order, assign the piped inputs to the dependent nodes before the dependent nodes' relevant method is called
        for node_idx in sorted {
            let piped_inputs = graph.node_weight(node_idx).unwrap();
            // get the inputs that have been piped into this node; by this point, all nodes with an edge going into this node will have already been processed, and will have put the relevant clip type into this hashmap

            let node = store
                .nodes
                .get(node_id_to_index.get_by_right(&node_idx).unwrap())
                .unwrap();
            let node_registration = node_register.get(&node.node_type).unwrap();
            let io = (node_registration.get_io)(
                node.id.clone(),
                &node.properties,
                &piped_inputs,
                &composited_clip_data,
                store,
                node_register,
            );
            // get the inputs and outputs based off the current set of piped inputs, properties, etc.

            if io.is_err() {
                return Err(format!(
                    "Could not find IO data for node {}: {}",
                    node.id,
                    io.unwrap_err()
                ));
            }
            let (inputs, outputs) = io.unwrap();

            let data = (piped_inputs.clone(), inputs.clone(), outputs.clone());
            // println!("Data for node {}: {:#?}", node.id.clone(), data.clone());
            node_type_data.insert(node.id.clone(), data);

            let pipeline = if get_output {
                (node_registration.get_output)(
                    node.id.clone(),
                    &node.properties,
                    &piped_inputs,
                    &composited_clip_data,
                    store,
                    node_register,
                )
            } else {
                Err(String::from(""))
            };

            if pipeline.is_err() {
                do_return = false;
            } else {
                let pipeline = pipeline.unwrap();

                for (k, v) in pipeline {
                    let out_type = outputs.get(&k).unwrap();

                    let from_piped_type = PipedType {
                        stream_type: out_type.property_type,
                        node_id: node.id.clone(),
                        property_name: k.clone(),
                        io: InputOrOutput::Output,
                        cache_id: None,
                    };

                    let output_location = from_piped_type.get_gst_save_location();

                    v.save_to_uri(output_location.as_str(), None as Option<&ges::Asset>, true)
                        .unwrap();
                    ges::Asset::needs_reload(
                        ges::UriClip::static_type(),
                        Some(output_location.as_str()),
                    );
                }
            }

            if node.node_type == output_node::IDENTIFIER {
                let composited_clip_type = piped_inputs.get(output_node::inputs::MEDIA);

                if composited_clip_type.is_none() {
                    continue;
                }
                let composited_clip_type = composited_clip_type.unwrap().to_owned();

                let composited_clip_id = serde_json::from_value::<ClipIdentifier>(
                    node.properties
                        .get(output_node::inputs::CLIP)
                        .unwrap()
                        .to_owned(),
                )
                .unwrap()
                .id;

                composited_clip_data.insert(composited_clip_id, composited_clip_type);
            } else {
                let graph_clone = graph.clone();
                let edges = graph_clone
                    .edges_directed(node_idx, EdgeDirection::Outgoing)
                    .clone();
                for edge in edges {
                    let (from_property, to_property) = edge.weight().as_ref().unwrap();
                    let target = edge.target();

                    let out_type = outputs.get(from_property).unwrap();
                    let to_node = node_id_to_index.get_by_right(&target).unwrap();

                    let next_node_inputs = graph.node_weight_mut(target).unwrap();

                    let cache_id = if let Some(node_outputs) = cache.get(&node.id) {
                        let output = node_outputs.get(from_property).unwrap();
                        Some(output.clone())
                    } else {
                        None
                    };

                    let from_piped_type = PipedType {
                        stream_type: out_type.property_type,
                        node_id: node.id.clone(),
                        property_name: from_property.clone(),
                        io: InputOrOutput::Output,
                        cache_id,
                    };

                    let to_piped_type = PipedType {
                        stream_type: out_type.property_type,
                        node_id: to_node.clone(),
                        property_name: to_property.clone(),
                        io: InputOrOutput::Input,
                        cache_id,
                    };

                    if do_return {
                        let from_location = from_piped_type.get_save_location();
                        let to_location = to_piped_type.get_save_location();
                        fs::copy(from_location, to_location).unwrap();

                        let to_location = to_piped_type.get_gst_save_location();
                        ges::Asset::needs_reload(
                            ges::UriClip::static_type(),
                            Some(to_location.as_str()),
                        );
                    }

                    next_node_inputs.insert(to_property.clone(), to_piped_type.clone());
                }
            }
        }

        // we should then have populated both the node type hashmap and the composited clip type hashmap.

        let output = (node_type_data, composited_clip_data, do_return);

        return Ok(output);
    }
}
