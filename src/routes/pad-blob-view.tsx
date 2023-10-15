import { useNavigate } from "react-router-dom";
import { useCipherpad } from "../providers/CipherpadProvider";
import { useEffect, useState } from "react";
import { decrpytPadToBlob } from "../api/pad";

export interface PadBlobViewState {
  pad: {
    blob: Blob,
    mime: string
  } | undefined
}

export default function PadBlobView() {
  const [{pad}, setPadViewState] = useState<PadBlobViewState>({pad: undefined});
  const navigate = useNavigate();
  const cipherpadContext = useCipherpad();
  const { cipherpadUiState: {currentPad} } = cipherpadContext;

  const loadBlob = async () => {
    if (currentPad !== null) {
      const decryptedBlob = await decrpytPadToBlob(currentPad);
      setPadViewState({pad: decryptedBlob});
      console.log(decryptedBlob);
    }
  };

  const getViewer = () => {
    if (pad === undefined) return <p>Decrypting...</p>;
    if (/^image/g.test(pad.mime)) {
      return <img src={URL.createObjectURL(pad.blob)} alt={currentPad?.metadata.name} />
    }
    else if (/^video/g.test(pad.mime)) {
      return <video src={URL.createObjectURL(pad.blob)} />
    }
    else {
      return <p>Unable to view. Unsupported type.</p>
    }
  }

  useEffect(() => {
    loadBlob();
  }, []);

  return (
    <div className="viewer">
      <div>
        <button onClick={() => navigate('/app')}>Back</button>
      </div>
      {getViewer()}
    </div>
  );
}