import React from 'react';
import { Transform } from 'react-flow-renderer';
import EditorNode, { Position } from '../classes/Node';
import Store from '../classes/Store';

interface Context {
    addNode: (node: EditorNode) => boolean,
    currentGroup: string,
    nodeEditorTransform: Transform
}

const NodeEditorContext = React.createContext<Context>({
    addNode: () => true,
    currentGroup: "",
    nodeEditorTransform: null
})

export default NodeEditorContext;