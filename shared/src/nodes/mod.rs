pub mod audio_gain_node;
pub mod blur_node;
pub mod concat_node;
pub mod media_import_node;
pub mod output_node;

use self::{
    audio_gain_node::audio_gain, blur_node::blur_node, concat_node::concat_node,
    output_node::output_node,
};

use super::node::NodeType;
use crate::nodes::media_import_node::media_import_node;
use std::collections::HashMap;

pub type NodeRegister = HashMap<String, NodeType>;

pub fn get_node_register() -> NodeRegister {
    let mut register = HashMap::new();

    register.insert(
        String::from(media_import_node::IDENTIFIER),
        media_import_node(),
    );
    register.insert(String::from(output_node::IDENTIFIER), output_node());
    register.insert(String::from(concat_node::IDENTIFIER), concat_node());
    register.insert(String::from(blur_node::IDENTIFIER), blur_node());
    register.insert(String::from(audio_gain_node::IDENTIFIER), audio_gain());

    register
}
