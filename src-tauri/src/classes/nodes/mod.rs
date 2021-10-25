pub mod media_import_node;
pub mod output_node;
use self::output_node::output_node;

use super::node::NodeType;
use crate::classes::nodes::media_import_node::media_import_node;
use std::{collections::HashMap, fmt::Display};

pub fn get_node_register() -> HashMap<String, NodeType> {
  let mut register = HashMap::new();

  register.insert(
    String::from(media_import_node::IDENTIFIER),
    media_import_node(),
  );
  register.insert(String::from(output_node::IDENTIFIER), output_node());

  register
}
