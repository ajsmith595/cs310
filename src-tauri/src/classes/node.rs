use std::{
  collections::{hash_map, HashMap},
  hash::Hash,
};
use uuid::Uuid;

use serde_json::Value;

use super::{global::uniq_id, nodes::NodeRegister, store::Store, ID};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Position {
  pub x: i32,
  pub y: i32,
}
impl Position {
  pub fn new() -> Self {
    Self { x: 0, y: 0 }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
  pub position: Position,
  pub id: ID,
  pub node_type: String,
  pub properties: HashMap<String, Value>, // value from serde_json?
  pub group: ID,
}
impl Node {
  pub fn new(node_type: String, group: Option<ID>) -> Self {
    let group_id;
    if group.is_none() {
      group_id = uniq_id();
    } else {
      group_id = group.unwrap();
    }
    Self {
      position: Position::new(),
      id: uniq_id(),
      node_type,
      properties: HashMap::new(),
      group: group_id,
    }
  }

  pub fn get_gstreamer_handle_id(node_id: String, property: String) -> String {
    format!("{}-{}", node_id, property)
  }
}
#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub struct Restrictions {
  pub min: f64,
  pub max: f64,
  pub step: f64,
  pub default: f64,
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub struct PipeableType {
  pub video: i32,
  pub audio: i32,
  pub subtitles: i32,
}

impl PipeableType {
  pub fn of_type(&self, stream_type: &PipeableStreamType) -> i32 {
    match stream_type {
      &PipeableStreamType::Video => self.video,
      &PipeableStreamType::Audio => self.audio,
      &PipeableStreamType::Subtitles => self.subtitles,
    }
  }

  pub fn min(stream1: &PipeableType, stream2: &PipeableType) -> PipeableType {
    return PipeableType {
      video: std::cmp::min(stream1.video, stream2.video),
      audio: std::cmp::min(stream1.audio, stream2.audio),
      subtitles: std::cmp::min(stream1.subtitles, stream2.subtitles),
    };
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InputOrOutput {
  Input,
  Output,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipedType {
  pub stream_type: PipeableType,
  pub node_id: String,
  pub property_name: String,
  pub io: InputOrOutput,
}

impl PipedType {
  pub fn get_number_of_streams(&self, stream_type: &PipeableStreamType) -> i32 {
    match stream_type {
      PipeableStreamType::Video => self.stream_type.video,
      PipeableStreamType::Audio => self.stream_type.audio,
      PipeableStreamType::Subtitles => self.stream_type.subtitles,
    }
  }
}
#[derive(PartialEq)]
pub enum PipeableStreamType {
  Video,
  Audio,
  Subtitles,
}
impl PipeableStreamType {
  pub fn to_string(&self) -> String {
    match self {
      PipeableStreamType::Video => String::from("video"),
      PipeableStreamType::Audio => String::from("audio"),
      PipeableStreamType::Subtitles => String::from("subtitles"),
    }
  }

  pub fn stream_linker(&self) -> String {
    String::from(match &self {
      &PipeableStreamType::Video => "videoconvert",
      &PipeableStreamType::Audio => "audioconvert",
      &PipeableStreamType::Subtitles => "subparse",
    })
  }
}

impl PipedType {
  pub fn get_gst_handle(&self, stream_type: &PipeableStreamType, index: i32) -> Option<String> {
    let io = match self.io {
      InputOrOutput::Input => "input",
      InputOrOutput::Output => "output",
    };
    let stream_type_str = stream_type.to_string();
    if self.get_number_of_streams(&stream_type) <= index {
      return None;
    }
    return Some(format!(
      "{}-{}-{}-{}-{}",
      self.node_id, io, self.property_name, stream_type_str, index
    ));
  }

  pub fn gst_transfer_pipe(from: PipedType, to: PipedType) -> Option<String> {
    if from.stream_type.video < to.stream_type.video
      || from.stream_type.audio < to.stream_type.audio
      || from.stream_type.subtitles < to.stream_type.subtitles
    {
      return None;
    }
    let video = Self::gst_transfer_pipe_type(&from, &to, &PipeableStreamType::Video);
    let audio = Self::gst_transfer_pipe_type(&from, &to, &PipeableStreamType::Audio);
    let subtitles = Self::gst_transfer_pipe_type(&from, &to, &PipeableStreamType::Subtitles);
    if video.is_none() || audio.is_none() || subtitles.is_none() {
      return None;
    }
    let str = format!(
      "{} {} {}",
      video.unwrap(),
      audio.unwrap(),
      subtitles.unwrap()
    );
    return Some(str);
  }

  pub fn gst_transfer_pipe_type(
    from: &PipedType,
    to: &PipedType,
    stream_type: &PipeableStreamType,
  ) -> Option<String> {
    if from.stream_type.video < to.stream_type.video
      || from.stream_type.audio < to.stream_type.audio
      || from.stream_type.subtitles < to.stream_type.subtitles
    {
      return None;
    }
    let num = to.get_number_of_streams(stream_type);

    let stream_linker = stream_type.stream_linker();
    let mut str = String::from("");
    for i in 0..num {
      let gst1 = from.get_gst_handle(stream_type, i);
      let gst2 = to.get_gst_handle(stream_type, i);
      if gst1.is_none() || gst2.is_none() {
        return None;
      }
      let (gst1, gst2) = (gst1.unwrap(), gst2.unwrap());

      str = format!("{} {}. ! {} name={}", str, gst1, stream_linker, gst2);
    }
    return Some(str);
  }
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
pub enum Type {
  Pipeable(PipeableType, PipeableType),
  Number(Restrictions),
  String(i32), // TODO: make these easier to read + add more properties. e.g. min string length, max string length, regex for valid string, etc.
  // Maybe some restrictions on video min/max duration, resolution, etc?
  Clip,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeInput {
  pub name: String,
  pub display_name: String,
  pub description: String,
  pub property_type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeOutput {
  pub name: String,
  pub display_name: String,
  pub description: String,
  pub property_type: PipeableType,
}

type NodeTypeFunc<T> = fn(
  node_id: String,
  properties: &HashMap<String, Value>,
  piped_inputs: &HashMap<String, PipedType>,
  composited_clip_types: &HashMap<String, PipedType>,
  store: &Store,
  node_register: &NodeRegister,
) -> Result<T, String>;

#[derive(Serialize, Clone)]
pub struct NodeType {
  pub id: String,
  pub display_name: String,
  pub description: String,
  pub default_properties: HashMap<String, NodeTypeInput>,

  #[serde(skip_serializing)]
  pub get_io: NodeTypeFunc<(
    HashMap<String, NodeTypeInput>,
    HashMap<String, NodeTypeOutput>,
  )>,

  #[serde(skip_serializing)]
  pub get_output: NodeTypeFunc<String>,
}

// impl NodeType {
//   pub fn new(
//     id: String,
//     display_name: String,
//     description: String,

//     get_properties: fn(
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<HashMap<String, NodeTypeInput>, String>,

//     get_output_types: fn(
//       node_id: String,
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<HashMap<String, NodeTypeOutput>, String>,
//     get_output: fn(
//       node_id: String,
//       properties: &HashMap<String, Value>,
//       store: &Store,
//       node_register: &NodeRegister,
//     ) -> Result<String, String>,

//     store: &Store,
//   ) -> Self {
//     let default_properties = get_properties(HashMap::new(), store, )
//     Self {
//       id,
//     display_name,
//     description,
//     get_properties,
//     get_output_types,
//     get_output,

//     }
//   }
// }
