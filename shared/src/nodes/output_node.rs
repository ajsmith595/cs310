use std::collections::HashMap;

use serde_json::Value;

use crate::{
    abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode, AbstractPipeline},
    clip::{ClipIdentifier, ClipType, CompositedClip},
    constants::{data_location, media_output_location, CHUNK_LENGTH},
    node::{
        Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableStreamType, PipeableType, PipedType,
        Type,
    },
    store::Store,
    ID,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "output";
pub mod INPUTS {
    pub const MEDIA: &str = "media";
    pub const CLIP: &str = "clip";
}
pub mod OUTPUTS {}

fn default_properties() -> HashMap<String, NodeTypeInput> {
    let mut default_properties = HashMap::new();

    default_properties.insert(
        String::from(INPUTS::MEDIA),
        NodeTypeInput {
            name: String::from(INPUTS::MEDIA),
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
        String::from(INPUTS::CLIP),
        NodeTypeInput {
            name: String::from(INPUTS::CLIP),
            display_name: String::from("Clip"),
            description: String::from("Clip to output"),
            property_type: Type::Clip,
        },
    );
    default_properties
}

fn get_io(
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
    let inputs = default_properties();
    let outputs = HashMap::new();
    return Ok((inputs, outputs));
}
fn get_output(
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<AbstractPipeline, String> {
    let mut pipeline = AbstractPipeline::new();

    let media = piped_inputs.get(INPUTS::MEDIA);
    if media.is_none() {
        return Err(format!("Media is none!"));
    }
    let media = media.unwrap();
    let clip = get_clip(properties, store);
    if clip.is_err() {
        return Err(clip.unwrap_err());
    }
    let clip = clip.unwrap();

    let gst_clip_id = format!("composited-clip-file-{}", clip.id);
    {
        let mut props = HashMap::new();
        props.insert("location".to_string(), clip.get_output_location_template());
        props.insert("muxer-factory".to_string(), "mp4mux".to_string());
        props.insert(
            "muxer-properties".to_string(),
            "\"properties,streamable=true,fragment-duration=1000\"".to_string(),
        );
        // makes it fragmented; one fragment each second (=1000 ms)
        props.insert("async-finalize".to_string(), "true".to_string());
        let nanoseconds = (CHUNK_LENGTH as u64) * 1000000000;
        props.insert("max-size-time".to_string(), nanoseconds.to_string());
        props.insert("send-keyframe-requests".to_string(), "true".to_string());

        let splitmuxsink_node =
            AbstractNode::new_with_props("splitmuxsink", Some(gst_clip_id.clone()), props);

        pipeline.add_node(splitmuxsink_node);
    }

    for (stream_type, num) in media.stream_type.get_map() {
        if stream_type == PipeableStreamType::Video && num > 1 {
            panic!("Currently, splitmuxsink only supports one video stream. The application has attempted to pipe in {} streams, which is unsupported", num);
        }
        for i in 0..num {
            let gst1 = media.get_gst_handle(&stream_type, i);
            let gst2 = clip.get_gstreamer_id(&stream_type, i);
            if gst1.is_none() {
                return Err(format!("Cannot get handle for media"));
            }
            let gst1 = gst1.unwrap();
            {
                let stream_linker_node =
                    AbstractNode::new(stream_type.stream_linker().as_str(), Some(gst2.clone()));
                let link = AbstractLink {
                    from: AbstractLinkEndpoint::new(gst1),
                    to: AbstractLinkEndpoint::new(stream_linker_node.id.clone()),
                };
                pipeline.add_node(stream_linker_node);
                pipeline.link_abstract(link);
            }
            {
                let queue_node = AbstractNode::new("queue", None);
                let link = AbstractLink {
                    from: AbstractLinkEndpoint::new(gst2.clone()),
                    to: AbstractLinkEndpoint::new(queue_node.id.clone()),
                };

                pipeline.link_abstract(link);

                let encoder_input_id;
                let encoder_output_id;

                match &stream_type {
                    PipeableStreamType::Video => {
                        let mut props = HashMap::new();
                        props.insert("bitrate".to_string(), 400.to_string());
                        let nvh264enc_node = AbstractNode::new_with_props("nvh264enc", None, props);

                        let h264parse_node = AbstractNode::new("h264parse", None);

                        encoder_input_id = nvh264enc_node.id.clone();
                        encoder_output_id = h264parse_node.id.clone();

                        pipeline.link(&nvh264enc_node, &h264parse_node);
                        pipeline.add_node(nvh264enc_node);
                        pipeline.add_node(h264parse_node);
                    }
                    PipeableStreamType::Audio => {
                        let avenc_aac_node = AbstractNode::new("avenc_aac", None);

                        encoder_input_id = avenc_aac_node.id.clone();
                        encoder_output_id = avenc_aac_node.id.clone();

                        pipeline.add_node(avenc_aac_node);
                    }
                    PipeableStreamType::Subtitles => todo!(),
                }

                let link = AbstractLink {
                    from: AbstractLinkEndpoint::new(queue_node.id.clone()),
                    to: AbstractLinkEndpoint::new(encoder_input_id.clone()),
                };
                pipeline.link_abstract(link);

                let link = AbstractLink {
                    from: AbstractLinkEndpoint::new(encoder_output_id.clone()),
                    to: AbstractLinkEndpoint::new_with_property(
                        gst_clip_id.clone(),
                        match stream_type {
                            PipeableStreamType::Video => String::from("video"),
                            _ => format!("{}_{}", stream_type.to_string(), i),
                        },
                    ),
                };
                pipeline.link_abstract(link);

                pipeline.add_node(queue_node);
            }
        }
    }
    return Ok(pipeline);
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
        //   let media = properties.get(INPUTS::MEDIA).unwrap();
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
    let clip = properties.get(INPUTS::CLIP);
    if clip.is_none() {
        return Err(String::from("No clip given"));
    }
    let clip = clip.unwrap().to_owned();
    let clip = serde_json::from_value::<ClipIdentifier>(clip).unwrap();
    return Ok(store.clips.composited.get(&clip.id).unwrap().clone());
}
