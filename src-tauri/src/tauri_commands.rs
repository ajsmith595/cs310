use std::{fs::File, io::Write};

use rfd::AsyncFileDialog;

use crate::{
  classes::{clip::SourceClip, global::uniq_id, store::Store, ID},
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
  let file = dialog.pick_file().await;
  match file {
    None => Err(String::from("No file selected")),
    Some(path) => {
      println!("Starting return...");
      let clip = SourceClip {
        id: uniq_id(),
        name: String::from("Placeholder"),
        file_location: path.path().to_str().unwrap().to_string(),
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
      let state = state.0.clone().lock().unwrap().stored_state.store.clone();
      let mut f = File::create("state.json").unwrap();
      f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
        .unwrap();
      Ok(state)
    }
  }
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

  let state = state.0.clone().lock().unwrap().stored_state.store.clone();
  let mut f = File::create("state.json").unwrap();
  f.write_all(serde_json::ser::to_string(&state).unwrap().as_bytes())
    .unwrap();
  Ok(state)
}
