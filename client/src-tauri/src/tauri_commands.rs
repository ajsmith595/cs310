use std::{collections::HashMap, fs::File, io::Write};

use rfd::AsyncFileDialog;
use uuid::Uuid;

use crate::state_manager::{ConnectionStatus, SharedStateWrapper};
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

        println!("Sending file: {}", file_path.clone());
        let stream = networking::connect_to_server();

        if stream.is_err() {
          return Err(format!(
            "Could not connect to server: {}",
            stream.unwrap_err()
          ));
        };
        let mut stream = stream.unwrap();
        networking::send_message(&mut stream, networking::Message::GetFileID).unwrap();
        let temp = networking::receive_data(&mut stream, 16).unwrap();
        let mut uuid_bytes = [0 as u8; 16];
        uuid_bytes.copy_from_slice(&temp);
        let id = Uuid::from_bytes(uuid_bytes);

        //        let mut thumbnail = None;
        let x = format!(
          "{}/thumbnails/source",
          std::env::current_dir().unwrap().to_str().unwrap()
        );
        std::fs::create_dir_all(x).unwrap();

        // let source_type = info.to_pipeable_type();
        // if source_type.video > 0 {
        //   Pipeline::get_video_thumbnail(file_path.clone(), id.to_string());
        // } else if source_type.audio > 0 {
        //   Pipeline::get_audio_thumbnail(file_path.clone(), id.to_string());
        // }

        let clip = SourceClip {
          id,
          name: path.file_name(),
          original_file_location: Some(file_path.clone()),
          original_device_id: None,
          file_location: None,
          thumbnail_location: None,
          info: Some(info),
          status: clip::SourceClipServerStatus::LocalOnly,
        };

        hm.insert(clip.id.clone(), clip.clone());

        (&mut state
          .0
          .clone()
          .lock()
          .unwrap()
          .store
          .as_mut()
          .unwrap()
          .clips
          .source)
          .insert(clip.id.clone(), clip.clone());
      }
      Ok(hm)
    }
  }
}

#[tauri::command]
pub fn create_composited_clip(
  state: tauri::State<'_, SharedStateWrapper>,
) -> Result<(ID, Store), String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let clip = CompositedClip {
    id: uniq_id(),
    name: "New Clip".to_string(),
  };
  let id = clip.id.clone();
  (&mut state
    .0
    .clone()
    .lock()
    .unwrap()
    .store
    .as_mut()
    .unwrap()
    .clips
    .composited)
    .insert(clip.id.clone(), clip);

  let state = state.0.clone().lock().unwrap().store.clone().unwrap();
  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
    .unwrap();
  Ok((id, state))
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
pub fn change_clip_name(
  clip_type: String,
  id: ID,
  name: String,
  state: tauri::State<'_, SharedStateWrapper>,
) -> Result<Store, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  match clip_type.as_str() {
    "source" => {
      if let Some(x) = (&mut state
        .0
        .clone()
        .lock()
        .unwrap()
        .store
        .as_mut()
        .unwrap()
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
        .store
        .as_mut()
        .unwrap()
        .clips
        .composited)
        .get_mut(&id)
      {
        x.name = name;
      }
    }
    _ => {}
  }

  let store = state.0.clone().lock().unwrap().store.clone().unwrap();
  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&store).unwrap().as_bytes())
    .unwrap();
  Ok(store)
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
pub fn update_node(state: tauri::State<SharedStateWrapper>, node: Node) -> Result<(), String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let mut state = state.0.lock().unwrap();
  state
    .store
    .as_mut()
    .unwrap()
    .nodes
    .insert(node.id.clone(), node.clone());
  state.file_written = false;
  Ok(())
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
