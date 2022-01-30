use once_cell::sync::Lazy;
use std::{fs, sync::Mutex};

static DATA_LOCATION: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub const CHUNK_LENGTH: u8 = 10;
pub const CHUNK_FILENAME_NUMBER_LENGTH: u8 = 6;

pub fn data_location() -> String {
    DATA_LOCATION.lock().unwrap().as_ref().unwrap().clone()
}
pub fn media_output_location() -> String {
    format!("{}\\output", data_location())
}
pub fn source_files_location() -> String {
    format!("{}\\source", data_location())
}
pub fn store_json_location() -> String {
    format!("{}\\pipeline.json", data_location())
}

pub fn init(data_location: String) {
    let mut value = DATA_LOCATION.lock().unwrap();
    *value = Some(data_location);

    drop(value);

    fs::create_dir_all(media_output_location()).unwrap();
    fs::create_dir_all(source_files_location()).unwrap();
}
