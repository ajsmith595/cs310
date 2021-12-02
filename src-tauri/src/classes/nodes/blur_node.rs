use std::collections::HashMap;

use serde_json::Value;

use crate::classes::{
  clip::{ClipIdentifier, ClipType},
  node::{
    InputOrOutput, Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableStreamType, PipeableType,
    PipedType, Restrictions, Type,
  },
  store::Store,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "blur";
pub mod INPUTS {
  pub const MEDIA: &str = "media";
  pub const SIGMA: &str = "sigma";
}
pub mod OUTPUTS {
  pub const OUTPUT: &str = "output";
}

fn default_properties() -> HashMap<String, NodeTypeInput> {
  let mut default_properties = HashMap::new();
  {
    default_properties.insert(
      String::from(INPUTS::MEDIA),
      NodeTypeInput {
        name: String::from(INPUTS::MEDIA),
        display_name: String::from("Media"),
        description: String::from("The media to be blurred"),
        property_type: Type::Pipeable(
          PipeableType {
            video: 1,
            audio: 0,
            subtitles: 0,
          },
          PipeableType {
            video: 1,
            audio: i32::MAX,
            subtitles: i32::MAX,
          },
        ),
      },
    );

    default_properties.insert(
      String::from(INPUTS::SIGMA),
      NodeTypeInput {
        name: String::from(INPUTS::SIGMA),
        display_name: String::from("Blur Amount"),
        description: String::from(
          "The sigma value for the blur; the higher the value, the more the media is blurred",
        ),
        property_type: Type::Number(Restrictions {
          min: (0.0 as f64),
          max: (100.0 as f64),
          step: (0.01 as f64),
          default: (1.2 as f64),
        }),
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
  let mut outputs = HashMap::new();
  let mut stream_type = PipeableType {
    video: i32::MAX,
    audio: i32::MAX,
    subtitles: i32::MAX,
  };
  let piped_input = piped_inputs.get(INPUTS::MEDIA);

  if let Some(piped_input) = piped_input {
    stream_type = piped_input.stream_type;
  }
  outputs.insert(
    OUTPUTS::OUTPUT.to_string(),
    NodeTypeOutput {
      name: OUTPUTS::OUTPUT.to_string(),
      description: "The blurred media".to_string(),
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

  let media = piped_inputs.get(INPUTS::MEDIA);
  if media.is_none() {
    return Err(format!("No media input!"));
  }
  let media = media.unwrap();
  let sigma = properties.get(INPUTS::SIGMA).unwrap();
  if let Value::Number(sigma) = sigma {
    let gst_string = String::from("");

    let output = outputs.get(OUTPUTS::OUTPUT).unwrap();
    let output = PipedType {
      stream_type: output.property_type,
      node_id,
      property_name: String::from(OUTPUTS::OUTPUT),
      io: InputOrOutput::Output,
    };

    let audio_passthrough =
      PipedType::gst_transfer_pipe_type(&media, &output, &PipeableStreamType::Audio);
    let subtitle_passthrough =
      PipedType::gst_transfer_pipe_type(&media, &output, &PipeableStreamType::Subtitles);

    if audio_passthrough.is_none() || subtitle_passthrough.is_none() {
      return Err(format!("Could not get video/subtitle passthrough"));
    }
    let (audio_passthrough, subtitle_passthrough) =
      (audio_passthrough.unwrap(), subtitle_passthrough.unwrap());

    let mut gst_string = format!("{} {} ", audio_passthrough, subtitle_passthrough);

    for i in 0..output.stream_type.video {
      gst_string = format!(
        "{} {}. ! gaussianblur sigma={} ! {}.",
        gst_string,
        media.get_gst_handle(&PipeableStreamType::Video, i).unwrap(),
        sigma,
        output
          .get_gst_handle(&PipeableStreamType::Video, i)
          .unwrap()
      );
    }

    return Ok(gst_string);
  }
  return Err(format!(
    "Media is invalid type (gaussian blur): \n{:#?}\n\n",
    properties
  ));
}

pub fn blur_node() -> NodeType {
  NodeType {
    id: String::from(IDENTIFIER),
    display_name: String::from("Blur"),
    description: String::from("Blur a media source"),
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
