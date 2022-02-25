use serde_json::Value;
use uuid::Uuid;

use crate::{
    clip::{ClipIdentifier, ClipType, CompositedClip, SourceClip},
    node::Node,
    pipeline::Link,
    store::Store,
    ID,
};

#[derive(Clone)]
pub enum Task {
    UpdateNode(ID, Node),
    AddNode(Node),
    AddLink(Link),
    DeleteLinks(ID, Option<String>),
    DeleteNode(ID),
    UpdateClip(ID, ClipType, Value),
    CreateSourceClip(SourceClip),
    CreateCompositedClip(CompositedClip),
}

#[derive(Clone)]
pub enum NetworkTask {
    GetSourceClipID(ID),
    GetCompositedClipID(ID),
    GetNodeID(ID),
    UpdateNode(ID),
    AddLink(Link),
    DeleteLinks(ID, Option<String>),
    DeleteNode(ID),
    UpdateClip(ID, ClipType),
}

impl Task {
    pub fn apply_tasks(store: &mut Store, tasks: Vec<Task>) -> Vec<NetworkTask> {
        let mut network_jobs = Vec::new();
        for task in tasks {
            match task {
                Task::UpdateNode(id, node) => {
                    let res = store.nodes.insert(id, node);
                    if res.is_none() {
                        store.nodes.remove(&id);
                    } else {
                        network_jobs.push(NetworkTask::UpdateNode(id));
                    }
                }
                Task::AddNode(node) => {
                    let id = node.id.clone();
                    network_jobs.push(NetworkTask::GetNodeID(id.clone()));
                    store.nodes.insert(id, node);
                }
                Task::AddLink(link) => {
                    let mut new_links = store.pipeline.links.clone();
                    new_links = new_links
                        .into_iter()
                        .filter(|x| x.to.get_id() != link.to.get_id())
                        .collect();

                    network_jobs.push(NetworkTask::AddLink(link.clone()));
                    new_links.push(link);

                    store.pipeline.links = new_links;
                }
                Task::DeleteLinks(id, property) => {
                    let mut new_links = store.pipeline.links.clone();

                    new_links = match property.clone() {
                        None => new_links
                            .into_iter()
                            .filter(|x: &Link| x.to.node_id != id)
                            .collect(),
                        Some(prop) => new_links
                            .into_iter()
                            .filter(|x: &Link| x.to.node_id != id && x.to.property != prop)
                            .collect(),
                    };
                    store.pipeline.links = new_links;
                    network_jobs.push(NetworkTask::DeleteLinks(id, property));
                }
                Task::DeleteNode(id) => {
                    let mut new_links = store.pipeline.links.clone();

                    new_links = new_links
                        .into_iter()
                        .filter(|x: &Link| x.to.node_id != id && x.from.node_id != id)
                        .collect();
                    store.pipeline.links = new_links;
                    store.nodes.remove(&id);

                    network_jobs.push(NetworkTask::DeleteNode(id));
                }
                Task::CreateSourceClip(clip) => {
                    network_jobs.push(NetworkTask::GetSourceClipID(clip.id.clone()));
                    store.clips.source.insert(clip.id.clone(), clip);
                }
                Task::CreateCompositedClip(clip) => {
                    let mut output_node = Node::new(String::from("output"), Some(Uuid::new_v4()));
                    let clip_identifier = ClipIdentifier {
                        id: clip.id,
                        clip_type: ClipType::Composited,
                    };
                    output_node.properties.insert(
                        String::from("clip"),
                        serde_json::to_value(&clip_identifier).unwrap(),
                    );

                    network_jobs.push(NetworkTask::GetCompositedClipID(clip.id.clone()));
                    store.clips.composited.insert(clip.id.clone(), clip);
                    store.nodes.insert(output_node.id.clone(), output_node);
                }
                Task::UpdateClip(id, clip_type, clip) => {
                    match clip_type {
                        ClipType::Source => {
                            let clip = serde_json::from_value::<SourceClip>(clip);
                            if clip.is_err() {
                                continue;
                            }
                            let mut clip = clip.unwrap();

                            clip.id = id.clone();
                            let existing_clip = store.clips.source.get_mut(&id);
                            if existing_clip.is_none() {
                                continue;
                            }
                            let existing_clip = existing_clip.unwrap();
                            *existing_clip = clip;
                        }
                        ClipType::Composited => {
                            let clip = serde_json::from_value::<CompositedClip>(clip);
                            if clip.is_err() {
                                continue;
                            }
                            let mut clip = clip.unwrap();

                            clip.id = id.clone();
                            let existing_clip = store.clips.composited.get_mut(&id);
                            if existing_clip.is_none() {
                                continue;
                            }
                            let existing_clip = existing_clip.unwrap();
                            *existing_clip = clip;
                        }
                    }
                    network_jobs.push(NetworkTask::UpdateClip(id, clip_type));
                }
            }
        }
        network_jobs
    }
}
