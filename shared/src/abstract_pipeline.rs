use std::collections::HashMap;

use petgraph::graph::DiGraph;
use serde_json::Value;

use super::{global::uniq_id, node::PipeableStreamType, pipeline::LinkEndpoint};

#[derive(Debug, Clone)]
pub struct AbstractNode {
    pub id: String,
    pub node_type: String,
    pub properties: HashMap<String, String>,
}

impl AbstractNode {
    pub fn new(node_type: &str, id: Option<String>) -> Self {
        let id = match id {
            Some(id) => id,
            _ => uniq_id().to_string(),
        };
        Self {
            id,
            node_type: String::from(node_type),
            properties: HashMap::new(),
        }
    }
    pub fn new_with_props(
        node_type: &str,
        id: Option<String>,
        properties: HashMap<String, String>,
    ) -> Self {
        let id = match id {
            Some(id) => id,
            _ => uniq_id().to_string(),
        };
        Self {
            id,
            node_type: String::from(node_type),
            properties: properties,
        }
    }

    pub fn new_encoder(stream_type: &PipeableStreamType) -> Self {
        Self {
            id: uniq_id().to_string(),
            node_type: format!("encoder:{}", stream_type.to_string()),
            properties: HashMap::new(),
        }
    }
    pub fn new_encoder_with_props(
        stream_type: &PipeableStreamType,
        properties: HashMap<String, String>,
    ) -> Self {
        Self {
            id: uniq_id().to_string(),
            node_type: format!("encoder:{}", stream_type.to_string()),
            properties,
        }
    }

    pub fn new_decoder(stream_type: &PipeableStreamType) -> Self {
        Self {
            id: uniq_id().to_string(),
            node_type: format!("decoder:{}", stream_type.to_string()),
            properties: HashMap::new(),
        }
    }
    pub fn new_decoder_with_props(
        stream_type: &PipeableStreamType,
        properties: HashMap<String, String>,
    ) -> Self {
        Self {
            id: uniq_id().to_string(),
            node_type: format!("decoder:{}", stream_type.to_string()),
            properties,
        }
    }

    pub fn to_gstreamer_pipeline(&self) -> String {
        let mut pipeline = format!("{} name={}", self.node_type, self.id.clone());
        for (prop, value) in &self.properties {
            pipeline = format!("{} {}={}", pipeline, prop, value);
        }
        pipeline
    }

    pub fn is_linker(&self) -> bool {
        match &self.node_type[..] {
            "videoconvert" => true,
            "audioconvert" => true,
            "subparse" => true,
            _ => false,
        }
    }

    pub fn linker_to_type(&self) -> PipeableStreamType {
        if !self.is_linker() {
            panic!("Attempted to obtain linker type of non-linker node");
        }

        match &self.node_type[..] {
            "videoconvert" => PipeableStreamType::Video,
            "audioconvert" => PipeableStreamType::Audio,
            "subparse" => PipeableStreamType::Subtitles,
            _ => panic!("Invalid type!"),
        }
    }

    pub fn is_aliased(&self) -> bool {
        return self.is_encoder() || self.is_decoder();
    }
    fn is_decoder(&self) -> bool {
        return self.node_type.starts_with("decoder:");
    }
    fn is_encoder(&self) -> bool {
        return self.node_type.starts_with("encoder:");
    }

