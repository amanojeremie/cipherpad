use std::{collections::{HashMap, HashSet}, cell::RefCell, rc::Rc, io::{Read, Write}};
use anyhow::{bail, Context};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use tokio::{fs::{File, self}, io::{BufReader, AsyncReadExt, BufWriter, AsyncWriteExt}};
use uuid::Uuid;

use crate::cipherpad::utils::read_exact_chunk;

use self::{db::{DatabasePool, SqlParamsBuilder, value_from_sql}, utils::{create_temp_file, CHUNK_SIZE}, crypto::{KEY_SIZE, SALT_SIZE}};

mod crypto;
mod db;
mod utils;

const MAX_BLOB_SIZE: usize = 1_000_000_000;

pub struct Cipherpad {
  pub pool: Option<DatabasePool>,
  pub node_tree: NodeTree,
  pub pad_map: PadMap,
  pub master_key: Option<[u8; KEY_SIZE]>
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeTree {
  pub nodes: Vec<Node>,
}

impl NodeTree {
  fn new() -> Self {
    Self {
      nodes: Vec::new()
    }
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PadMap {
  pub pads: HashMap<Uuid, EncryptedPad>
}

impl PadMap {
  fn new() -> Self {
    Self {
      pads: HashMap::new()
    }
  }
}

#[derive(Clone, Serialize, Debug)]
pub struct Node {
  pub id: Uuid,
  pub children: Vec<Node>,
}

impl Node {
  fn new(id: Uuid) -> Self {
    Self {
      id,
      children: Vec::new(),
    }
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptedPad {
  pub id: Uuid,
  #[serde(rename = "parentId")]
  pub parent_id: Option<Uuid>,
  #[serde(rename = "metadata")]
  pub metadata: String
}

impl EncryptedPad {
  pub fn new(id: Uuid, parent_id: Option<Uuid>, pad_metadata_encrypted: Vec<u8>, master_key: &[u8]) -> Result<Self, anyhow::Error> {
    let metadata = crypto::decrypt_as_string(&pad_metadata_encrypted, master_key)?;
    Ok(Self {
      id,
      parent_id,
      metadata
    })
  }


  async fn select_encrypted_data(self, pool: &DatabasePool) -> Result<Vec<u8>, anyhow::Error> {
    let data_select_result = pool.select_query_single(
      "SELECT pad_data FROM NODE WHERE id = ?1",
      SqlParamsBuilder::new()
        .add_param(self.id)
        .build(),
        1
    ).await?;
    let encrypted_data_vec = value_from_sql::<Vec<u8>>(data_select_result.get(0))?;
    Ok(encrypted_data_vec)
  }

  pub async fn decrypt_pad_data(self, master_key: &[u8], pool: &DatabasePool) -> Result<String, anyhow::Error> {

    match crypto::decrypt_as_string(&self.select_encrypted_data(pool).await?, master_key) {
      Ok(pad_data) => {
        Ok(pad_data)
      },
      Err(err) => bail!("Error: {}", err)
    }
  }

  pub async fn delete_node(self, pool: &DatabasePool) -> Result<(), anyhow::Error> {
    pool.execute_query("DELETE FROM node WHERE id = ?1", 
      SqlParamsBuilder::new().add_param(self.id).build()
    ).await?;
    Ok(())
  }


  pub fn get_blob_pad_metadata(self) -> Result<BlobPadMetadata, anyhow::Error> {
    let blob_pad_metadata = serde_json::from_str(&self.metadata)?;
    Ok(blob_pad_metadata)
  }

  pub async fn clear_pad_data(self, pool: &DatabasePool) -> Result<(), anyhow::Error> {
    pool.execute_query("UPDATE node SET pad_data = ZEROBLOB(0) WHERE id = ?1",
    SqlParamsBuilder::new()
      .add_param(self.id).build()
    ).await?;
    Ok(())
  }

  pub async fn encrypt_and_append_pad_data(self, file_writer: &mut BufWriter<File>, master_key: &[u8], data: &[u8]) -> Result<usize, anyhow::Error> {
    let encrypted_data = crypto::encrypt(data, master_key)?;
    let encrypted_data_len = encrypted_data.len();
    file_writer.write(&encrypted_data).await?;
    Ok(encrypted_data_len)
  }

  pub async fn encrypt_file_to_pad(self, pool: &DatabasePool, master_key: &[u8], file: &str) -> Result<(), anyhow::Error> {
    self.clone().clear_pad_data(pool).await?;
    let select_row_id_result = pool.select_query_single(
      "SELECT rowid FROM node WHERE id = ?1",
      SqlParamsBuilder::new()
        .add_param(self.id)
        .build(),
      1
    ).await?;
    let row_id = value_from_sql::<i64>(select_row_id_result.get(0))?;
    let file = File::open(file).await?;
    let mut reader = BufReader::new(file);
    let mut encrypted_chunk_sizes = Vec::<u8>::new();
    let mut buffer = [0u8; CHUNK_SIZE];
    let (temp_path, temp_file) = create_temp_file().await?;
    let mut temp_file_writer = BufWriter::new(temp_file);
    loop {
      let bytes_read = reader.read(&mut buffer[..]).await?;
      if bytes_read == 0 {
        break;
      }
      let encrypted_data_len = self.clone().encrypt_and_append_pad_data(&mut temp_file_writer, master_key, &buffer[..bytes_read]).await?;
      encrypted_chunk_sizes.extend(encrypted_data_len.to_be_bytes());
    }
    temp_file_writer.flush().await?;
    let temp_file_metadata = fs::metadata(&temp_path).await?;
    let encrypted_encrypted_chunk_sizes = crypto::encrypt(&encrypted_chunk_sizes, master_key)?;
    let mut blob_pad_metadata = self.clone().get_blob_pad_metadata()?;
    blob_pad_metadata.encrypted_data_offset = encrypted_encrypted_chunk_sizes.len();
    let blob_pad_metadata = serde_json::to_string(&blob_pad_metadata)?;
    let encrypted_blob_pad_metadata = crypto::encrypt(blob_pad_metadata.as_bytes(), master_key)?;

    if encrypted_encrypted_chunk_sizes.len() + temp_file_metadata.len() as usize >= MAX_BLOB_SIZE {
      bail!("File is too large to store into Cipherpad.")
    }

    pool.execute_query("UPDATE node \
      SET pad_metadata = ?1, \
      pad_data = ZEROBLOB(?2)
      WHERE id = ?3",
      SqlParamsBuilder::new()
      .add_param(encrypted_blob_pad_metadata)
      .add_param(encrypted_encrypted_chunk_sizes.len() + temp_file_metadata.len() as usize)
      .add_param(self.id)
      .build()
    ).await?;

    pool.open_blob(row_id, "pad_data", "node", false, move |blob| {
      let mut blob_writer = std::io::BufWriter::new(blob);
      blob_writer.write(&encrypted_encrypted_chunk_sizes)?;
      let std_fs_tmp_file = std::fs::File::open(&temp_path)?;
      let mut file_reader = std::io::BufReader::new(std_fs_tmp_file);

      let mut buffer = [0u8; CHUNK_SIZE];
      loop {
        let bytes_read = file_reader.read(&mut buffer)?;
        if bytes_read == 0 {
          break;
        }
        blob_writer.write(&buffer[..bytes_read])?;
      }
      blob_writer.flush()?;
      std::fs::remove_file(&temp_path)?;
      Ok(())
    }).await?;
    
    Ok(())
  }

  pub async fn decrypt_pad_to_file(self, pool: &DatabasePool, master_key: &[u8], file_to_create: &str) -> Result<(), anyhow::Error> {
    let select_row_id_result = pool.select_query_single(
      "SELECT rowid FROM node WHERE id = ?1",
      SqlParamsBuilder::new()
        .add_param(self.id)
        .build(),
        1).await?;
    let master_key = master_key.to_vec();
    let file_to_create = file_to_create.to_string();
    let row_id = value_from_sql::<i64>(select_row_id_result.get(0))?;
    let blob_pad_metadata = self.clone().get_blob_pad_metadata()?;
    pool.open_blob(row_id, "pad_data", "node", true, move |blob| {
      let mut reader = std::io::BufReader::new(blob);
      let mut encrypted_encrypted_chunk_sizes = vec![0u8; blob_pad_metadata.encrypted_data_offset];
      reader.read(&mut encrypted_encrypted_chunk_sizes)?;
      let encrypted_chunk_sizes_bytes = crypto::decrypt(&encrypted_encrypted_chunk_sizes, &master_key)?;
      let mut encrpyted_chunk_sizes = Vec::<usize>::new();
      for chunk in encrypted_chunk_sizes_bytes.chunks_exact(8) {
        let chunk_size = usize::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]]);
        encrpyted_chunk_sizes.push(chunk_size);
      }
      let file = std::fs::File::create(file_to_create)?;
      let mut file_writer = std::io::BufWriter::new(file);
      for encrypted_chunk_size in encrpyted_chunk_sizes {
        let mut encrypted_chunk = vec![0u8; encrypted_chunk_size];
        read_exact_chunk(&mut reader, &mut encrypted_chunk)?;
        let chunk = crypto::decrypt(&encrypted_chunk, &master_key)?;
        file_writer.write(&chunk)?;
      }
      Ok(())
    }).await?;
    Ok(())
  }

  pub async fn decrypt_pad_to_blob(self, pool: &DatabasePool, master_key: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
    let select_row_id_result = pool.select_query_single(
      "SELECT rowid FROM node WHERE id = ?1",
      SqlParamsBuilder::new()
        .add_param(self.id)
        .build(),
        1).await?;
    let master_key = master_key.to_vec();
    let row_id = value_from_sql::<i64>(select_row_id_result.get(0))?;
    let blob_pad_metadata = self.clone().get_blob_pad_metadata()?;
    let blob = pool.open_blob(row_id, "pad_data", "node", true, move |blob| {
      let mut reader = std::io::BufReader::new(blob);
      let mut encrypted_encrypted_chunk_sizes = vec![0u8; blob_pad_metadata.encrypted_data_offset];
      reader.read(&mut encrypted_encrypted_chunk_sizes)?;
      let encrypted_chunk_sizes_bytes = crypto::decrypt(&encrypted_encrypted_chunk_sizes, &master_key)?;
      let mut encrpyted_chunk_sizes = Vec::<usize>::new();
      for chunk in encrypted_chunk_sizes_bytes.chunks_exact(8) {
        let chunk_size = usize::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]]);
        encrpyted_chunk_sizes.push(chunk_size);
      }
      let mut blob = Vec::<u8>::new();
      for encrypted_chunk_size in encrpyted_chunk_sizes {
        let mut encrypted_chunk = vec![0u8; encrypted_chunk_size];
        read_exact_chunk(&mut reader, &mut encrypted_chunk)?;
        let chunk = crypto::decrypt(&encrypted_chunk, &master_key)?;
        blob.extend(chunk);
      }
      Ok(blob)
    }).await?;
    Ok(blob)
  }

}

