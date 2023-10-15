import { invoke } from '@tauri-apps/api';
import { NodeTree } from '../types/pad';

export async function openOrCreateCipherpad(path: string) {
  await invoke('open_or_create_cipherpad', {path});
}

export async function unlockCipherpad(password: string) {
  await invoke('unlock_cipherpad', {password});
}

export async function getNodeTree() {
  return await invoke('get_node_tree') as NodeTree;
}