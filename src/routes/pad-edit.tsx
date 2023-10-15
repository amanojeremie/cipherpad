import DOMPurify from "dompurify";
import { marked } from "marked";
import React, { useEffect, useState } from "react";
import { updatePad } from "../api/pad";
import { useCipherpad } from "../providers/CipherpadProvider";
import { useNavigate } from "react-router-dom";
import { createNewPad, decryptTextPad } from "../utils/pad-utils";
import { TextPadData, TextPadMetadata } from "../types/pad";

export interface PadEditState {
  metadata: TextPadMetadata,
  cleanData: TextPadData,
  data: TextPadData,
  showRendered: boolean,
  rendered: string,
  historyStack: string[],
  isNewPad: boolean,
  isDirty: boolean
}

export default function PadEdit() {
  const navigate = useNavigate();
  const cipherpadContext = useCipherpad();
  const { cipherpadUiState: { currentPad, currentNode }} = cipherpadContext;
  const [ padEditState, setPadEditState ] = useState<PadEditState>({
    metadata: {
      type: 'text',
      name: '',
      createdAt: 0,
      lastModifiedAt: 0
    },
    cleanData: {
      text: '',
      revisionHistory: []
    },
    data: {
      text: '',
      revisionHistory: []
    },
    showRendered: true,
    rendered: '',
    historyStack: [],
    isNewPad: false,
    isDirty: false
  })
  const { metadata, data, showRendered, rendered, historyStack, isDirty } = padEditState;
  const { name } = metadata;
  const { text } = data;

  const setMetadata = (metadata: TextPadMetadata) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      metadata
    }));
  }

  const setCleanData = (cleanData: TextPadData) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      cleanData
    }));
  }

  const setData = (data: TextPadData) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      data
    }));
  }

  const setName = (name: string) => {
    setMetadata({
      ...metadata,
      name
    });
  };

  const setText = (text: string) => {
    setData({
      ...data,
      text
    })
  };

  const setShowRendered = (showRendered: boolean) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      showRendered
    }));
  }

  const setRendered = (rendered: string) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      rendered
    }));
  };

  const setIsDirty = (isDirty: boolean) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      isDirty
    }));
  };

  const pushToHistory = (history: string) => {
    setPadEditState(padEditState => ({
      ...padEditState,
      historyStack: [...padEditState.historyStack, history]
    }))
  }

  const undo = () => {
    if (historyStack.length > 0) {
      const history = padEditState.historyStack[padEditState.historyStack.length - 1];
      setPadEditState(padEditState => {
        if (padEditState.historyStack.length > 0) {

          return {
            ...padEditState,
            historyStack: padEditState.historyStack.slice(0, -1)
          }
        }
        else {
          return padEditState;
        }
      });

      setText(history);
    }
  }

  const refreshNote = async() => {
    if (currentPad !== null && currentPad.metadata.type == 'text') {
      setMetadata(currentPad.metadata);
      const textPadData = await decryptTextPad(currentPad.id);
      setCleanData(textPadData);
      setData(textPadData);
    }
    else {
      setName('');
      setText('');
    }
  };

  useEffect(() => {
    setRendered(marked(text));
  }, [text]);

  useEffect(() => {
    refreshNote();
  }, [currentPad]);

  const save = async () => {
    if (currentPad === null) {
      await createNewPad({padMetadata: {...metadata, createdAt: Date.now(), lastModifiedAt: Date.now()}, padData: data, parentId: currentNode}, cipherpadContext);
    }
    else {
      await updatePad({id: currentPad.id, pad: {padMetadata: {...metadata, lastModifiedAt: Date.now()}, padData: data, parentId: currentNode}});
    }
    setIsDirty(false);
  }

  const onBackClicked = async () => {
    if (isDirty) {
      await save();
    }
    navigate('/app');
  }

  const onSaveClicked = async () => {
    await save();
  }

  const handleTitleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const newName = event.target?.value;
    setName(newName);
    setIsDirty(true);
  }

  const handleShowRenderedChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const newShowRenderedValue = event.target?.checked;
    setShowRendered(newShowRenderedValue);
  }

  const handleNoteChange = (event: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newText = event.target?.value;
    pushToHistory(text);
    setText(newText);
    setIsDirty(true);
  }
  
  return (
    <div className="note-editor">
      <div>
        <button onClick={onBackClicked}>Back</button>
        <button onClick={onSaveClicked}>Save{' '}{isDirty ? '*' : ''}</button>
        <button onClick={undo}>Undo</button>
        <input value={name} onChange={handleTitleChange}></input>
        <input checked={showRendered} onChange={handleShowRenderedChange} type="checkbox"></input>
      </div>
      <div className="note-editor-container">
        <textarea className="note" value={text} onChange={handleNoteChange} />
        {showRendered && <div className="markdown" dangerouslySetInnerHTML={{__html: DOMPurify.sanitize(rendered)}} />}
      </div>
    </div>
  )
}