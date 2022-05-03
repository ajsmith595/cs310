use ges::traits::TimelineExt;
use std::{collections::HashMap, hash::Hash};

use serde_json::Value;

use crate::constants::{cache_files_location, intermediate_files_location};

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
    pub properties: HashMap<String, Value>, // A set of key-value pairs - the serde Value allows any type of value to be stored
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
// Specifies numeric restrictions on a property
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

    pub fn get_map(&self) -> HashMap<PipeableStreamType, i32> {
        let mut map = HashMap::new();
        map.insert(PipeableStreamType::Video, self.video);
        map.insert(PipeableStreamType::Audio, self.audio);
        map.insert(PipeableStreamType::Subtitles, self.subtitles);
        map
    }

    pub fn create_timeline(&self) -> ges::Timeline {
        let timeline = ges::Timeline::new();

        for _ in 0..self.video {
            let track = ges::VideoTrack::new();
            timeline.add_track(&track).unwrap();
        }
        for _ in 0..self.audio {
            let track = ges::AudioTrack::new();
            timeline.add_track(&track).unwrap();
        }
        if self.subtitles > 0 {
            panic!("Subtitles have not yet been implemented due to GES ")
        }

        timeline
    }

    pub fn is_singular_type(&self) -> bool {
        let v = self.video > 0;
        let a = self.audio > 0;
        let s = self.subtitles > 0;
        return !((v && a) || (v && s) || (a && s));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InputOrOutput {
    Input,
    Output,
}

impl std::fmt::Display for InputOrOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InputOrOutput::Input => "input",
                InputOrOutput::Output => "output",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipedType {
    pub stream_type: PipeableType,
    pub node_id: ID,
    pub property_name: String,
    pub io: InputOrOutput,
    pub cache_id: Option<ID>,
}

impl PipedType {
    pub fn get_number_of_streams(&self, stream_type: &PipeableStreamType) -> i32 {
        match stream_type {
            PipeableStreamType::Video => self.stream_type.video,
            PipeableStreamType::Audio => self.stream_type.audio,
            PipeableStreamType::Subtitles => self.stream_type.subtitles,
        }
    }

    pub fn get_gst_save_location(&self) -> String {
        format!("file:///{}", self.get_save_location())
    }
    pub fn get_save_location(&self) -> String {
        format!(
            "{}/{}_{}_{}.xges",
            intermediate_files_location(),
            self.node_id,
            self.property_name,
            self.io
        )
        .replace("\\", "/")
    }

    pub fn get_gst_save_location_with_cache(&self) -> String {
        format!("file:///{}", self.get_save_location_with_cache())
    }
    pub fn get_save_location_with_cache(&self) -> String {
        if let Some(cache_id) = self.cache_id {
            format!("{}/{}", cache_files_location(), cache_id).replace("\\", "/")
        } else {
            self.get_save_location()
        }
    }
}
#[derive(PartialEq, Eq, Hash)]
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
}

#[derive(Copy, Serialize, Deserialize, Debug, Clone)]
/// The enum specifying a particular input/output's type
pub enum Type {
    Pipeable(PipeableType, PipeableType),
    Number(Restrictions),
    String(i32),
    Clip,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeInput {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub property_type: Type, // The input type is a restriction, rather than a definitive type. Allows different stream types to be supplied and accepted
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeTypeOutput {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub property_type: PipeableType, // The output type is definitive, unlike the input
}

/// Shorthand for specifying the `NodeType` functions
type NodeTypeFunc<T> = fn(
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<T, String>;

pub enum MemorySafetyWrapper {
    UriClip(ges::UriClip),
    UriClipAsset(ges::UriClipAsset),
    Effect(ges::Effect),
}

#[derive(Serialize, Clone)]
pub struct NodeType {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub default_properties: HashMap<String, NodeTypeInput>,

    #[serde(skip_serializing)] // we cannot serialise functions, so do not try to serialise them
    pub get_io: NodeTypeFunc<(
        HashMap<String, NodeTypeInput>,
        HashMap<String, NodeTypeOutput>,
    )>, // this function returns the inputs, outputs and properties of a node, given its current inputs and properties.

    #[serde(skip_serializing)]
    pub get_output: NodeTypeFunc<(HashMap<String, ges::Timeline>, Vec<MemorySafetyWrapper>)>, // Gets the GES timeline for that particular node
}
