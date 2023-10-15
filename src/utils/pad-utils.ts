import { createPad, decryptPad } from "../api/pad";
import { CipherpadContextType } from "../providers/CipherpadProvider";
import { EncryptedPad, Pad, PadMetadata, PadNode, SerializedEncryptedPad, SerializedPad, SerializedPadNode, TextPadData } from "../types/pad";


export const createNewPad = async (pad: Pad, {refreshCipherpadData, setCurrentPad}: CipherpadContextType) => {
  const newPadId = await createPad(pad);
  const newCipherpadState = await refreshCipherpadData();
  if (newCipherpadState !== undefined) {
    const { padMap } = newCipherpadState;
    setCurrentPad(padMap.pads[newPadId]);
  }
}

export const decryptTextPad = async (id: string): Promise<TextPadData> => {
  return await decryptPad(id) as TextPadData;
}

export const serializePad = (pad: Pad): SerializedPad => {
  return {
    ...pad,
    padData: JSON.stringify(pad.padData),
    padMetadata: JSON.stringify(pad.padMetadata)
  }
}

export const parseSerializedPad = (serializedPad: SerializedPad): Pad => {
  const padMetadata = JSON.parse(serializedPad.padMetadata) as PadMetadata;
  return {
    ...serializedPad,
    padData: JSON.parse(serializedPad.padData),
    padMetadata,
  }
}

export const serializePadNode = (padNode: PadNode): SerializedPadNode => {
  return {
    ...padNode,
    pad: serializePad(padNode.pad)
  }
}

export const parseSerializedPadNode = (serializedPadNode: SerializedPadNode): PadNode => {
  return {
    ...serializedPadNode,
    pad: parseSerializedPad(serializedPadNode.pad)
  }
}

export const serializeEncryptedPad = (encryptedPad: EncryptedPad): SerializedEncryptedPad => {
  return {
    ...encryptedPad,
    metadata: JSON.stringify(encryptedPad.metadata)
  }
}

export const parseSerializedEncryptedPad = (serializedEncryptedPad: SerializedEncryptedPad): EncryptedPad => {
  return {
    ...serializedEncryptedPad,
    metadata: JSON.parse(serializedEncryptedPad.metadata)
  }
}