use std::collections::HashMap;

use rfd::AsyncFileDialog;
use uuid::Uuid;

use crate::{
  network_task_manager,
  state_manager::{ConnectionStatus, SharedStateWrapper},
  task_manager::Task,
};
use cs310_shared::{
  clip::{self, ClipType, CompositedClip, SourceClip},
  constants::media_output_location,
  node::{Node, NodeTypeInput, NodeTypeOutput, PipeableType},
  nodes::NodeRegister,
  pipeline::Link,
  store::Store,
  ID,
};

/// Executed when the 'import' button is pressed (for source clips). Will open a file dialog for the user to choose relevant files.
#[tauri::command]
pub async fn import_media(
  state: tauri::State<'_, SharedStateWrapper>,
  window: tauri::Window,
) -> Result<HashMap<ID, SourceClip>, String> {
  println!("Importing media...");
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
      // let mut jobs = Vec::new();

      let mut tasks = Vec::new();
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
        let task = Task::CreateSourceClip(clip);
        tasks.push(task);
      }

      let mut lock = state.0.lock().unwrap();
      lock.tasks.append(&mut tasks);
      lock
        .task_manager_notifier
        .as_ref()
        .unwrap()
        .send(true)
        .unwrap();
      Ok(hm)
    }
  }
}

/// Executed when the page loads and requests the initial state data for the React UI
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

/// Gets a hashmap containing the different outputs of the targeted node
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

/// Gets a hashmap containing the different inputs of the targeted node
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
pub fn get_output_directory() -> String {
  media_output_location()
}

/// Returns the type of the targeted clip i.e. the different tracks
#[tauri::command]
pub fn get_clip_type(
  state: tauri::State<SharedStateWrapper>,
  clip_type: ClipType,
  id: ID,
) -> Result<PipeableType, String> {
  {
    let lock = state.0.lock().unwrap();
    if lock.store.is_none() {
      return Err(format!("Store is not yet set"));
    }
  }

  let state = state.0.lock().unwrap();
  match clip_type {
    ClipType::Source => {
      let clip = state.store.as_ref().unwrap().clips.source.get(&id).unwrap();
      return Ok(clip.get_clip_type());
    }
    ClipType::Composited => {
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

#[tauri::command]
pub fn create_composited_clip(state: tauri::State<SharedStateWrapper>) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::CreateCompositedClip(CompositedClip {
    id: Uuid::new_v4(),
    name: String::from("New Composited Clip"),
  }));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}

#[tauri::command]
pub fn add_link(state: tauri::State<SharedStateWrapper>, link: Link) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::AddLink(link));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}

#[tauri::command]
pub fn update_node(state: tauri::State<SharedStateWrapper>, node: Node) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::UpdateNode(node.id.clone(), node));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}

#[tauri::command]
pub fn add_node(state: tauri::State<SharedStateWrapper>, node: Node) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::AddNode(node));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}

#[tauri::command]
pub fn delete_node(state: tauri::State<SharedStateWrapper>, id: Uuid) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::DeleteNode(id));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}

#[tauri::command]
pub fn delete_links(
  state: tauri::State<SharedStateWrapper>,
  node_id: Uuid,
  property: Option<String>,
) {
  let mut lock = state.0.lock().unwrap();
  lock.tasks.push(Task::DeleteLinks(node_id, property));

  lock
    .task_manager_notifier
    .as_ref()
    .unwrap()
    .send(true)
    .unwrap();
}