#[derive(Clone, Deserialize)]
pub struct PadNode {
  id: Uuid,
  pad: Pad
}

#[derive(Clone, Deserialize)]
pub struct Pad {
  #[serde(rename = "parentId")]
  parent_id: Option<Uuid>,
  #[serde(rename = "padMetadata")]
  pad_metadata: String,
  #[serde(rename = "padData")]
  pad_data: String
}

#[derive(Clone, Deserialize, Serialize)]
pub struct BlobPadMetadata {
  #[serde(rename = "type")]
  _type: String,
  name: String,
  #[serde(rename = "createdAt")]
  created_at: Value,
  #[serde(rename = "lastModifiedAt")]
  last_modified_at: Value,
  #[serde(rename = "fileName")]
  file_name: String,
  #[serde(rename = "encryptedDataOffset")]
  encrypted_data_offset: usize
}

impl PadNode {
  pub fn new(id: Uuid, pad: Pad) -> Self {
    Self {
      id,
      pad
    }
  }

  pub async fn encrypt_and_save(self, pool: &DatabasePool, master_key: &[u8]) -> Result<(), anyhow::Error> { 
    let encrypted_pad_metadata = crypto::encrypt(self.pad.pad_metadata.as_bytes(), master_key)?;
    let encrypted_pad_data = crypto::encrypt(self.pad.pad_data.as_bytes(), master_key)?;
    if let Some(parent_id) = self.pad.parent_id {
      pool.execute_query(
        "UPDATE node \
        SET parent_id = ?1, \
        pad_metadata = ?2, \
        pad_data = ?3 \
        WHERE id = ?4;",
        SqlParamsBuilder::new()
        .add_param(parent_id)
        .add_param(encrypted_pad_metadata)
        .add_param(encrypted_pad_data)
        .add_param(self.id)
        .build()
      ).await?;
    } else { 

      pool.execute_query(
        "UPDATE node \
        SET pad_metadata = ?1, \
        pad_data = ?2 \
        WHERE id = ?3;",
        SqlParamsBuilder::new()
        .add_param(encrypted_pad_metadata)
        .add_param(encrypted_pad_data)
        .add_param(self.id)
        .build()
      ).await?;
    }
    Ok(())
  }
  
