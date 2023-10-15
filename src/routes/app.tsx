import { open, save } from '@tauri-apps/api/dialog';
import { useEffect, useState } from "react"
import { useCipherpad } from "../providers/CipherpadProvider"
import { useNavigate } from "react-router-dom";
import Button from "react-bootstrap/Button";
import Container from "react-bootstrap/Container";
import Modal from "react-bootstrap/Modal";
import Table from "react-bootstrap/Table";
import { createPad, decrpytPadToFile, deletePadById, encryptFileToPad } from "../api/pad";
import { EncryptedPad, Pad } from '../types/pad';

export default function App() {
  const {cipherpadState: { padMap }, cipherpadUiState: {loading, currentNodeChildren, currentNode, parentNode}, refreshCipherpadData, setCurrentNode, setCurrentPad} = useCipherpad();
  const [deleteId, setDeleteId] = useState<string | undefined>(undefined);
  const [uploadingId, setUploadingId] = useState<string | undefined>(undefined);
  const [lastError, setLastError] = useState<string | undefined>(undefined);
  const [selectedBlobPad, setSelectedBlobPad] = useState<EncryptedPad | undefined>(undefined);
  const navigate = useNavigate();

  const onCreateButtonClicked = () => {
    setCurrentPad(null);
    navigate(`/pad-edit`);
  };

  const onUploadButtonClicked = async () => {
    try {
      const fileToUpload = await open();
      if (fileToUpload !== null && !Array.isArray(fileToUpload)) {
        const fileName = fileToUpload.split(/[\\/]/).pop();
        if (fileName !== undefined) {
          const newBlobPad: Pad = {
            parentId: parentNode,
            padMetadata: {type: 'blob', name: fileName, fileName, encryptedDataOffset: 0, createdAt: Date.now(), lastModifiedAt: Date.now()},
            padData: ''
          };
    
          const id = await createPad(newBlobPad);
          setUploadingId(id);
          try {
            await refreshCipherpadData();
            await encryptFileToPad({id, parentId: newBlobPad.parentId, metadata: newBlobPad.padMetadata}, fileToUpload);
          }
          catch (e) {
            await deletePadById(id);
            throw e;
          }
          await refreshCipherpadData();
        }
      }
    }
    catch (e) {
      setLastError(e instanceof Error ? e.message : String(e));
    }
    setUploadingId(undefined);
  }

  const downloadPadToFile = async (pad: EncryptedPad) => {
    try {
      if (pad.metadata.type == 'blob') {
        const padToTrySave = await save({
          defaultPath: pad.metadata.fileName
        });
        if (padToTrySave !== null) {
          await decrpytPadToFile(pad, padToTrySave);
        }
      }
      setSelectedBlobPad(undefined);
    }
    catch (e) {
      setLastError(e instanceof Error ? e.message : String(e));
    }
  }

  const viewBlobPad = (pad: EncryptedPad) => {
    if (pad.metadata.type == 'blob') {
      setCurrentPad(pad);
      navigate('/pad-blob-view');
    }
  }

  const onDeleteConfirmed = async () => {
    try {
      if (deleteId !== undefined) {
        await deletePadById(deleteId);
        setDeleteId(undefined);
        await refreshCipherpadData();
      }
    }
    catch (e) {
      setLastError(e instanceof Error ? e.message : String(e));
    }
  }

  useEffect(() => {
    refreshCipherpadData();
  }, [lastError]);

  if (loading) return (<p>Loading...</p>);
  return (
    <Container>
      <h1>Cipherpad</h1>
      <Button variant="secondary" role="link" size="sm" onClick={onCreateButtonClicked}>Create</Button>{' '}
      <Button variant="secondary" role="link" size="sm" onClick={onUploadButtonClicked}>Upload</Button>{' '}
      {currentNode !== null && <Button variant="secondary" role="link" size="sm" onClick={() => {setCurrentNode(parentNode)}}>Up</Button>}
      <Table bordered size="sm">
        <tbody>
          {currentNodeChildren.map((encryptedPad) => {
            return <tr key={encryptedPad.id}>
              <td className="text-center" colSpan={2} >
                <Button variant="secondary" role="link" onClick={() => {
                  setCurrentPad(encryptedPad);
                  switch (encryptedPad.metadata.type) {
                    case 'text':
                      navigate('/pad-edit');
                      break;
                    case 'blob':
                      setSelectedBlobPad(encryptedPad);
                      break;
                  }
                }}>
                  {encryptedPad.metadata.name}{uploadingId === encryptedPad.id && <span>{' '}Uploading...</span>}
                </Button>
              </td>
              <td className="text-center">
                <Button variant="secondary" role="link" onClick={() => {
                  setCurrentNode(encryptedPad.id);
                }}>
                  /
                </Button>
              </td>
              <td className="text-center">
                <Button variant="secondary" onClick={() => {
                  setDeleteId(encryptedPad.id);
                }}>
                  Delete
                </Button>
              </td>
            </tr>
          })}
        </tbody>
      </Table>
      <Modal show={deleteId !== undefined} onHide={() => {setDeleteId(undefined)}}>
        <Modal.Header closeButton>
          <Modal.Title>Delete pad?</Modal.Title>
        </Modal.Header>
        <Modal.Body>Are you sure you want to delete{' '}{deleteId !== undefined && padMap.pads[deleteId].metadata.name}?</Modal.Body>
        <Modal.Footer>
          <Button variant="secondary" onClick={() => setDeleteId(undefined)}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onDeleteConfirmed}>
            Delete
          </Button>
        </Modal.Footer>
      </Modal>
      <Modal show={selectedBlobPad !== undefined} onHide={() => {setSelectedBlobPad(undefined)}}>
        <Modal.Header closeButton>
          <Modal.Title>Selected: {selectedBlobPad !== undefined && selectedBlobPad.metadata.name}</Modal.Title>
        </Modal.Header>
        <Modal.Body>Do you want to open this file within Cipherpad or download it to your system? Either option will decrypt your file beforehand</Modal.Body>
        <Modal.Footer>
          <Button variant="secondary" onClick={() => setSelectedBlobPad(undefined)}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => {selectedBlobPad !== undefined && viewBlobPad(selectedBlobPad)}}>
            View
          </Button>
          <Button variant="primary" onClick={() => {selectedBlobPad !== undefined && downloadPadToFile(selectedBlobPad)}}>
            Download
          </Button>
        </Modal.Footer>
      </Modal>
      <Modal show={lastError !== undefined} onHide={() => {setLastError(undefined)}}>
        <Modal.Header closeButton>
          <Modal.Title>Error</Modal.Title>
        </Modal.Header>
        <Modal.Body>{lastError}</Modal.Body>
        <Modal.Footer>
          <Button variant="primary" onClick={() => setLastError(undefined)}>
            OK
          </Button>
        </Modal.Footer>
      </Modal>
    </Container>
  );
}