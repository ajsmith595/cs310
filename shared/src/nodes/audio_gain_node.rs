use std::collections::HashMap;

use serde_json::Value;

use crate::{
    abstract_pipeline::{AbstractLink, AbstractLinkEndpoint, AbstractNode, AbstractPipeline},
    clip::{ClipIdentifier, ClipType},
    node::{
        InputOrOutput, Node, NodeType, NodeTypeInput, NodeTypeOutput, PipeableStreamType,
        PipeableType, PipedType, Restrictions, Type,
    },
    store::Store,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "audio_gain";
pub mod INPUTS {
    pub const MEDIA: &str = "media";
    pub const GAIN: &str = "gain";
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
                description: String::from("The media to be gained"),
                property_type: Type::Pipeable(
                    PipeableType {
                        video: 0,
                        audio: 1,
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
            String::from(INPUTS::GAIN),
            NodeTypeInput {
                name: String::from(INPUTS::GAIN),
                display_name: String::from("Gain Amount"),
                description: String::from("The amount to gain by"),
                property_type: Type::Number(Restrictions {
                    min: (-12 as f64),
                    max: (12 as f64),
                    step: (0.1 as f64),
                    default: (0 as f64),
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
    let mut pipeable_type = PipeableType {
        video: 0,
        audio: 1,
        subtitles: 0,
    };
    let piped_input = piped_inputs.get(INPUTS::MEDIA);
    if let Some(piped_input) = piped_input {
        pipeable_type = piped_input.stream_type;
    }

    outputs.insert(
        OUTPUTS::OUTPUT.to_string(),
        NodeTypeOutput {
            name: OUTPUTS::OUTPUT.to_string(),
            description: "The gained media".to_string(),
            display_name: "Output".to_string(),
            property_type: pipeable_type,
        },
    );

    return Ok((inputs, outputs));
}

pub fn get_output(
    node_id: String,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<String, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<AbstractPipeline, String> {
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
    let gain = properties.get(INPUTS::GAIN);
    if gain.is_none() {
        return Err(format!("No gain input!"));
    }
    let gain = gain.unwrap();
    if let Value::Number(gain) = gain {
        let mut pipeline = AbstractPipeline::new();

        //let gst_string = String::from("");

        let output = outputs.get(OUTPUTS::OUTPUT).unwrap();
        let output = PipedType {
            stream_type: output.property_type,
            node_id,
            property_name: String::from(OUTPUTS::OUTPUT),
            io: InputOrOutput::Output,
        };

        let video_passthrough =
            PipedType::gst_transfer_pipe_type(&media, &output, &PipeableStreamType::Video);
        let subtitle_passthrough =
            PipedType::gst_transfer_pipe_type(&media, &output, &PipeableStreamType::Subtitles);

        if video_passthrough.is_none() || subtitle_passthrough.is_none() {
            return Err(format!("Could not get video/subtitle passthrough"));
        }
        let (video_passthrough, subtitle_passthrough) =
            (video_passthrough.unwrap(), subtitle_passthrough.unwrap());

        pipeline.merge(video_passthrough);
        pipeline.merge(subtitle_passthrough);

        for i in 0..output.stream_type.audio {
            let mut props = HashMap::new();
            props.insert("volume".to_string(), gain.as_f64().unwrap().to_string());
            let volume_node = AbstractNode::new_with_props("volume", None, props);

            pipeline.link_abstract(AbstractLink {
                from: AbstractLinkEndpoint::new(
                    media.get_gst_handle(&PipeableStreamType::Audio, i).unwrap(),
                ),
                to: AbstractLinkEndpoint::new(volume_node.id.clone()),
            });
            pipeline.link_abstract(AbstractLink {
                from: AbstractLinkEndpoint::new(volume_node.id.clone()),
                to: AbstractLinkEndpoint::new(
                    output
                        .get_gst_handle(&PipeableStreamType::Audio, i)
                        .unwrap(),
                ),
            });
            pipeline.add_node(volume_node);
        }

        return Ok(pipeline);
    }
    return Err(format!("Media is invalid type (audio gain blur)"));
}
pub fn audio_gain() -> NodeType {
    NodeType {
        id: String::from(IDENTIFIER),
        display_name: String::from("Audio Gain"),
        description: String::from("Increase the volume of a source"),
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
