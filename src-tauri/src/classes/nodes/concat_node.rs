use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode, AbstractPipeline},
  clip::{ClipIdentifier, ClipType},
  global::uniq_id,
  node::{
    InputOrOutput, Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableStreamType, PipeableType,
    PipedType, Type,
  },
  store::Store,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "concat";
pub mod INPUTS {
  pub const MEDIA1: &str = "media1";
  pub const MEDIA2: &str = "media2";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}

fn default_properties() -> HashMap<String, NodeTypeInput> {
  let mut default_properties = HashMap::new();
  {
    default_properties.insert(
      String::from(INPUTS::MEDIA1),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA1),
        display_name: String::from("Media 1"),
        description: String::from("The first media to play"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 0,
            audio: 0,
            subtitles: 0,
          },
          PipeableType {
            video: i32::MAX,
            audio: i32::MAX,
            subtitles: i32::MAX,
          },
        ),
      },
    );

    default_properties.insert(
      String::from(INPUTS::MEDIA2),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA2),
        display_name: String::from("Media 2"),
        description: String::from("The second media to play"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 0,
            audio: 0,
            subtitles: 0,
          },
          PipeableType {
            video: i32::MAX,
            audio: i32::MAX,
            subtitles: i32::MAX,
          },
        ),
      },
    );
  }

  default_properties
}

pub fn get_io(
  node_id: String,
  properties: &HashMap<String, Value>,
  piped_inputs: &HashMap<String, PipedType>,
  composited_clip_types: &HashMap<String, PipedType>,
  store: &Store,
  node_register: &NodeRegister,
) -> Result<
  (
    HashMap<String, NodeTypeInput>,
    HashMap<String, NodeTypeOutput>,
  ),
  String,
> {
  let mut inputs = default_properties();
  let mut stream_type = PipeableType {
    video: i32::MAX,
    audio: i32::MAX,
    subtitles: i32::MAX,
  };

  let piped_input1 = piped_inputs.get(INPUTS::MEDIA1);
  if let Some(piped_input1) = piped_input1 {
    stream_type = PipeableType::min(&piped_input1.stream_type, &stream_type);
  }
  let piped_input2 = piped_inputs.get(INPUTS::MEDIA2);
  if let Some(piped_input2) = piped_input2 {
    stream_type = PipeableType::min(&piped_input2.stream_type, &stream_type);
  }

  // inputs.get_mut(INPUTS::MEDIA2).unwrap().property_type =
  //   Type::Pipeable(stream_type.clone(), stream_type.clone());

  let mut outputs = HashMap::new();
  outputs.insert(
    OUTPUTS::OUTPUT.to_string(),
    NodeTypeOutput {
      name: OUTPUTS::OUTPUT.to_string(),
      description: "The concatenation of the two media".to_string(),
      display_name: "Output".to_string(),
      property_type: stream_type,
    },
  );

  return Ok((inputs, outputs));
}
fn get_output(
  node_id: String,
  properties: &HashMap<String, Value>,
  piped_inputs: &HashMap<String, PipedType>,
  composited_clip_types: &HashMap<String, PipedType>,
  store: &Store,
  node_register: &NodeRegister,
) -> Result<AbstractPipeline, String> {
  let mut pipeline = AbstractPipeline::new();

  let io = get_io(
    node_id.clone(),
    properties,
    piped_inputs,
    composited_clip_types,
    store,
    node_register,
  );
  if io.is_err() {
    return Err(io.unwrap_err());
  }

  let (inputs, outputs) = io.unwrap();

  let media1 = piped_inputs.get(INPUTS::MEDIA1);
  let media2 = piped_inputs.get(INPUTS::MEDIA2);
  if media1.is_none() || media2.is_none() {
    return Err(format!("No media input!"));
  }
  let media1 = media1.unwrap();
  let media2 = media2.unwrap();

  let output = outputs.get(OUTPUTS::OUTPUT).unwrap();
  let output = PipedType {
    stream_type: output.property_type,
    node_id,
    property_name: String::from(OUTPUTS::OUTPUT),
    io: InputOrOutput::Output,
  };

  for (stream_type, num) in output.stream_type.get_map() {
    for i in 0..num {
      let id = uniq_id();

      let output_gst = output.get_gst_handle(&stream_type, i);
      let media1_gst = media1.get_gst_handle(&stream_type, i);
      let media2_gst = media2.get_gst_handle(&stream_type, i);

      if output_gst.is_none() || media1_gst.is_none() || media2_gst.is_none() {
        return Err(format!("Invalid types to link by"));
      }

      let output_gst = output_gst.unwrap();
      let media1_gst = media1_gst.unwrap();
      let media2_gst = media2_gst.unwrap();
      {
        let concat_node = AbstractNode::new("concat", Some(id.clone()));
        let stream_linker_node =
          AbstractNode::new(&stream_type.stream_linker(), Some(output_gst.clone()));

        pipeline.add_node(concat_node);
        pipeline.add_node(stream_linker_node);

        pipeline.link_abstract(AbstractLink {
          from: AbstractLinkEndpoint::new(id.clone()),
          to: AbstractLinkEndpoint::new(output_gst.clone()),
        });

        let queue_node1 = AbstractNode::new("queue", None);
        let queue_node2 = AbstractNode::new("queue", None);

        pipeline.link_abstract(AbstractLink {
          from: AbstractLinkEndpoint::new(media1_gst.clone()),
          to: AbstractLinkEndpoint::new(queue_node1.id.clone()),
        });

        pipeline.link_abstract(AbstractLink {
          from: AbstractLinkEndpoint::new(queue_node1.id.clone()),
          to: AbstractLinkEndpoint::new(id.clone()),
        });

        pipeline.link_abstract(AbstractLink {
          from: AbstractLinkEndpoint::new(media2_gst.clone()),
          to: AbstractLinkEndpoint::new(queue_node2.id.clone()),
        });

        pipeline.link_abstract(AbstractLink {
          from: AbstractLinkEndpoint::new(queue_node2.id.clone()),
          to: AbstractLinkEndpoint::new(id.clone()),
        });

        pipeline.add_node(queue_node1);
        pipeline.add_node(queue_node2);
      }
    }
  }
  // println!(
  //   "\nPipeline for concat: {}\n",
  //   pipeline.to_gstreamer_pipeline()
  // );
  return Ok(pipeline);
}

pub fn concat_node() -> NodeType {
  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Concatenation"),
    description: String::from("Concatenate two media sources"),
    default_properties: default_properties(),

    get_io: |node_id: String,
             properties: &HashMap<String, Value>,
             piped_inputs: &HashMap<String, PipedType>,
             composited_clip_types: &HashMap<String, PipedType>,
             store: &Store,
             node_register: &NodeRegister| {
      return get_io(
        node_id,
        properties,
        piped_inputs,
        composited_clip_types,
        store,
        node_register,
      );
    },
    get_output: |node_id: String,
                 properties: &HashMap<String, Value>,
                 piped_inputs: &HashMap<String, PipedType>,
                 composited_clip_types: &HashMap<String, PipedType>,
                 store: &Store,
                 node_register: &NodeRegister| {
      return get_output(
        node_id,
        properties,
        piped_inputs,
        composited_clip_types,
        store,
        node_register,
      );
    },
  }
}
