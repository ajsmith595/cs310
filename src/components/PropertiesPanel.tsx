import React from 'react';
import { CompositedClip, SourceClip } from '../classes/Clip';
import EventBus from '../classes/EventBus';
import EditorNode from '../classes/Node';


interface Props {
    cache?: Map<string, any>;
}

interface State {
    // state
}

class PropertiesPanel extends React.Component<Props, State> {
    constructor(props: Props) {
        super(props);
    }

    render() {

        let selection = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION);

        let content = <h1>Cannot provide information on the current selection</h1>;
        if (selection instanceof EditorNode) {

            let registration = EditorNode.NodeRegister.get(selection.node_type);
            let props = [];

            for (let prop in registration.properties) {
                let prop_detail = registration.properties[prop];
                let value = selection.properties.get(prop);
                if (typeof value == 'object') {
                    value = JSON.stringify(value);
                }
                props.push(
                    <div>
                        <p>{prop_detail.display_name}</p>
                        <small>{prop_detail.description}</small>
                        <p>Value: {value || 'Not set'}</p>
                    </div>
                )
            }
            content = <>
                <h1>{registration.display_name}</h1>
                <p>{registration.description}</p>
                <hr />
                <p>ID: {selection.id}</p>
                <p>Type: {selection.node_type}</p>
                {props}
            </>
        }
        else if (selection instanceof SourceClip) {
            content = <>
                <p>Name: {selection.name}</p>
                <small>File location: {selection.file_location}</small>
            </>;
        }
        else if (selection instanceof CompositedClip) {

        }
        return <div className="text-white">
            {content}
        </div>;

    }
}

export default PropertiesPanel;