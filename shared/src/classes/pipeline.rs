use std::{
    borrow::{Borrow, BorrowMut},
    collections::HashMap,
    hash::Hash,
    sync::mpsc,
};

use gstreamer::{glib, prelude::*};
use gstreamer_pbutils::{Discoverer, DiscovererInfo, DiscovererResult, DiscovererStreamInfo};
use petgraph::visit::EdgeRef;
use petgraph::{
    data::Build,
    graph::{DiGraph, NodeIndex},
    EdgeDirection, Graph,
};

use bimap::BiMap;
use serde_json::Value;

use crate::classes::{
    clip::{ClipIdentifier, ClipType},
    node::{InputOrOutput, PipedType, Type},
    nodes::{self, NodeRegister},
};

use super::{
    abstract_pipeline::AbstractPipeline,
    node::{Node, NodeTypeInput, NodeTypeOutput, PipeableType},
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
        return String::from(self.node_id.clone() + "." + &self.property);
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
    pub target_node_id: Option<ID>,
}

pub struct NodeData {
    pub node: Node,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            target_node_id: None,
        }
    }

    pub fn gen_graph_new(
        &self,
        store: &Store,
        node_register: &NodeRegister,
    ) -> Result<
        (
            HashMap<
                String,
                (
                    HashMap<String, PipedType>,
                    HashMap<String, NodeTypeInput>,
                    HashMap<String, NodeTypeOutput>,
                ),
            >,
            HashMap<String, PipedType>,
            Option<AbstractPipeline>,
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
                let clip = node.properties.get(output_node::INPUTS::CLIP);
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
                let clip = node.properties.get(media_import_node::INPUTS::CLIP);
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

        // topologically sort graph
        let sorted = petgraph::algo::toposort(&graph, None);
        // if there's a cycle, it's an invalid pipeline anyway
        if sorted.is_err() {
            return Err(format!("Found cycle in the graph!"));
        }

        let mut node_type_data = HashMap::new();
        let mut composited_clip_data = HashMap::new();

        let sorted = sorted.unwrap();

        let mut abstract_pipeline = AbstractPipeline::new();
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

            let pipeline = (node_registration.get_output)(
                node.id.clone(),
                &node.properties,
                &piped_inputs,
                &composited_clip_data,
                store,
                node_register,
            );
            if pipeline.is_err() {
                println!("get_output failed: {}", pipeline.unwrap_err());
                do_return = false;
            } else {
                let pipeline = pipeline.unwrap();
                abstract_pipeline.merge(pipeline);
            }

            if node.node_type == output_node::IDENTIFIER {
                let composited_clip_type = piped_inputs.get(output_node::INPUTS::MEDIA);

                if composited_clip_type.is_none() {
                    continue;
                }
                let composited_clip_type = composited_clip_type.unwrap().to_owned();

                let composited_clip_id = serde_json::from_value::<ClipIdentifier>(
                    node.properties
                        .get(output_node::INPUTS::CLIP)
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

                    let from_piped_type = PipedType {
                        stream_type: out_type.property_type,
                        node_id: node.id.clone(),
                        property_name: from_property.clone(),
                        io: InputOrOutput::Output,
                    };
                    let to_piped_type = PipedType {
                        stream_type: out_type.property_type,
                        node_id: to_node.clone(),
                        property_name: to_property.clone(),
                        io: InputOrOutput::Input,
                    };
                    next_node_inputs.insert(to_property.clone(), to_piped_type.clone());

                    let output =
                        PipedType::gst_transfer_pipe(from_piped_type, to_piped_type).unwrap();

                    abstract_pipeline.merge(output);
                }
            }
        }

        // we should then have populated both the node type hashmap and the composited clip type hashmap.
        // we can also perform actual obtaining of the output here

        // TODO: check piped inputs meet minimum requirements for the inputs generated
        // TODO: check if a piped input does not correspond to an input, then we need to delete the link from the store, since it's invalid now

        let mut abstract_pipeline = Some(abstract_pipeline);
        if !do_return {
            abstract_pipeline = None;
        }
        let output = (node_type_data, composited_clip_data, abstract_pipeline);

        return Ok(output);
    }

    pub fn execute_pipeline(
        pipeline: String,
        timeout: u32,
        composited_clip_callback: Option<Box<dyn Fn(String, u32) + Send>>,
    ) -> Result<(), ()> {
        let main_loop = glib::MainLoop::new(None, false);

        println!("Pipeline: {}", pipeline);
        // This creates a pipeline by parsing the gst-launch pipeline syntax.

        let pipeline = gstreamer::parse_launch(pipeline.as_str()).unwrap();
        let pipeline = pipeline.dynamic_cast::<gstreamer::Pipeline>().unwrap();

        let bus = pipeline.bus().unwrap();

        let res = pipeline.set_state(gstreamer::State::Playing);
        if res.is_err() {
            println!("Error! {:?}", res.unwrap_err());
            return Err(());
        }

        let pipeline_weak = pipeline.downgrade();

        let main_loop_clone = main_loop.clone();
        let (tx, rx) = mpsc::channel();

        let tx_clone = tx.clone();
        glib::timeout_add_seconds(timeout, move || {
            let pipeline = match pipeline_weak.upgrade() {
                Some(pipeline) => pipeline,
                None => return glib::Continue(false),
            };
            let main_loop = &main_loop_clone;
            println!("sending eos");
            main_loop.quit();

            tx_clone.send(Ok(()));
            glib::Continue(false)
        });

        let main_loop_clone = main_loop.clone();
        bus.add_watch(move |_, msg| {
            use gstreamer::MessageView;

            let main_loop = &main_loop_clone;
            match msg.view() {
                MessageView::Eos(..) => {
                    tx.send(Ok(()));
                }
                MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    main_loop.quit();
                    tx.send(Err(()));
                }
                MessageView::Element(_) => {
                    if let Some(callback) = &composited_clip_callback {
                        let src = msg.src();
                        let structure = msg.structure();

                        if let (Some(src), Some(structure)) = (src, structure) {
                            let event = structure.name().to_string();

                            if event == String::from("splitmuxsink-fragment-closed") {
                                let location = structure.get::<String>("location");
                                let running_time = structure.get::<u64>("running-time");

                                if let (Ok(location), Ok(running_time)) = (location, running_time) {
                                    let node_id = src.name().to_string();
                                    let mut segment = (running_time / 10000000000 - 1) as u32;
                                    if running_time % 10000000000 != 0 {
                                        segment = segment + 1; // deal with final case; the division will be automatically rounded down; this will mean we'll get the first and last segment being the same number, which is wrong.
                                    }
                                    callback(node_id, segment);
                                }
                            }
                        }
                    }
                }
                _ => (),
            };

            glib::Continue(true)
        })
        .expect("Failed to add bus watch");

        main_loop.run();

        pipeline
            .set_state(gstreamer::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");

        bus.remove_watch().unwrap();

        let res = rx.recv();
        if res.is_ok() {
            if res.unwrap().is_ok() {
                return Ok(());
            }
        }
        return Err(());
    }

    pub fn get_video_thumbnail(path: String, id: String) {
        let path = path.replace("\\", "/");
        let pipeline = format!("filesrc location=\"{}\" ! decodebin ! jpegenc snapshot=TRUE ! filesink location=\"thumbnails/source/{}.jpg\"", path, id);
        Self::execute_pipeline(pipeline, 10, None).unwrap();
    }
}
