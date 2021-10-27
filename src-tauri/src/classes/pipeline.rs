use std::{
  borrow::{Borrow, BorrowMut},
  collections::HashMap,
  hash::Hash,
};

use petgraph::{
  data::Build,
  graph::{DiGraph, NodeIndex},
  Graph,
};

use bimap::BiMap;
use serde_json::Value;

use crate::classes::{
  clip::ClipIdentifier,
  node::Type,
  nodes::{self, NodeRegister},
};

use super::{node::Node, store::Store, ID};

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

impl Pipeline {
  pub fn new() -> Self {
    Self {
      links: Vec::new(),
      target_node_id: None,
    }
  }

  fn generate_graph(
    &self,
    store: &Store,
    node_register: &NodeRegister,
  ) -> Result<(Graph<String, String>, BiMap<String, NodeIndex>), String> {
    let mut graph = DiGraph::new();

    let mut node_id_to_index = BiMap::new();
    for (_, node) in &store.nodes {
      let node_index = graph.add_node(node.id.clone());
      node_id_to_index.insert(node.id.clone(), node_index);
      let node_type = &node_register.get(&node.node_type);
      if node_type.is_none() {
        return Err(String::from("Node type not found!"));
      }
      let node_type = node_type.unwrap();
      for (k, v) in &node_type.properties {
        let primary_output = &v.property_type[0];
        match primary_output {
          Type::Pipeable(_) => {
            let le = LinkEndpoint {
              node_id: node.id.clone(),
              property: k.clone(),
            };
            let handle_index = graph.add_node(le.get_id());
            graph.add_edge(
              handle_index,
              node_index,
              le.get_id() + "-" + node.id.as_str(),
            );
            node_id_to_index.insert(le.get_id(), handle_index);
          }
          _ => {}
        }
      }
      let outputs =
        (node_type.get_output_types)(node.id.clone(), &node.properties, &store, node_register);
      if outputs.is_err() {
        return Err(String::from("Getting output type caused error!"));
      }
      let outputs = outputs.unwrap();
      for (k, _) in &outputs {
        let le = LinkEndpoint {
          node_id: node.id.clone(),
          property: k.clone(),
        };
        let handle_index = graph.add_node(le.get_id());
        graph.add_edge(
          node_index,
          handle_index,
          node.id.clone() + "-" + le.get_id().as_str(),
        );
        node_id_to_index.insert(le.get_id(), handle_index);
      }
      // add nodes for all the nodes' handles aswell.
    }
    for link in &self.links {
      let from = node_id_to_index.get_by_left(&link.from.get_id());
      let to = node_id_to_index.get_by_left(&link.to.get_id());
      if from.is_none() || to.is_none() {
        return Err(String::from(
          "Link is invalid - connects non-existent handles!",
        ));
      }
      let from = from.unwrap();
      let to = to.unwrap();
      graph.add_edge(*from, *to, link.get_id());
    }
    Ok((graph, node_id_to_index))
  }

  pub fn generate_pipeline_string(
    &self,
    store: &Store,
    node_register: &NodeRegister,
  ) -> Result<String, String> {
    if self.target_node_id.is_none() {
      return Err(String::from("No target node chosen"));
    }
    let res = self.generate_graph(store, node_register);
    if res.is_err() {
      return Err(String::from("Graph could not be generated: ") + res.unwrap_err().as_str());
    }
    let (graph, node_id_to_index) = res.unwrap();
    if petgraph::algo::is_cyclic_directed(&graph) {
      return Err(String::from("Cycle in pipeline"));
    }

    let mut new_nodes = store.nodes.clone();
    for Link { from, to } in &self.links {
      let mut to_node = new_nodes.get(&to.node_id.clone()).unwrap().to_owned();
      let from_node = new_nodes.get(&from.node_id.clone()).unwrap().to_owned();
      to_node.properties.insert(
        to.property.clone(),
        Value::String(Node::get_gstreamer_handle_id(
          from_node.id.clone(),
          from.property.clone(),
        )),
      );
      new_nodes.insert(to_node.id.clone(), to_node.clone());
      new_nodes.insert(from_node.id.clone(), from_node);
    }

    let target_node_id = self.target_node_id.as_ref().unwrap();
    let target_idx = node_id_to_index.get_by_left(target_node_id);
    if target_idx.is_none() {
      return Err(String::from("Target can't be found in pipeline"));
    }
    let target_idx = target_idx.unwrap();
    let mut store = store.clone();
    store.nodes = new_nodes;

    let out_str = Self::get_node_output_string(
      &graph,
      &store,
      node_register,
      &node_id_to_index,
      *target_idx,
    );
    if out_str.is_err() {
      return Err(String::from("Could not get output"));
    }
    let out_str = out_str.unwrap();

    let mut clip_str = String::new();
    for (_, clip) in store.clips.composited {
      clip_str = format!(
        "{} {}. ! nvh264enc ! h264parse ! mp4mux ! filesink location=\"output/composited/{}.mp4\"",
        clip_str,
        clip.get_gstreamer_id(),
        clip.get_gstreamer_id(),
      );
    }
    let out_str = format!("{} {}", out_str, clip_str);
    return Ok(out_str);
  }

