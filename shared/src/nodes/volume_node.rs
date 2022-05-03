use std::collections::HashMap;

use ges::traits::{GESContainerExt, LayerExt, TimelineExt};
use serde_json::Value;

use crate::{
    node::{
        InputOrOutput, NodeType, NodeTypeInput, NodeTypeOutput, PipeableType, PipedType,
        Restrictions, Type,
    },
    store::Store,
    ID,
};

use super::NodeRegister;

pub const IDENTIFIER: &str = "audio_gain";
pub mod inputs {
    pub const MEDIA: &str = "media";
    pub const GAIN: &str = "gain";
}
pub mod outputs {
    pub const OUTPUT: &str = "output";
}

fn default_properties() -> HashMap<String, NodeTypeInput> {
    let mut default_properties = HashMap::new();
    {
        default_properties.insert(
            String::from(inputs::MEDIA),
            NodeTypeInput {
                name: String::from(inputs::MEDIA),
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
            String::from(inputs::GAIN),
            NodeTypeInput {
                name: String::from(inputs::GAIN),
                display_name: String::from("Multiplier"),
                description: String::from("The volume multiplier"),
                property_type: Type::Number(Restrictions {
                    min: (0 as f64),
                    max: (10 as f64),
                    step: (0.01 as f64),
                    default: (1 as f64),
                }),
            },
        );
    }
    default_properties
}
pub fn get_io(
    _node_id: ID,
    _properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
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
    let mut outputs = HashMap::new();
    let mut pipeable_type = PipeableType {
        video: 0,
        audio: 1,
        subtitles: 0,
    };
    let piped_input = piped_inputs.get(inputs::MEDIA);
    if let Some(piped_input) = piped_input {
        pipeable_type = piped_input.stream_type;
    }

    outputs.insert(
        outputs::OUTPUT.to_string(),
        NodeTypeOutput {
            name: outputs::OUTPUT.to_string(),
            description: "The gained media".to_string(),
            display_name: "Output".to_string(),
            property_type: pipeable_type,
        },
    );

    return Ok((inputs, outputs));
}

pub fn get_output(
    node_id: ID,
    properties: &HashMap<String, Value>,
    piped_inputs: &HashMap<String, PipedType>,
    composited_clip_types: &HashMap<ID, PipedType>,
    store: &Store,
    node_register: &NodeRegister,
) -> Result<HashMap<String, ges::Timeline>, String> {
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

    let media = piped_inputs.get(inputs::MEDIA);
    if media.is_none() {
        return Err(format!("No media input!"));
    }
    let media = media.unwrap();
    let gain = properties.get(inputs::GAIN);
    if gain.is_none() {
        return Err(format!("No gain input!"));
    }
    let gain = gain.unwrap();
    if let Value::Number(gain) = gain {
        let output = outputs.get(outputs::OUTPUT).unwrap();

        let output = PipedType {
            stream_type: output.property_type,
            node_id,
            property_name: String::from(outputs::OUTPUT),
            io: InputOrOutput::Output,
            cache_id: None,
        };

        let effect = ges::Effect::new(
            format!(
                "audioamplify amplification={}",
                gain.as_f64().unwrap().to_string()
            )
            .as_str(),
        )
        .unwrap();
        let timeline = output.stream_type.create_timeline();

        let layer = timeline.append_layer();
        let clip = ges::UriClip::new(media.get_gst_save_location_with_cache().as_str()).unwrap();

        clip.add(&effect).unwrap();
        layer.add_clip(&clip).unwrap();

        let mut hm = HashMap::new();
        hm.insert(outputs::OUTPUT.to_string(), timeline);
        return Ok(hm);
    }
    return Err(format!("Media is invalid type (audio gain blur)"));
}
pub fn volume_node() -> NodeType {
    NodeType {
        id: String::from(IDENTIFIER),
        display_name: String::from("Volume"),
        description: String::from("Apply a multiplier to the volume of the source clip"),
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
