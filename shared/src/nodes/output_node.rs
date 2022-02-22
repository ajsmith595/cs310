use std::collections::HashMap;

use ges::{
    traits::{LayerExt, TimelineExt},
    TrackType,
};
use glib::StaticType;
use serde_json::Value;

use crate::{
    clip::{ClipIdentifier, CompositedClip},
    node::{NodeType, NodeTypeInput, NodeTypeOutput, PipeableType, PipedType, Type},
    store::Store,
    ID,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "output";
pub mod inputs {
    pub const MEDIA: &str = "media";
    pub const CLIP: &str = "clip";
}
pub mod outputs {}

fn default_properties() -> HashMap<String, NodeTypeInput> {
    let mut default_properties = HashMap::new();

    default_properties.insert(
        String::from(inputs::MEDIA),
        NodeTypeInput {
            name: String::from(inputs::MEDIA),
            display_name: String::from("Media"),
            description: String::from("Media to output to clip"),
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
        String::from(inputs::CLIP),
        NodeTypeInput {
            name: String::from(inputs::CLIP),
            display_name: String::from("Clip"),
            description: String::from("Clip to output"),
            property_type: Type::Clip,
        },
    );
    default_properties
}

fn get_io(
    _node_id: ID,
    _properties: &HashMap<String, Value>,
    _piped_inputs: &HashMap<String, PipedType>,
    _composited_clip_types: &HashMap<ID, PipedType>,
    _store: &Store,
    _node_register: &NodeRegister,
) -> Result<
    (
        HashMap<String, NodeTypeInput>,
        HashMap<String, NodeTypeOutput>,
    ),
    String,
> {
    let inputs = default_properties();
    let outputs = HashMap::new();
    return Ok((inputs, outputs));
}
fn get_output(
    _node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    _composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    _node_register: &NodeRegister,
) -> Result<HashMap<String, ges::Timeline>, String> {
    let media = piped_inputs.get(inputs::MEDIA);
    if media.is_none() {
        return Err(format!("Media is none!"));
    }
    let media = media.unwrap();
    let clip = get_clip(properties, store);
    if clip.is_err() {
        return Err(clip.unwrap_err());
    }
    let clip = clip.unwrap();

    let output_location = clip.get_location();

    let timeline = media.stream_type.create_timeline();
    let clip = ges::UriClipAsset::request_sync(media.get_location().as_str()).unwrap();

    let layer = timeline.append_layer();
    layer
        .add_asset(&clip, None, None, None, TrackType::UNKNOWN)
        .unwrap();

    timeline
        .save_to_uri(output_location.as_str(), None as Option<&ges::Asset>, true)
        .unwrap();

    ges::Asset::needs_reload(ges::UriClip::static_type(), Some(output_location.as_str()));
    Ok(HashMap::new())
}

pub fn output_node() -> NodeType {
    NodeType {
        id: String::from(IDENTIFIER),
        display_name: String::from("Output"),
        description: String::from("Output media to a clip"),
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
        // get_output: |_, properties: &HashMap<String, Value>, store: &Store, _| {
        //   let media = properties.get(inputs::MEDIA).unwrap();
        //   if let Value::String(media) = media {
        //     let clip = get_clip(properties, store);
        //     if clip.is_err() {
        //       return Err(clip.unwrap_err());
        //     }
        //     let clip = clip.unwrap();
        //     return Ok(format!(
        //       "{}. ! videoconvert name={}",
        //       media,
        //       clip.get_gstreamer_id()
        //     ));
        //   }
        //   return Err(format!("Media is invalid type"));
        // },
    }
}

pub fn get_clip(
    properties: &HashMap<String, Value>,
    store: &Store,
) -> Result<CompositedClip, String> {
    let clip = properties.get(inputs::CLIP);
    if clip.is_none() {
        return Err(String::from("No clip given"));
    }
    let clip = clip.unwrap().to_owned();
    let clip = serde_json::from_value::<ClipIdentifier>(clip).unwrap();
    return Ok(store.clips.composited.get(&clip.id).unwrap().clone());
}
