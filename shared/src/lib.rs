#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate enum_primitive;
// #[macro_use]
// extern crate erased_serde;
// extern crate dirs;
extern crate dirs;
extern crate gstreamer;
extern crate gstreamer_pbutils;
extern crate serde;
extern crate serde_json;
extern crate uuid;

pub mod abstract_pipeline;
pub mod clip;
pub mod global;
pub mod networking;
pub mod node;
pub mod nodes;
pub mod pipeline;
pub mod store;
pub type ID = String;
