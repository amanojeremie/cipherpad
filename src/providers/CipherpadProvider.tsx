import { PropsWithChildren, createContext, useContext, useEffect, useState } from "react";
import { EncryptedPad, Node, NodeTree, PadMap } from "../types/pad";
import { getNodeTree, openOrCreateCipherpad, unlockCipherpad } from "../api/cipherpad";
import { getPadMap } from "../api/pad";

interface CipherpadState {
  nodeTree: NodeTree,
  nodeMap: {[key: string]: Node},
  padMap: PadMap
}

interface CipherpadUiState {
  loading: boolean
  currentNode: string | null,
  currentPad: EncryptedPad | null,
  currentNodeChildren: EncryptedPad[],
  parentNode: string | null,
}


export interface CipherpadContextType {
  isCipherpadOpen: boolean,
  cipherpadState: CipherpadState,
  cipherpadUiState: CipherpadUiState,
  openOrCreateCipherpadWithPassword: (path: string, password: string) => Promise<void>,
  refreshCipherpadData: () => Promise<CipherpadState | undefined>,
  setCipherpadState: React.Dispatch<React.SetStateAction<CipherpadState>>,
  setCipherpadUiState: React.Dispatch<React.SetStateAction<CipherpadUiState>>,
  setCurrentNode: (id: string | null) => void;
  setCurrentPad: (encryptedPad: EncryptedPad | null) => void;
}

const CipherpadContext = createContext<CipherpadContextType | undefined>(undefined);

export function useCipherpad() {
  const context = useContext(CipherpadContext);

  if (!context) {
    throw new Error("Must be used in a CipherpadProvider");
  }

  return context;
}

function flattenNodeTree(nodeTree: NodeTree) {
  const nodeMap : {[key: string]: Node} = {};
  function traverse(node: Node) {
    nodeMap[node.id] = node;
    for (const childNode of node.children) {
      traverse(childNode);
    }
  }
  for (const node of nodeTree.nodes) {
    traverse(node);
  }
  return nodeMap;
}

export default function CipherpadProvider({children}: PropsWithChildren) {
  const [isCipherpadOpen, setIsCipherpadOpen] = useState(false);
  const [cipherpadUiState, setCipherpadUiState] = useState<CipherpadUiState>({
    loading: true,
    currentNode: null,
    currentPad: null,
    currentNodeChildren: [],
    parentNode: null
  });
  const {currentNode} = cipherpadUiState;

  const [cipherpadState, setCipherpadState] = useState<CipherpadState>({
    nodeTree: { nodes: [] },
    nodeMap: {},
    padMap: { pads: {} }
  });
  const {nodeTree, nodeMap, padMap} = cipherpadState;

  const refreshCipherpadData = async() => {
    try {
      setCipherpadUiState(cipherpadUiState => ({
        ...cipherpadUiState,
        loading: true
      }));
      const newNodeTree = await getNodeTree();
      const newCipherpadState: CipherpadState = {
        nodeTree: newNodeTree,
        nodeMap: flattenNodeTree(newNodeTree),
        padMap: await getPadMap(),
      };
      setCipherpadState(newCipherpadState);
      return newCipherpadState;
    }
    catch (e) {
      console.error(e);
    }
  }

  const openOrCreateCipherpadWithPassword = async(path: string, password: string) => {
    await openOrCreateCipherpad(path);
    await unlockCipherpad(password);
    setIsCipherpadOpen(true);
  }

  const setCurrentNode = (id: string | null) => {
    setCipherpadUiState(cipherpadUiState => ({
      ...cipherpadUiState,
      currentNode: id
    }));
  }

  const setCurrentPad = (encryptedPad: EncryptedPad | null) => {
    setCipherpadUiState(cipherpadUiState => ({
      ...cipherpadUiState,
      currentPad: encryptedPad
    }));
  }

  const cipherpadContext: CipherpadContextType = {
    isCipherpadOpen,
    cipherpadUiState,
    cipherpadState,
    openOrCreateCipherpadWithPassword,
    refreshCipherpadData,
    setCipherpadState,
    setCipherpadUiState,
    setCurrentNode,
    setCurrentPad,
  };

  useEffect(() => {
    if (currentNode !== null) {
      const newCurrentPageChildren: EncryptedPad[] = [];
      for (const childNode of nodeMap[currentNode].children) {
        newCurrentPageChildren.push(padMap.pads[childNode.id]);
      }
      setCipherpadUiState(cipherpadUiState => ({
        ...cipherpadUiState,
        parentNode: padMap.pads[currentNode].parentId,
        currentNodeChildren: newCurrentPageChildren.sort((a, b) => a.metadata.name.toLowerCase().localeCompare(b.metadata.name.toLowerCase())),
        loading: false
      }));
    }
    else {
      const newCurrentPageChildren: EncryptedPad[]  = [];
      for (const node of nodeTree.nodes) {
        newCurrentPageChildren.push(padMap.pads[node.id]);
      }
      setCipherpadUiState(cipherpadUiState => ({
        ...cipherpadUiState,
        parentNode: null,
        currentNodeChildren: newCurrentPageChildren.sort((a, b) => a.metadata.name.toLowerCase().localeCompare(b.metadata.name.toLowerCase())),
        loading: false
      }));
    }
  }, [cipherpadState, currentNode]);


  return <CipherpadContext.Provider value={cipherpadContext}>{children}</CipherpadContext.Provider>
}