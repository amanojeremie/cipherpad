// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cipherpad;

use std::sync::Arc;
use base64::{Engine, engine::general_purpose};
use cipherpad::{Cipherpad, PadNode, NodeTree, PadMap, Pad, EncryptedPad};
use file_format::FileFormat;
use tauri::async_runtime::Mutex;
use uuid::Uuid;

#[tauri::command]
async fn open_or_create_cipherpad(
  path: String,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<bool, String> {
  let database_url_string = format!("{}", path);
  let database_url = &database_url_string.as_str();
  match Cipherpad::create_connection_pool(database_url).await {
    Ok(cipherpad) => {
      if let Err(err) = cipherpad.create_tables_if_not_exists().await {
        return Err(format!("Failed to create tables: {}", err));
      }
      let mut state = state.inner().lock().await;
      state.pool = cipherpad.pool;
      Ok(true)
    }
    Err(err) => Err(format!("Failed to create connection: {}", err))
  }
}

#[tauri::command]
async fn unlock_cipherpad(
  password: String,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<NodeTree, String> {
  let mut cipherpad = state.inner().lock().await;
  if cipherpad.is_connected() {
    if let None = cipherpad.master_key {
      let derive_key_result = cipherpad.derive_master_key(&password).await;
      if derive_key_result.is_err() {
        return Err(format!("Error deriving key"))
      }
      match cipherpad.get_node_tree().await {
        Ok(tree) => Ok(tree),
        Err(err) => {
          cipherpad.master_key = None;
          Err(format!("Error: {}", err))
        }
      }
    } else {
      Err(format!("Password has already been set"))
    }
  } else {
    Err(format!("No connection available"))
  }
}

#[tauri::command]
async fn create_pad(
  pad: Pad,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<String, String> {
  let cipherpad = state.inner().lock().await;
  let id = Uuid::new_v4();
  let pad_node = PadNode::new(id, pad);
  if let (Some(pool), Some(master_key)) = (&cipherpad.pool, &cipherpad.master_key) {
    match pad_node.create_node(pool, master_key).await {
      Ok(_) => {
        Ok(id.to_string())
      },
      Err(err) => Err(format!("Error: {}", err))
    }
  } else {
    Err(format!("No connection and/or authentication"))
  }
}

#[tauri::command]
async fn update_pad(
  pad_node: PadNode,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<(), String> {
  let cipherpad = state.inner().lock().await;
  if let (Some(pool), Some(master_key)) = (&cipherpad.pool, &cipherpad.master_key) {
    match pad_node.encrypt_and_save(pool, master_key).await {
      Ok(_) => Ok(()),
      Err(err) => Err(format!("Error saving pad: {}", err))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tauri::command]
async fn encrypt_file_to_pad(
  encrypted_pad: EncryptedPad,
  file: String,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<(), String> {
  let cipherpad = state.inner().lock().await;
  if let (Some(pool), Some(master_key)) = (&cipherpad.pool, &cipherpad.master_key) {
    match encrypted_pad.encrypt_file_to_pad(pool, master_key, &file).await {
      Ok(_) => Ok(()),
      Err(err) => Err(format!("Error saving file to pad: {}", err))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tauri::command]
async fn decrypt_pad_to_file(
  encrypted_pad: EncryptedPad,
  file: String,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<(), String> {
  let cipherpad = state.inner().lock().await;
  if let (Some(pool), Some(master_key)) = (&cipherpad.pool, &cipherpad.master_key) {
    match encrypted_pad.decrypt_pad_to_file(pool, master_key, &file).await {
      Ok(_) => Ok(()),
      Err(err) => Err(format!("Error decrypting pad to file: {}", err))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tauri::command]
async fn decrypt_pad_to_blob(
  encrypted_pad: EncryptedPad,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<(String, String), String> {
  let cipherpad = state.inner().lock().await;
  if let (Some(pool), Some(master_key)) = (&cipherpad.pool, &cipherpad.master_key) {
    match encrypted_pad.decrypt_pad_to_blob(pool, master_key).await {
      Ok(blob) => Ok((general_purpose::STANDARD.encode(&blob), FileFormat::from_bytes(&blob).media_type().to_string())),
      Err(err) => Err(format!("Error decrypting pad to blob: {}", err))
    }
  } else {
    Err(format!("No connection"))
  }
}


#[tauri::command]
async fn get_node_tree(
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<NodeTree, String> {
  let mut cipherpad = state.inner().lock().await;
  if cipherpad.is_connected() {
    match cipherpad.get_node_tree().await {
      Ok(tree) => Ok(tree.clone()),
      Err(err) => Err(format!("Error: {}", err))
    }
  } else  {
    Err(format!["No connection available"])
  }
}

#[tauri::command]
async fn get_pad_map(
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>
) -> Result<PadMap, String> {
  let cipherpad = state.inner().lock().await;
  if cipherpad.is_connected() {
    if let Some(_) = &cipherpad.master_key {
      Ok(cipherpad.pad_map.clone())
    } else {
      Err(format!("No password"))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tauri::command]
async fn decrypt_pad(
  id: Uuid,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>
) -> Result<String, String> {
  let cipherpad = state.inner().lock().await;
  if let Some(pool) = &cipherpad.pool {
    if let Some(master_key) = &cipherpad.master_key {
      if let Some(encrypted_pad) = cipherpad.pad_map.pads.get(&id) {
        match encrypted_pad.clone().decrypt_pad_data(master_key, pool).await {
          Ok(pad_data) => Ok(pad_data),
          Err(err) => Err(format!("Error decrypting pad: {}", err))
        }
      } else {
        Err(format!("No pad with that id"))
      }
    } else {
      Err(format!("No password"))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tauri::command]
async fn delete_pad(
  id: Uuid,
  state: tauri::State<'_, Arc<Mutex<Cipherpad>>>,
) -> Result<(), String> {
  let cipherpad = state.inner().lock().await;
  if let (Some(pool), Some(_)) = (&cipherpad.pool, &cipherpad.master_key) {
    if let Some(encrypted_pad) = cipherpad.pad_map.pads.clone().get(&id) {
      match encrypted_pad.clone().delete_node(pool).await {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Error deleting pad: {}", err))
      }
    } else {
      Err(format!("No pad with that id"))
    }
  } else {
    Err(format!("No connection"))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

  let cipherpad = Cipherpad::new();
  let cipherpad = Arc::new(Mutex::new(cipherpad));

  tauri::Builder::default()
    .manage(cipherpad)
    .invoke_handler(tauri::generate_handler![open_or_create_cipherpad, unlock_cipherpad, get_node_tree, get_pad_map, create_pad, update_pad, delete_pad, encrypt_file_to_pad, decrypt_pad_to_file, decrypt_pad_to_blob, decrypt_pad])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");

  Ok(())
}
    