  pub async fn create_node(self, pool: &DatabasePool, master_key: &[u8]) -> Result<(), anyhow::Error> {
    let encrypted_pad_metadata = crypto::encrypt(self.pad.pad_metadata.as_bytes(), master_key)?;
    let encrypted_pad_data = crypto::encrypt(self.pad.pad_data.as_bytes(), master_key)?;

    pool.execute_query(
      "INSERT INTO node (id, parent_id, pad_metadata, pad_data) VALUES (?, ?, ?, ?)",
      SqlParamsBuilder::new()
      .add_param(self.id)
      .add_param(self.pad.parent_id)
      .add_param(encrypted_pad_metadata)
      .add_param(encrypted_pad_data)
      .build()
    ).await?;
    Ok(())
  }
}

fn populate_node_map(
  node_map: &mut HashMap<Uuid, Rc<RefCell<Node>>>,
  node_db_map: &HashMap<Uuid, Uuid>,
  root_nodes: &HashSet<Uuid>
) {
  let mut process_stack = Vec::new();
  let mut postponed_stack = Vec::new();
  
  for &root_id in root_nodes {
    if let Some(root_node) = node_map.get(&root_id) {
      process_stack.push(root_node.clone());
    }
  }
  
  while let Some(current_node) = process_stack.pop() {
    let current_node_id = current_node.borrow().id;
  
    postponed_stack.push(current_node.clone());
  
    for (&child_id, &_parent_id) in node_db_map.iter().filter(|&(_cid, &pid)| pid == current_node_id) {
      if let Some(child_node) = node_map.get(&child_id) {
        process_stack.push(child_node.clone());
      }
    }
  }
  
  while let Some(child_node) = postponed_stack.pop() {
    let child_node_id = child_node.borrow().id;
    if let Some(&parent_id) = node_db_map.get(&child_node_id) {
      if let Some(parent_node) = node_map.get(&parent_id) {
        parent_node.borrow_mut().children.push(child_node.borrow().clone());
      }
    }
  }
}

