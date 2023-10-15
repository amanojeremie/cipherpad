export interface TextPadData {
  text: string,
  revisionHistory: {
    historyAt: number,
    text: string
  }[]
}

export type BlobPadData = '';

interface BasePadMetadata {
  type: 'text' | 'blob';
  name: string,
  createdAt: number,
  lastModifiedAt: number
}

export interface TextPadMetadata extends BasePadMetadata {
  type: 'text'
}

export interface BlobPadMetadata extends BasePadMetadata {
  type: 'blob',
  fileName: string,
  encryptedDataOffset: number
}

export type PadMetadata = TextPadMetadata | BlobPadMetadata;

export type PadData = TextPadData | BlobPadData;

export interface Pad {
  parentId: string | null,
  padMetadata: PadMetadata,
  padData: PadData
}

export interface SerializedPad {
  parentId: string | null,
  padMetadata: string,
  padData: string
}

export interface PadNode {
  id: string,
  pad: Pad
}

export interface SerializedPadNode {
  id: string,
  pad: SerializedPad
}

export interface Node {
  id: string,
  children: Node[]
}

export interface SerializedEncryptedPad {
  id: string,
  parentId: string | null,
  metadata: string
}

export interface EncryptedPad {
  id: string,
  parentId: string | null,
  metadata: PadMetadata
}

export interface NodeTree {
  nodes: Node[]
}

export interface SerializedPadMap {
  pads: {
    [key: string]: SerializedEncryptedPad
  }
}

export interface PadMap {
  pads: {
    [key: string]: EncryptedPad
  }
}