use std::{collections::HashMap, fs::File, io::Write};

use rfd::AsyncFileDialog;

use crate::{
  classes::{
    clip::{CompositedClip, SourceClip},
    global::uniq_id,
    node::{Node, NodeTypeProperty},
    nodes::NodeRegister,
    pipeline::Pipeline,
    store::Store,
    ID,
  },
  state_manager::SharedStateWrapper,
};

#[tauri::command]
pub async fn import_media(
  state: tauri::State<'_, SharedStateWrapper>,
  window: tauri::Window,
) -> Result<Store, String> {
  let dialog = AsyncFileDialog::new()
    .set_parent(&tauri::api::dialog::window_parent(&window).expect("Could not get window parent"))
    .add_filter("Video", &["mp4"]);
  let file = dialog.pick_files().await;
  match file {
    None => Err(String::from("No file selected")),
    Some(paths) => {
      for path in paths {
        println!("Starting return...");

        let file_path = path.path().to_str().unwrap().to_string();
        let id = uniq_id();
        Pipeline::get_video_thumbnail(file_path.clone(), id.clone());

        let thumbnail = format!(
          "{}/thumbnails/source/{}.jpg",
          std::env::current_dir().unwrap().to_str().unwrap(),
          id.clone()
        );
        let clip = SourceClip {
          id,
          name: path.file_name(),
          file_location: file_path,
          thumbnail_location: Some(thumbnail),
        };

        (&mut state
          .0
          .clone()
          .lock()
          .unwrap()
          .stored_state
          .store
          .clips
          .source)
          .insert(clip.id.clone(), clip);
      }
      let state = state.0.clone().lock().unwrap().stored_state.store.clone();
      let mut f = File::create("state.json").unwrap();
      f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
        .unwrap();
      Ok(state)
    }
  }
}

#[tauri::command]
pub fn create_composited_clip(
  state: tauri::State<'_, SharedStateWrapper>,
  window: tauri::Window,
) -> Result<(String, Store), String> {
  let clip = CompositedClip {
    id: uniq_id(),
    name: "New Clip".to_string(),
    pipeline_id: uniq_id(),
  };
  let id = clip.id.clone();
  (&mut state
    .0
    .clone()
    .lock()
    .unwrap()
    .stored_state
    .store
    .clips
    .composited)
    .insert(clip.id.clone(), clip);

  let state = state.0.clone().lock().unwrap().stored_state.store.clone();
  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
    .unwrap();
  Ok((id, state))
}

#[tauri::command]
pub fn get_initial_data(state: tauri::State<SharedStateWrapper>) -> (Store, NodeRegister) {
  let state = state.0.lock().unwrap();
  (
    state.stored_state.store.clone(),
    state.node_register.clone(),
  )
}

#[tauri::command]
pub fn change_clip_name(
  clip_type: String,
  id: ID,
  name: String,
  state: tauri::State<'_, SharedStateWrapper>,
  window: tauri::Window,
) -> Result<Store, String> {
  match clip_type.as_str() {
    "source" => {
      if let Some(x) = (&mut state
        .0
        .clone()
        .lock()
        .unwrap()
        .stored_state
        .store
        .clips
        .source)
        .get_mut(&id)
      {
        x.name = name;
      }
    }
    "composited" => {
      if let Some(x) = (&mut state
        .0
        .clone()
        .lock()
        .unwrap()
        .stored_state
        .store
        .clips
        .composited)
        .get_mut(&id)
      {
        x.name = name;
      }
    }
    _ => {}
  }

  let state = state.0.clone().lock().unwrap().stored_state.store.clone();
  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
    .unwrap();
  Ok(state)
}

#[tauri::command]
pub fn get_node_outputs(
  state: tauri::State<SharedStateWrapper>,
  node: Node,
) -> Result<HashMap<String, NodeTypeProperty>, String> {
  let state = state.0.lock().unwrap();
  let node_registration = state.node_register.get(&node.node_type);
  if node_registration.is_none() {
    return Err(String::from("Could not find relevant registration"));
  }
  let node_registration = node_registration.unwrap();

  let outputs = (node_registration.get_output_types)(
    node.id,
    &node.properties,
    &state.stored_state.store,
    &state.node_register,
  );

  if outputs.is_err() {
    return Err(format!(
      "Outputs could not be calculated: {}",
      outputs.unwrap_err()
    ));
  }
  let outputs = outputs.unwrap();

  Ok(outputs)
}

#[tauri::command]
pub fn update_node(state: tauri::State<SharedStateWrapper>, node: Node) -> Result<(), String> {
  let mut state = state.0.lock().unwrap();
  state
    .stored_state
    .store
    .nodes
    .insert(node.id.clone(), node.clone());
  state.stored_state.file_written = false;
  Ok(())
}