impl Cipherpad {
  pub fn new() -> Self {
    Self {
      pool: None, 
      pad_map: PadMap::new(),
      node_tree: NodeTree::new(),
      master_key: None
    }
  }

  pub async fn create_connection_pool(database_url: &str) -> Result<Self, anyhow::Error> {
  
    let pool = DatabasePool::new(database_url)?;
    pool.execute_query("PRAGMA foreign_keys = ON;", vec![]).await?;
    pool.execute_query("PRAGMA auto_vacuum = FULL;", vec![]).await?;
    Ok(Self {
      pool: Some(pool),
      pad_map: PadMap::new(),
      node_tree: NodeTree::new(),
      master_key: None
    })
  }

  pub async fn create_tables_if_not_exists(&self) -> Result<(), anyhow::Error> {
    if let Some(ref pool) = self.pool {
      pool.begin_transaction().await?;
      pool.execute_query(
        "CREATE TABLE IF NOT EXISTS node ( \
          id TEXT PRIMARY KEY, \
          parent_id TEXT, \
          pad_metadata BLOB NOT NULL, \
          pad_data BLOB NOT NULL, \
          FOREIGN KEY (parent_id) REFERENCES node (id) ON DELETE CASCADE \
        );", vec![]
      ).await?;
      pool.execute_query(
        "CREATE TABLE IF NOT EXISTS cipherpad ( \
          id INTEGER PRIMARY KEY CHECK(id = 1), \
          master_key_salt BLOB
        );", vec![]
      ).await?;
      pool.commit().await?;
    }
    Ok(()) 
  }

  pub async fn derive_master_key(&mut self, password: &str) -> Result<(), anyhow::Error> {
    if let Some(ref pool) = self.pool {
      let select_master_key_result = pool.select_query_single("SELECT master_key_salt FROM cipherpad WHERE id = 1;",
        SqlParamsBuilder::new().build(),
        1
      ).await;
      
      let salt = match select_master_key_result {
        Ok(salt_result) => {
          let salt = value_from_sql::<[u8; SALT_SIZE]>(salt_result.get(0))?;
          Ok::<[u8; SALT_SIZE], anyhow::Error>(salt)
        },
        Err(_) => {
          let salt = crypto::generate_salt()?;
          pool.execute_query("INSERT INTO cipherpad (master_key_salt) VALUES (?1);",
            SqlParamsBuilder::new()
            .add_param(salt)
            .build()
          ).await?;
          Ok(salt)
        }
      }?;

      let mut master_key = [0u8; KEY_SIZE];

      crypto::derive_key(password.as_bytes(), &salt, &mut master_key)?;
      self.master_key = Some(master_key);
      Ok(())
    }
    else {
      bail!("No connection")
    }
  }

  pub async fn get_node_tree(&mut self) -> Result<NodeTree, anyhow::Error> {
    if let Some(ref pool) = self.pool {
      if let Some(master_key) = &self.master_key {
        let nodes = pool.select_query(
          "SELECT id, parent_id, pad_metadata, pad_data FROM node",
          vec![],
          3
        ).await?;
        let mut node_tree = NodeTree::new();
        let mut node_map = HashMap::new();
        let mut node_db_map = HashMap::new();
        let mut root_nodes = HashSet::new();
        self.pad_map.pads.clear();
        for node in nodes {
          let id = value_from_sql::<Uuid>(node.get(0)).context("Failed to read Uuid id")?;
          let parent_id = value_from_sql::<Option<Uuid>>(node.get(1)).context("Failed to read Uuid id")?;

          let pad_metadata_encrypted = value_from_sql::<Vec<u8>>(node.get(2)).context("Failed to read Vec metadata")?;

          let encrypted_pad = EncryptedPad::new(id, parent_id, pad_metadata_encrypted, master_key)?;
          self.pad_map.pads.insert(id, encrypted_pad);

          if let Some(parent_id) = parent_id {
            node_db_map.insert(id, parent_id);
          } else {
            root_nodes.insert(id);
          }

          let node = Rc::new(RefCell::new(Node::new(id)));
          node_map.insert(id, node);
        }

        populate_node_map(&mut node_map, &node_db_map, &root_nodes);

        node_tree.nodes = root_nodes
          .into_iter()
          .filter_map(|node_id| node_map.get(&node_id))
          .map(|node| node.borrow().clone())
          .collect();

        Ok(node_tree)
      } else {
        bail!("No password supplied");
      }
    } else {
      bail!("No connection")
    }
  }

  pub fn is_connected(&self) -> bool {

    self.pool.is_some()

  }
}