  fn get_node_output_string(
    graph: &Graph<String, String>,
    store: &Store,
    node_register: &NodeRegister,
    node_id_to_index: &BiMap<String, NodeIndex>,
    node_index: NodeIndex,
  ) -> Result<String, String> {
    let mut str = String::from("");
    let target_input_handles =
      graph.neighbors_directed(node_index, petgraph::EdgeDirection::Incoming);

    // let dependents = Vec::new();
    println!("Getting output for node index: {:?}", node_index);
    for input in target_input_handles {
      println!("Neighbor: {:?}", input);
      let output = graph
        .neighbors_directed(input, petgraph::EdgeDirection::Incoming)
        .next();
      if output.is_none() {
        panic!("Graph not generated properly!");
      }
      let output = output.unwrap();
      let node = graph
        .neighbors_directed(output, petgraph::EdgeDirection::Incoming)
        .next();
      if node.is_none() {
        panic!("Graph not generated properly!");
      }
      let node = node.unwrap();
      let node_string =
        Self::get_node_output_string(graph, store, node_register, node_id_to_index, node);
      if node_string.is_err() {
        return Err(String::from("An error occured in a dependent node"));
      }
      let node_string = node_string.unwrap();
      str = format!("{} {}", str, node_string);
    }
    let node_id = node_id_to_index.get_by_right(&node_index).unwrap();
    let node = store.nodes.get(node_id).unwrap();
    let node_type = node_register.get(&node.node_type).unwrap();
    let out =
      (node_type.get_output)(node.id.clone(), &node.properties, store, node_register).unwrap();
    str = format!("{} {}", str, out);
    return Ok(str);
  }

  pub fn get_output_type(
    &self,
    output_clip_id: ID,
    store: &Store,
    node_register: &NodeRegister,
  ) -> Result<Type, String> {
    // 1. look at the nodes, find all the output nodes.
    // 2. find the specific output node (if exists) for the relevant clip ID
    // 3. recurse until done: look at the previous node, and determine its output.
    let mut node_id = None;
    for (_, node) in &store.nodes {
      if node.node_type == nodes::output_node::IDENTIFIER {
        let clip = node.properties.get(nodes::output_node::INPUTS::CLIP);
        if let Some(clip) = clip {
          let clip = serde_json::from_value::<ClipIdentifier>(clip.to_owned());
          if let Ok(clip) = clip {
            if output_clip_id == clip.id {
              node_id = Some(node.id.as_str());
              break;
            }
          }
        }
      }
    }

    if node_id.is_none() {
      return Err(String::from("Clip output node not found"));
    }
    let node_id = node_id.unwrap();
    let endpoint = self.get_connecting_endpoint(LinkEndpoint {
      node_id: String::from(node_id),
      property: String::from(nodes::output_node::INPUTS::MEDIA),
    });
    if endpoint.is_none() {
      return Err(String::from("No endpoint connecting to output node"));
    }
    let LinkEndpoint { node_id, property } = endpoint.unwrap();
    let node = store.nodes.get(&node_id);
    if node.is_none() {
      return Err(String::from("Link is invalid!"));
    }
    let node = node.unwrap();
    let node_type = node_register.get(&node.node_type).unwrap();
    let outputs =
      (node_type.get_output_types)(node.id.clone(), &node.properties, &store, node_register);
    if outputs.is_err() {
      return Err(String::from(
        "Could not get output type of node before output node",
      ));
    }
    let outputs = outputs.unwrap();
    let output_type = outputs.get(&property).unwrap();
    return Ok(output_type.property_type[0]);
  }
  fn get_connecting_endpoint(&self, input_link: LinkEndpoint) -> Option<LinkEndpoint> {
    for Link { from, to } in &self.links {
      if *to == input_link {
        return Some(from.clone());
      }
    }
    return None;
  }
}
