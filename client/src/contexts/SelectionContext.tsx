import React from 'react';
import { Transform } from 'react-flow-renderer';
import { CompositedClip, SourceClip } from '../classes/Clip';
import EditorNode, { Position } from '../classes/Node';
import Store from '../classes/Store';


type Selectable = EditorNode | SourceClip | CompositedClip;
interface Context {
	currentSelection: Selectable;
	setSelection: (newSelection: Selectable) => void;
}

const SelectionContext = React.createContext<Context>({
	currentSelection: null,
	setSelection: (_) => { }
})

export default SelectionContext;