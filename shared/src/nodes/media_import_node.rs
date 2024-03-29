use std::collections::HashMap;

use ges::traits::{LayerExt, TimelineExt};
use glib::StaticType;
use serde_json::Value;

use crate::{
    clip::{ClipIdentifier, ClipType},
    node::{
        self, MemorySafetyWrapper, NodeType, NodeTypeInput, NodeTypeOutput, PipeableType,
        PipedType, Type,
    },
    nodes::NodeRegister,
    store::Store,
    ID,
};

pub const IDENTIFIER: &str = "clip_import";
pub mod inputs {
    pub const CLIP: &str = "clip";
}
pub mod outputs {
    pub const OUTPUT: &str = "output";
}

fn default_properties() -> HashMap<String, NodeTypeInput> {
    let mut default_properties = HashMap::new();

    default_properties.insert(
        String::from(inputs::CLIP),
        NodeTypeInput {
            name: String::from("clip"),
            display_name: String::from("Clip"),
            description: String::from("Clip to import"),
            property_type: Type::Clip,
        },
    );
    default_properties
}

fn get_io(
    _node_id: ID,
    properties: &HashMap<String, Value>,
    _piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    _node_register: &NodeRegister,
) -> Result<
    (
        HashMap<String, NodeTypeInput>,
        HashMap<String, NodeTypeOutput>,
    ),
    String,
> {
    let inputs = default_properties();

    let clip = properties.get(inputs::CLIP);
    if clip.is_none() {
        let mut hm = HashMap::new();
        hm.insert(
            String::from(outputs::OUTPUT),
            NodeTypeOutput {
                name: String::from(outputs::OUTPUT),
                display_name: String::from("Output"),
                description: String::from("The clip itself"),
                property_type: PipeableType {
                    video: i32::MAX,
                    audio: i32::MAX,
                    subtitles: i32::MAX,
                },
            },
        );
        return Ok((inputs, hm));
    }
    let clip = clip.unwrap().to_owned();
    let clip = serde_json::from_value::<ClipIdentifier>(clip);
    if clip.is_err() {
        return Err(String::from("Clip identifier is malformed"));
    }
    let clip = clip.unwrap();
    let property_type;
    match clip.clip_type {
        ClipType::Source => {
            // If it's a source clip, we get the relevant source clip from the store, and we get its clip type directly (by looking at the file)
            let source_clip = store.clips.source.get(&clip.id);
            if source_clip.is_none() {
                return Err(String::from("Clip ID is invalid"));
            }
            let source_clip = source_clip.unwrap();
            property_type = source_clip.get_clip_type();
        }
        ClipType::Composited => {
            let composited_clip_type = composited_clip_types.get(&clip.id);
            if composited_clip_type.is_none() {
                return Err(String::from("composited Clip type is invalid"));
            }
            let composited_clip_type = composited_clip_type.unwrap();

            property_type = composited_clip_type.stream_type;
        }
    }
    let mut hm = HashMap::new();
    hm.insert(
        String::from(outputs::OUTPUT),
        NodeTypeOutput {
            name: String::from(outputs::OUTPUT),
            display_name: String::from("Output"),
            description: String::from("The clip itself"),
            property_type: property_type,
        },
    );
    return Ok((inputs, hm));
}
fn get_output(
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<(HashMap<String, ges::Timeline>, Vec<MemorySafetyWrapper>), String> {
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

    let (_, outputs) = io.unwrap();

    let clip_identifier = get_clip_identifier(properties);
    if clip_identifier.is_err() {
        return Err(clip_identifier.unwrap_err());
    }
    let clip_identifier = clip_identifier.unwrap();

    let output = outputs.get(outputs::OUTPUT).unwrap();
    let output = PipedType {
        node_id: node_id.clone(),
        io: node::InputOrOutput::Output,
        stream_type: output.property_type,
        property_name: outputs::OUTPUT.to_string(),
        cache_id: None,
    };

    let (timeline, mem_safety) = match clip_identifier.clip_type {
        ClipType::Source => {
            let clip = store.clips.source.get(&clip_identifier.id).unwrap();

            let timeline = output.stream_type.create_timeline();
            let layer = timeline.append_layer();

            let location = clip.get_server_url();
            ges::Asset::needs_reload(ges::UriClip::static_type(), Some(location.as_str()));
            let clip = ges::UriClipAsset::request_sync(location.as_str()).unwrap();
            layer
                .add_asset(&clip, None, None, None, ges::TrackType::UNKNOWN)
                .unwrap();
            (timeline, vec![MemorySafetyWrapper::UriClipAsset(clip)])
        }
        ClipType::Composited => {
            let clip = store.clips.composited.get(&clip_identifier.id).unwrap();

            let timeline = output.stream_type.create_timeline();
            let layer = timeline.append_layer();

            let location = clip.get_location();
            ges::Asset::needs_reload(ges::UriClip::static_type(), Some(location.as_str()));
            let clip = ges::UriClipAsset::request_sync(location.as_str()).unwrap();
            layer
                .add_asset(&clip, None, None, None, ges::TrackType::UNKNOWN)
                .unwrap();
            (timeline, vec![MemorySafetyWrapper::UriClipAsset(clip)])
        }
    };

    let mut hm = HashMap::new();
    hm.insert(outputs::OUTPUT.to_string(), timeline);
    Ok((hm, mem_safety))
}

pub fn media_import_node() -> NodeType {
    NodeType {
        id: String::from(IDENTIFIER),
        display_name: String::from("Clip Import"),
        description: String::from("Import a source or composited clip"),
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

pub fn get_clip_identifier(properties: &HashMap<String, Value>) -> Result<ClipIdentifier, String> {
    let clip = properties.get(inputs::CLIP);
    if clip.is_none() {
        return Err(String::from("No clip given"));
    }
    let clip = clip.unwrap().to_owned();
    let clip = serde_json::from_value::<ClipIdentifier>(clip);
    if clip.is_err() {
        return Err(String::from("Clip identifier is malformed"));
    }
    let clip = clip.unwrap();

    Ok(clip)
}
