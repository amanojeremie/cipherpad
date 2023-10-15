use std::{env, path::PathBuf, io::{BufReader, Read}};

use rusqlite::blob::Blob;
use tokio::fs::File;

pub const CHUNK_SIZE: usize = 4096; // Size to chunk files when encrypting them
pub async fn create_temp_file() -> Result<(PathBuf, File), anyhow::Error> {
  let mut temp_path = env::temp_dir();
  temp_path.push(format!("cipherpad_{}", uuid::Uuid::new_v4()));
  let temp_file = File::create(&temp_path).await?;
  Ok((temp_path, temp_file))
}

pub fn read_exact_chunk(reader: &mut BufReader<Blob<'_>>, buffer: &mut Vec<u8>) -> Result<usize, anyhow::Error>{
  let mut total_bytes_read = 0;
  while total_bytes_read < buffer.len() {
    let bytes_read = reader.read(&mut buffer[total_bytes_read..])?;
    if bytes_read == 0 {
      break;
    }
    total_bytes_read += bytes_read;
  }
  Ok(total_bytes_read)
 }