    pub fn alias_to_pipeline(&self) -> (AbstractPipeline, String, String) {
        if !self.is_aliased() {
            panic!("Attempted to convert non-aliased node to pipeline!");
        }

        let split = self.node_type.split(":").nth(1).unwrap();
        let stream_type = match split {
            "video" => PipeableStreamType::Video,
            "audio" => PipeableStreamType::Audio,
            "subtitles" => PipeableStreamType::Subtitles,
            _ => panic!("Cannot determine stream type"),
        };

        if self.is_encoder() {
            match stream_type {
                PipeableStreamType::Video => {
                    let mut pipeline = AbstractPipeline::new();

                    let mut props = HashMap::new();
                    props.insert("bitrate".to_string(), 400.to_string());
                    let nvh264enc_node = AbstractNode::new_with_props("nvh264enc", None, props);

                    let h264parse_node = AbstractNode::new("h264parse", None);

                    let encoder_input_id = nvh264enc_node.id.clone();
                    let encoder_output_id = h264parse_node.id.clone();

                    pipeline.link(&nvh264enc_node, &h264parse_node);
                    pipeline.add_node(nvh264enc_node);
                    pipeline.add_node(h264parse_node);

                    (pipeline, encoder_input_id, encoder_output_id)
                }
                PipeableStreamType::Audio => {
                    let mut pipeline = AbstractPipeline::new();

                    let avenc_aac_node = AbstractNode::new("avenc_aac", None);

                    let encoder_input_id = avenc_aac_node.id.clone();
                    let encoder_output_id = avenc_aac_node.id.clone();

                    pipeline.add_node(avenc_aac_node);

                    (pipeline, encoder_input_id, encoder_output_id)
                }
                PipeableStreamType::Subtitles => todo!(),
            }
        } else {
            // it's a decoder!
            match stream_type {
                PipeableStreamType::Video => {
                    let mut pipeline = AbstractPipeline::new();

                    let h264parse_node = AbstractNode::new("h264parse", None);
                    let nvh264dec_node = AbstractNode::new("nvh264dec", None);

                    let encoder_input_id = h264parse_node.id.clone();
                    let encoder_output_id = nvh264dec_node.id.clone();

                    pipeline.link(&h264parse_node, &nvh264dec_node);
                    pipeline.add_node(h264parse_node);
                    pipeline.add_node(nvh264dec_node);

                    (pipeline, encoder_input_id, encoder_output_id)
                }
                PipeableStreamType::Audio => {
                    let mut pipeline = AbstractPipeline::new();

                    let avdec_aac_node = AbstractNode::new("avdec_aac", None);
                    let audioconvert_node = AbstractNode::new("audioconvert", None);
                    let audioresample_node = AbstractNode::new("audioresample", None);

                    let encoder_input_id = avdec_aac_node.id.clone();
                    let encoder_output_id = audioresample_node.id.clone();

                    pipeline.link(&avdec_aac_node, &audioconvert_node);
                    pipeline.link(&audioconvert_node, &audioresample_node);
                    pipeline.add_node(avdec_aac_node);
                    pipeline.add_node(audioconvert_node);
                    pipeline.add_node(audioresample_node);

                    (pipeline, encoder_input_id, encoder_output_id)
                }
                PipeableStreamType::Subtitles => todo!(),
            }
        }
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AbstractLinkEndpoint {
    pub id: String,
    pub property: Option<String>,
}

impl AbstractLinkEndpoint {
    pub fn new(id: String) -> Self {
        Self { id, property: None }
    }

    pub fn new_with_property(id: String, property: String) -> Self {
        Self {
            id,
            property: Some(property),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AbstractLink {
    pub from: AbstractLinkEndpoint,
    pub to: AbstractLinkEndpoint,
}

impl AbstractLink {
    pub fn to_gstreamer_pipeline(&self) -> String {
        let from = match &self.from.property {
            Some(property) => format!("{}.{}", self.from.id, property),
            _ => format!("{}.", self.from.id),
        };
        let to = match &self.to.property {
            Some(property) => format!("{}.{}", self.to.id, property),
            _ => format!("{}.", self.to.id),
        };
        format!("{} ! {}", from, to)
    }
}

#[derive(Debug)]
pub struct AbstractPipeline {
    pub nodes: HashMap<String, AbstractNode>,
    pub links: Vec<AbstractLink>,
}

impl AbstractPipeline {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            links: Vec::new(),
        }
    }
    pub fn add_node(&mut self, node: AbstractNode) {
        self.nodes.insert(node.id.clone(), node);
    }
    pub fn link(&mut self, from: &AbstractNode, to: &AbstractNode) {
        self.links.push(AbstractLink {
            from: AbstractLinkEndpoint::new(from.id.clone()),
            to: AbstractLinkEndpoint::new(to.id.clone()),
        });
    }
    pub fn link_abstract(&mut self, link: AbstractLink) {
        self.links.push(link);
    }

    pub fn merge(&mut self, other: AbstractPipeline) {
        for (id, node) in other.nodes {
            self.add_node(node);
        }
        for link in other.links {
            self.link_abstract(link);
        }
    }

    fn handle_splits(&mut self) {
        let mut map = HashMap::new();

        for link in &self.links {
            if !map.contains_key(&link.from) {
                map.insert(link.from.clone(), 0);
            }
            let new_value = map.get(&link.from).unwrap() + 1;
            map.insert(link.from.clone(), new_value);
        }

        let mut io_map = HashMap::new();

        for (link, num) in &map {
            if *num > 1 {
                let target = self.nodes.get(&link.id).unwrap().clone();

                if !target.is_linker() {
                    println!(
                        "NOT A LINKER: {} (it is instead a {})",
                        target.id, target.node_type
                    );
                } else {
                    let linker_type = target.linker_to_type();
                    let encoder = AbstractNode::new_encoder(&linker_type);
                    let tee = AbstractNode::new("tee", None);

                    io_map.insert(
                        link.clone(),
                        (tee.id.clone(), encoder.id.clone(), linker_type),
                    );

                    // self.link(&target, &encoder);
                    self.link(&encoder, &tee);

                    self.add_node(encoder);
                    self.add_node(tee);
                }
            }
        }

        let mut extra_pipeline = AbstractPipeline::new();
        for link in &mut self.links {
            if io_map.contains_key(&link.from) {
                let (tee_id, encoder_id, linker_type) = io_map.get(&link.from).unwrap();

                let mut props = HashMap::new();

                props.insert("max-size-buffers".to_string(), "0".to_string());
                props.insert("max-size-bytes".to_string(), "0".to_string());
                props.insert("max-size-time".to_string(), "0".to_string());
                let queue_node = AbstractNode::new_with_props("queue", None, props);
                let decoder_node = AbstractNode::new_decoder(linker_type);

                extra_pipeline.link_abstract(AbstractLink {
                    from: AbstractLinkEndpoint::new(tee_id.clone()),
                    to: AbstractLinkEndpoint::new(queue_node.id.clone()),
                });

                extra_pipeline.link(&queue_node, &decoder_node);

                link.from = AbstractLinkEndpoint::new(decoder_node.id.clone());

                extra_pipeline.add_node(queue_node);
                extra_pipeline.add_node(decoder_node);
            }
        }

        self.merge(extra_pipeline);

        for (link, (tee_id, encoder_id, linker_type)) in io_map {
            self.link_abstract(AbstractLink {
                from: AbstractLinkEndpoint::new(link.id.clone()),
                to: AbstractLinkEndpoint::new(encoder_id),
            })
        }

        // for link in &self.links {
        //   if map.get(&link.from).unwrap() > 1 {}
        // }
        // todo!(); // DOES NOT WORK WITHOUT THIS IMPLEMENTED! CURRENTLY PARSING ERROR DUE TO MULTIPLE SPLITS (with no tee element used)
        // adds tee elements and queue elements as necessary
    }

    fn optimise(&mut self) {
        // removes double encoding/decoding pairs, etc.

        // remove unused
    }

    fn convert_aliases(&mut self) {
        let mut map = HashMap::new();
        for (id, node) in self.nodes.clone() {
            if node.is_aliased() {
                let (extra_pipeline, input_id, output_id) = node.alias_to_pipeline();

                self.merge(extra_pipeline);

                map.insert(node.id.clone(), (input_id, output_id));
            }
        }
        for (node_id, (input_id, output_id)) in &map {
            self.nodes.remove(node_id);
        }

        for link in &mut self.links {
            if map.contains_key(&link.from.id) {
                let (input, output) = map.get(&link.from.id).unwrap();
                link.from.id = output.clone();
            }

            if map.contains_key(&link.to.id) {
                let (input, output) = map.get(&link.to.id).unwrap();
                link.to.id = input.clone();
            }
        }
        // converts the aliases (e.g. encoder/decoders) to the actual required representations
    }

    fn remove_dangling(&mut self) {
        let mut nodes_to_check = self
            .nodes
            .clone()
            .into_iter()
            .map(|(id, node)| id)
            .collect::<Vec<_>>();

        while (nodes_to_check.len() > 0) {
            let next_item = nodes_to_check.remove(0);

            match self.check_and_remove_dangling_node(next_item) {
                Some(vec) => {
                    let mut vec = vec.clone();
                    nodes_to_check.append(&mut vec);
                }
                None => {}
            }
        }
    }

    /// Checks the node specified to see if it is a "dead-end" that's not a filesink.
    ///
    /// If it is a dead-end that's not a filesink, it will be removed, and a list of node IDs that
    /// were connected to it are then returned. `None` returned otherwise
    fn check_and_remove_dangling_node(&mut self, id: String) -> Option<Vec<String>> {
        let node = self.nodes.get(&id);
        let node = match node {
            Some(node) => node,
            None => return None,
        };

        match node.node_type.as_str() {
            "splitmuxsink" | "filesink" => {
                return None;
            }
            _ => {}
        }

        let links_from_node: Vec<AbstractLink> = self
            .links
            .clone()
            .into_iter()
            .filter(|x| x.from.id == id)
            .collect::<Vec<_>>();

        if links_from_node.len() > 0 {
            return None;
        }

        let nodes_incoming: Vec<String> = self
            .links
            .clone()
            .into_iter()
            .filter(|x| x.to.id == id)
            .map(|x| x.from.id)
            .collect::<Vec<_>>();

        self.nodes.remove(&id);
        self.links.retain(|x| x.to.id != id);

        Some(nodes_incoming)
    }

    pub fn to_gstreamer_pipeline(&mut self) -> String {
        self.handle_splits();
        self.remove_dangling();
        self.optimise();
        self.convert_aliases();
        self.remove_dangling();

        // todo!();

        let mut str = String::from("");

        for link in &self.links {
            str = format!("{}\n {}", str, link.to_gstreamer_pipeline());
        }

        for (id, node) in &self.nodes {
            str = format!("{}\n {}", str, node.to_gstreamer_pipeline());
        }
        str
    }

    //   pub fn parse_simple(pipeline: String) -> Self {
    //     let mut nodes = Vec::new();
    //     let mut links = Vec::new();

    //     let parts = pipeline.trim().split(" ");
    //     enum Stage {
    //       Linker,
    //       NodeType,
    //       NamedNode,
    //       None,
    //     }
    //     let mut previous_stage = Stage::None;
    //     let mut previous_id: Option<String> = None;
    //     for part in parts {
    //       if part.ends_with(".") {
    //         let this_id = part.trim().replace(".", "");
    //         if let Stage::Linker = previous_stage {
    //           links.push(AbstractLink {
    //             from: previous_id.unwrap().clone(),
    //             to: this_id.clone(),
    //           });
    //         }

    //         previous_id = Some(this_id.clone());
    //         previous_stage = Stage::NamedNode;
    //       } else if part.trim() == "!" {
    //         previous_stage = Stage::Linker;
    //       } else if part.contains("=") {

    //       }
    //       else {
    //         previous_stage = Stage::NodeType;
    //       }
    //     }

    //     Self { nodes, links }
    //   }
}
