use anyhow::{bail, Result};
use argon2::Argon2;
use ring::{rand::{SecureRandom, SystemRandom}, aead::{UnboundKey, AES_256_GCM, Nonce, Aad, LessSafeKey}, hkdf};

pub const SALT_SIZE: usize = 16;
pub const INFO_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

pub fn derive_key(password: &[u8], salt: &[u8], key: &mut [u8]) -> Result<()>{
  match Argon2::default().hash_password_into(password, salt, key) {
    Ok(_) => Ok(()),
    Err(_) => bail!("Failed to hash with Argon2")
  }
}

fn hkdf_derive_key(master_key: &[u8], info: &[u8], key: &mut [u8]) -> Result<(), ring::error::Unspecified> {
  let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]);
  let prk = salt.extract(master_key);
  prk.expand(&[info], hkdf::HKDF_SHA256)?.fill(key)?;
  Ok(())
}

pub fn generate_salt() -> Result<[u8; SALT_SIZE]> {
  let rng = SystemRandom::new();
  let mut salt = [0u8; SALT_SIZE];
  match rng.fill(&mut salt) {
    Ok(_) => Ok(salt),
    Err(_) => bail!("Failed to generate salt")
  }
}

fn generate_nonce_and_info() -> Result<(Vec<u8>, Vec<u8>), ring::error::Unspecified> {
  let rng = SystemRandom::new();

  let mut nonce = vec![0u8; NONCE_SIZE];
  rng.fill(&mut nonce)?;

  let mut info = vec![0u8; INFO_SIZE];
  rng.fill(&mut info)?;

  Ok((nonce, info))
}

fn seal(data: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, ring::error::Unspecified> {
  let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
  let aead_key = LessSafeKey::new(unbound_key);
  let mut in_out = data.to_vec();
  let aad = Aad::empty();
  let nonce = Nonce::try_assume_unique_for_key(nonce)?;
  aead_key.seal_in_place_append_tag(nonce, aad, &mut in_out)?;
  Ok(in_out)
}

fn open(data: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, ring::error::Unspecified> {
  let unbound_key = UnboundKey::new(&AES_256_GCM, key)?;
  let aead_key = LessSafeKey::new(unbound_key);
  let mut in_out = data.to_vec();
  let aad = Aad::empty();
  let nonce = Nonce::try_assume_unique_for_key(nonce)?;
  aead_key.open_in_place(nonce, aad, &mut in_out)?;
  in_out.truncate(in_out.len() - AES_256_GCM.tag_len());
  Ok(in_out)
}

fn merge_nonce_info_and_encrypted_data(nonce: &[u8], info: &[u8], encrypted_data: &[u8]) -> Vec<u8> {
  let mut merged_data = Vec::new();
  merged_data.extend(nonce);
  merged_data.extend(info);
  merged_data.extend(encrypted_data);
  merged_data
}

fn split_nonce_info_and_encrypted_data(merged_data: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
  let nonce_start = 0;
  let nonce_end = nonce_start + NONCE_SIZE;

  let info_start = nonce_end;
  let info_end = info_start + INFO_SIZE;

  let encrypted_data_start = info_end;

  if merged_data.len() < encrypted_data_start {
    bail!("Invalid info or nonce lengths")
  }

  let nonce = merged_data[nonce_start..nonce_end].to_vec();
  let salt = merged_data[info_start..info_end].to_vec();
  let encrypted_data = merged_data[encrypted_data_start..].to_vec();
  
  Ok((nonce, salt, encrypted_data))
}

pub fn encrypt(data: &[u8], master_key: &[u8]) -> Result<Vec<u8>> {
  if let Ok((nonce, info)) = generate_nonce_and_info() {

    let mut key = [0u8; KEY_SIZE];
    let derive_result = hkdf_derive_key(master_key, &info, &mut key);
    if derive_result.is_err() { bail!("Derive failed") };

    match seal(&data, &key, &nonce) {
      Ok(encrypted_data) => Ok(merge_nonce_info_and_encrypted_data(&nonce, &info, &encrypted_data)),
      Err(err) => bail!("Encryption failed: {}", err)
    }
  } else{
    bail!("Error generating nonce/info");
  }
}

pub fn decrypt(data: &[u8], master_key: &[u8]) -> Result<Vec<u8>> {
  let (nonce, info, encrypted_data) = split_nonce_info_and_encrypted_data(data)?;

  let mut key = [0u8; KEY_SIZE];
  let derive_result = hkdf_derive_key(master_key, &info, &mut key);
  if derive_result.is_err() { bail!("Derive failed") };
  match open(&encrypted_data, &key, &nonce) {
    Ok(decrypted_data) => Ok(decrypted_data),
    Err(err) => bail!("Decryption failed: {}", err)
  }
}

pub fn decrypt_as_string(data: &[u8], master_key: &[u8]) -> Result<String> {
  match decrypt(&data, &master_key) {
    Ok(decrypted_data) => {
      match String::from_utf8(decrypted_data) {
        Ok(string) => Ok(string),
        Err(err) => bail!("Error converting decrypted data to string: {}", err)
      }
    },
    Err(err) => Err(err)
  }
}