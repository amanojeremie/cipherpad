import { invoke } from '@tauri-apps/api';
import { EncryptedPad, Pad, PadData, PadMap, PadMetadata, PadNode, SerializedPadMap } from '../types/pad';
import { serializeEncryptedPad, serializePad, serializePadNode } from '../utils/pad-utils';

export async function getPadMap(): Promise<PadMap> {
  const serializedPadMap = await invoke('get_pad_map') as SerializedPadMap;
  const padMap: {
    [key: string]: EncryptedPad
  } = Object.entries(serializedPadMap.pads).reduce((newPadMap, [id, encryptedPad]) => {
    newPadMap[id] = {...encryptedPad, metadata: JSON.parse(encryptedPad.metadata) as PadMetadata};
    return newPadMap;
  }, {} as {
    [key: string]: EncryptedPad
  });
  return {pads: padMap};
}

export async function createPad(pad: Pad) {
  const serializedPad = serializePad(pad);
  return await invoke('create_pad', {pad: serializedPad}) as string;
}

export async function encryptFileToPad(encryptedPad: EncryptedPad, file: string) {
  const serializedEncryptedPad = serializeEncryptedPad(encryptedPad);
  return await invoke('encrypt_file_to_pad', {encryptedPad: serializedEncryptedPad, file});
}

export async function decrpytPadToFile(encryptedPad: EncryptedPad, file: string) {
  const serializedEncryptedPad = serializeEncryptedPad(encryptedPad);
  return await invoke('decrypt_pad_to_file', {encryptedPad: serializedEncryptedPad, file});
}

export async function decrpytPadToBlob(encryptedPad: EncryptedPad): Promise<{blob: Blob, mime: string}> {
  const serializedEncryptedPad = serializeEncryptedPad(encryptedPad);
  const [blobBase64, blobMime] = await invoke('decrypt_pad_to_blob', {encryptedPad: serializedEncryptedPad}) as [string, string];
  const byteCharacters = atob(blobBase64);
  const uInt8Array = new Uint8Array(byteCharacters.length);
  for (let i = 0; i < byteCharacters.length; i++) {
    uInt8Array[i] = byteCharacters.charCodeAt(i);
  }

  return {blob: new Blob([uInt8Array]), mime: blobMime};
}

export async function deletePadById(id: string) {
  return await invoke('delete_pad', {id});
}

export async function updatePad(padNode: PadNode) {
  const serializedPadNode = serializePadNode(padNode);
  await invoke('update_pad', {padNode: serializedPadNode});
}

export async function decryptPad(id: string): Promise<PadData> {
  const decryptedPadData = await invoke('decrypt_pad', {id}) as string;
  return JSON.parse(decryptedPadData) as PadData;
}