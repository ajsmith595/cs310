use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
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
  let inputs = default_properties();
  let mut stream_type = PipeableType {
    video: i32::MAX,
    audio: i32::MAX,
    subtitles: i32::MAX,
  };

  let piped_input1 = piped_inputs.get(INPUTS::MEDIA1);
  if let Some(piped_input1) = piped_input1 {
    stream_type = piped_input1.stream_type;
  }

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
) -> Result<String, String> {
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

  let mut gst_string = String::from("");

  for i in 0..output.stream_type.video {
    let id = uniq_id();

    let output_gst = output.get_gst_handle(&PipeableStreamType::Video, i);
    let media1_gst = media1.get_gst_handle(&PipeableStreamType::Video, i);
    let media2_gst = media2.get_gst_handle(&PipeableStreamType::Video, i);

    if output_gst.is_none() || media1_gst.is_none() || media2_gst.is_none() {
      return Err(format!("Invalid types to link by"));
    }
    gst_string = format!(
      "{} concat name={} ! {}. {}. ! {}. {}. ! {}.",
      gst_string,
      id,
      output_gst.unwrap(),
      media1_gst.unwrap(),
      id,
      media2_gst.unwrap(),
      id,
    );
  }

  for i in 0..output.stream_type.audio {
    let id = uniq_id();
    gst_string = format!(
      "{} concat name={} ! {}. {}. ! {}. {}. ! {}.",
      gst_string,
      id,
      output
        .get_gst_handle(&PipeableStreamType::Audio, i)
        .unwrap(),
      media1
        .get_gst_handle(&PipeableStreamType::Audio, i)
        .unwrap(),
      id,
      media2
        .get_gst_handle(&PipeableStreamType::Audio, i)
        .unwrap(),
      id,
    );
  }

  for i in 0..output.stream_type.subtitles {
    let id = uniq_id();
    gst_string = format!(
      "{} concat name={} ! {}. {}. ! {}. {}. ! {}.",
      gst_string,
      id,
      output
        .get_gst_handle(&PipeableStreamType::Subtitles, i)
        .unwrap(),
      media1
        .get_gst_handle(&PipeableStreamType::Subtitles, i)
        .unwrap(),
      id,
      media2
        .get_gst_handle(&PipeableStreamType::Subtitles, i)
        .unwrap(),
      id,
    );
  }

  return Ok(gst_string);
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
