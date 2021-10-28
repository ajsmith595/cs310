use std::{fs::File, io::Write};

use rfd::AsyncFileDialog;

use crate::{
  classes::{
    clip::{CompositedClip, SourceClip},
    global::uniq_id,
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
pub fn get_initial_data(state: tauri::State<SharedStateWrapper>) -> Store {
  state.0.lock().unwrap().stored_state.store.clone()
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
