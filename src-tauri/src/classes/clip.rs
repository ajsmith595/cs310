use crate::classes::node::PipeableType;

use super::{node::Type, pipeline::Pipeline, ID};

#[derive(Serialize, Deserialize)]
pub enum ClipType {
  Source,
  Composited,
}

#[derive(Serialize, Deserialize)]
pub struct ClipIdentifier {
  pub id: ID,
  pub clip_type: ClipType,
}

#[derive(Clone)]
pub struct SourceClip {
  pub id: ID,
  pub name: String,
  pub file_location: String,
}

impl SourceClip {
  pub fn get_clip_type(&self) -> Type {
    return Type::Pipeable(Some(PipeableType::Video));
    todo!(); // TODO: look at file, determine whether it is audio/video/image
  }
}
#[derive(Clone)]
pub struct CompositedClip {
  pub id: ID,
  pub name: String,
  pub pipeline_id: ID,
}
impl CompositedClip {
  pub fn get_gstreamer_id(&self) -> String {
    format!("clip-{}", self.id)
  }
}
