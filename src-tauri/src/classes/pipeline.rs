use std::collections::HashMap;

use petgraph::{data::Build, graph::DiGraph};

use super::{node::Node, ID};

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
  fn generate_graph(&self) -> DiGraph<String, String> {
    todo!();

    let mut graph = DiGraph::new();

    let mut node_id_to_index = HashMap::new();
    for node in &self.nodes {
      node_id_to_index.insert(node.id.clone(), graph.add_node(node.id.clone()));
      // add nodes for all the nodes' handles aswell.
    }
    for link in &self.links {
      graph.add_edge(
        *node_id_to_index.get(link.from.node_id.as_str()).unwrap(),
        *node_id_to_index.get(link.from.node_id.as_str()).unwrap(),
        link.get_id(),
      );
    }
  }

  pub fn generate_pipeline_string(&self) -> Result<String, String> {
    let mut graph = DiGraph::new();

    let mut node_id_to_index = HashMap::new();
    for node in &self.nodes {
      node_id_to_index.insert(node.id.clone(), graph.add_node(node.id.clone()));
      // add nodes for all the nodes' handles aswell.
    }
    for link in &self.links {
      graph.add_edge(
        *node_id_to_index.get(link.from.node_id.as_str()).unwrap(),
        *node_id_to_index.get(link.from.node_id.as_str()).unwrap(),
        link.get_id(),
      );
    }
    if petgraph::algo::is_cyclic_directed(&graph) {
      return Err(String::from("Cycle in pipeline"));
    }

    Ok(String::from("test"))
    // step 1: generate graph from nodes + links
    // step 2: check for cycles: if there's cycles, error
    // step 3: use the graph to find any dependencies of the target_node_id, and generate the pipeline only including those nodes.
  }

  pub fn get_output_type(&self, output_clip_id: ID) {}
}
