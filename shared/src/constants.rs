use once_cell::sync::Lazy;
use std::{fs, sync::Mutex};

static DATA_LOCATION: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static IS_SERVER: Lazy<Mutex<Option<bool>>> = Lazy::new(|| Mutex::new(None));

pub const CHUNK_LENGTH: u8 = 10;
pub const CHUNK_FILENAME_NUMBER_LENGTH: u8 = 6;

pub fn data_location() -> String {
    DATA_LOCATION.lock().unwrap().as_ref().unwrap().clone()
}
pub fn media_output_location() -> String {
    format!("{}/output", data_location())
}
pub fn source_files_location() -> String {
    format!("{}/source", data_location())
}
pub fn store_json_location() -> String {
    format!("{}/pipeline.json", data_location())
}

pub fn temp_location() -> String {
    format!("{}/temp", data_location())
}

pub fn projects_location() -> String {
    format!("{}/projects", temp_location())
}

pub fn intermediate_files_location() -> String {
    format!("{}/intermediate", temp_location())
}
pub fn composited_clips_projects_location() -> String {
    format!("{}/composited-clips", intermediate_files_location())
}

pub fn is_server() -> bool {
    IS_SERVER.lock().unwrap().as_ref().unwrap().clone()
}

pub fn init(data_location: String, is_server: bool) {
    let mut value = DATA_LOCATION.lock().unwrap();
    *value = Some(data_location);

    drop(value);
    let mut value = IS_SERVER.lock().unwrap();
    *value = Some(is_server);
    drop(value);

    gst::init().unwrap();
    ges::init().unwrap();

    fs::create_dir_all(media_output_location()).unwrap();
    fs::create_dir_all(source_files_location()).unwrap();
    fs::create_dir_all(temp_location()).unwrap();
    fs::create_dir_all(projects_location()).unwrap();
    fs::create_dir_all(intermediate_files_location()).unwrap();
    fs::create_dir_all(composited_clips_projects_location()).unwrap();
}
