mod media_import_node;
mod output_node;
use super::node::NodeType;
use crate::classes::nodes::media_import_node::media_import_node;
use std::collections::HashMap;

pub fn get_node_register() -> HashMap<String, NodeType> {
  let mut register = HashMap::new();

  register.insert(String::from("x"), media_import_node());

  register
}
