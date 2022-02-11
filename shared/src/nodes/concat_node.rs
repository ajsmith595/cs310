use std::collections::HashMap;

use ges::{
    traits::{LayerExt, TimelineExt},
    TrackType,
};
use glib::StaticType;
use serde_json::Value;

use crate::{
    abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode, AbstractPipeline},
    clip::{ClipIdentifier, ClipType},
    global::uniq_id,
    node::{
        InputOrOutput, Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableStreamType,
        PipeableType, PipedType, Type,
    },
    store::Store,
    ID,
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
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
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
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<HashMap<String, ges::Timeline>, String> {
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

    let timeline = output.stream_type.create_timeline();

    let layer = timeline.append_layer();
    let clip1 = ges::UriClipAsset::request_sync(media1.get_location().as_str()).unwrap();
    let clip2 = ges::UriClipAsset::request_sync(media2.get_location().as_str()).unwrap();

    layer
        .add_asset(&clip1, None, None, None, TrackType::UNKNOWN)
        .unwrap();
    layer
        .add_asset(&clip2, None, None, None, TrackType::UNKNOWN)
        .unwrap();

    let mut hm = HashMap::new();
    hm.insert(OUTPUTS::OUTPUT.to_string(), timeline);
    return Ok(hm);
}

pub fn concat_node() -> NodeType {
    NodeType {
        id: String::from(IDENTIFIER),
        display_name: String::from("Concatenation"),
        description: String::from("Concatenate two media sources"),
        default_properties: default_properties(),

        get_io: |node_id: ID,
                 properties: &HashMap<String, Value>,
                 piped_inputs: &HashMap<String, PipedType>,
                 composited_clip_types: &HashMap<ID, PipedType>,
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
        get_output: |node_id: ID,
                     properties: &HashMap<String, Value>,
                     piped_inputs: &HashMap<String, PipedType>,
                     composited_clip_types: &HashMap<ID, PipedType>,
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
