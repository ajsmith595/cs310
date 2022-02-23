use std::{collections::HashMap, fs::File, io::Write};

use rfd::AsyncFileDialog;
use uuid::Uuid;

use crate::{
  network_task_manager,
  state_manager::{ConnectionStatus, SharedStateWrapper},
};
use cs310_shared::{
  clip::{self, CompositedClip, SourceClip},
  constants::media_output_location,
  global::uniq_id,
  networking::{self},
  node::{Node, NodeTypeInput, NodeTypeOutput, PipeableType},
  nodes::NodeRegister,
  store::Store,
  ID,
};

/// Executed when the 'import' button is pressed (for source clips). Will open a file dialog for the user to choose relevant files.
#[tauri::command]
pub async fn import_media(
  state: tauri::State<'_, SharedStateWrapper>,
  window: tauri::Window,
) -> Result<HashMap<ID, SourceClip>, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let dialog = AsyncFileDialog::new().add_filter("Media", &["mp4", "mkv", "mp3"]);

  #[cfg(not(target_os = "linux"))]
  let dialog = dialog
    .set_parent(&tauri::api::dialog::window_parent(&window).expect("Could not get window parent"));

  let file = dialog.pick_files().await;
  match file {
    None => Err(String::from("No file selected")),
    Some(paths) => {
      let mut hm = HashMap::new();
      let mut jobs = Vec::new();
      for path in paths {
        let file_path = path.path().to_str().unwrap().to_string();

        let info = SourceClip::get_file_info(file_path.clone());
        if info.is_err() {
          return Err(format!(
            "Could not get info for file: {}",
            info.unwrap_err()
          ));
        }
        let info = info.unwrap();
        let id = Uuid::new_v4(); // temporarily, we create a random file ID

        let clip = SourceClip {
          id: id.clone(),
          name: path.file_name(),
          original_file_location: Some(file_path.clone()),
          info: Some(info),
          status: clip::SourceClipServerStatus::NeedsNewID,
          original_device_id: None,
          file_location: None,
          thumbnail_location: None,
        };

        hm.insert(clip.id.clone(), clip.clone());
        jobs.push(network_task_manager::Task::GetSourceClipID(id.clone()))
      }

      let mut lock = state.0.lock().unwrap();
      lock.network_jobs.append(&mut jobs);
      lock.store.as_mut().unwrap().clips.source.extend(hm.clone());

      Ok(hm)
    }
  }
}

#[tauri::command]
pub fn get_initial_data(
  state: tauri::State<SharedStateWrapper>,
) -> Result<(Store, NodeRegister), String> {
  let state = state.0.lock().unwrap();
  if state.store.is_none() {
    Err(format!("Store is not yet set"))
  } else {
    Ok((state.store.clone().unwrap(), state.node_register.clone()))
  }
}

#[tauri::command]
pub fn get_node_outputs(
  state: tauri::State<SharedStateWrapper>,
  node: Node,
) -> Result<HashMap<String, NodeTypeOutput>, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let state = state.0.lock().unwrap();
  let res = state.store.as_ref().unwrap().pipeline.gen_graph_new(
    state.store.as_ref().unwrap(),
    &state.node_register,
    false,
  );
  if res.is_err() {
    return Err(format!("Could not get result!: {}", res.unwrap_err()));
  }
  let (node_type_data, _, _) = res.unwrap();

  let data = node_type_data.get(&node.id);

  if data.is_none() {
    return Err(format!("Data for node not found"));
  }
  let (_, _, outputs) = data.unwrap();

  return Ok(outputs.clone());
}

#[tauri::command]
pub fn get_node_inputs(
  state: tauri::State<SharedStateWrapper>,
  node: Node,
) -> Result<HashMap<String, NodeTypeInput>, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let state = state.0.lock().unwrap();
  let res = state.store.as_ref().unwrap().pipeline.gen_graph_new(
    &state.store.as_ref().unwrap(),
    &state.node_register,
    false,
  );
  if res.is_err() {
    return Err(format!("Could not get result!: {}", res.unwrap_err()));
  }
  let (node_type_data, _, _) = res.unwrap();

  let data = node_type_data.get(&node.id);

  if data.is_none() {
    return Err(format!("Data for node not found"));
  }
  let (_, inputs, _) = data.unwrap();

  return Ok(inputs.clone());
}

#[tauri::command]
pub fn store_update(state: tauri::State<SharedStateWrapper>, store: Store) -> Result<(), String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }
  println!("Updating store...");
  let mut state = state.0.lock().unwrap();
  state.store = Some(store.clone());
  state.file_written = false;
  println!("Store updated");

  Ok(())
}

#[tauri::command]
pub fn get_output_directory() -> String {
  media_output_location()
}

#[tauri::command]
pub fn get_clip_type(
  state: tauri::State<SharedStateWrapper>,
  clip_type: String,
  id: ID,
) -> Result<PipeableType, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let state = state.0.lock().unwrap();
  match clip_type.as_str() {
    "source" => {
      let clip = state.store.as_ref().unwrap().clips.source.get(&id).unwrap();
      return Ok(clip.get_clip_type());
    }
    "composited" => {
      let res = state.store.as_ref().unwrap().pipeline.gen_graph_new(
        state.store.as_ref().unwrap(),
        &state.node_register,
        false,
      );

      if let Ok((_, clip_data, _)) = res {
        if let Some(piped_type) = clip_data.get(&id) {
          return Ok(piped_type.stream_type);
        }
      }
    }
    _ => todo!(),
  }

  Ok(PipeableType {
    audio: 0,
    video: 0,
    subtitles: 0,
  })
}

#[tauri::command]
pub fn get_connection_status(state: tauri::State<SharedStateWrapper>) -> ConnectionStatus {
  state.0.lock().unwrap().connection_status.clone()
}
