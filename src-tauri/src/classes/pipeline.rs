use std::collections::HashMap;

use petgraph::{data::Build, graph::DiGraph, Graph};

use crate::classes::node::Type;

use super::{node::Node, store::Store, ID};

pub struct LinkEndpoint {
  pub node_id: ID,
  pub property: String,
}
impl LinkEndpoint {
  pub fn get_id(&self) -> String {
    return String::from(self.node_id.clone() + "." + &self.property);
  }
}
pub struct Link {
  pub from: LinkEndpoint,
  pub to: LinkEndpoint,
}
impl Link {
  pub fn get_id(&self) -> String {
    return String::from(self.from.get_id() + "-" + &self.to.get_id());
  }
}

pub struct Pipeline {
  pub nodes: Vec<Node>,
  pub links: Vec<Link>,
  pub target_node_id: ID,
}

impl Pipeline {
  fn generate_graph(&self, store: &Store) -> Result<Graph<String, String>, String> {
    let mut graph = DiGraph::new();

    let mut node_id_to_index = HashMap::new();
    for node in &self.nodes {
      let node_index = graph.add_node(node.id.clone());
      node_id_to_index.insert(node.id.clone(), node_index);
      let node_type = &store.node_types.get(&node.node_type);
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
      let outputs = (node_type.get_output_types)(&node.properties, &store);
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
      let from = node_id_to_index.get(&link.from.get_id());
      let to = node_id_to_index.get(&link.to.get_id());
      if from.is_none() || to.is_none() {
        return Err(String::from(
          "Link is invalid - connects non-existent handles!",
        ));
      }
      let from = from.unwrap();
      let to = to.unwrap();
      graph.add_edge(*from, *to, link.get_id());
    }
    Ok(graph)
  }

  pub fn generate_pipeline_string(&self, store: &Store) -> Result<String, String> {
    let graph = self.generate_graph(store);
    if graph.is_err() {
      return Err(String::from("Graph could not be generated: ") + graph.unwrap_err().as_str());
    }
    let graph = graph.unwrap();
    if petgraph::algo::is_cyclic_directed(&graph) {
      return Err(String::from("Cycle in pipeline"));
    }

    Ok(String::from("test"))
    // step 1: generate graph from nodes + links
    // step 2: check for cycles: if there's cycles, error
    // step 3: use the graph to find any dependencies of the target_node_id, and generate the pipeline only including those nodes.
  }

  pub fn get_output_type(&self, output_clip_id: ID) -> Type {
    // 1. look at the nodes, find all the output nodes.
    // 2. find the specific output node (if exists) for the relevant clip ID
    // 3. recurse until done: look at the previous node, and determine its output.

    todo!();
  }
}
