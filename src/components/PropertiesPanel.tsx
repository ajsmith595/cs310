import { faLayerGroup, faPhotoVideo } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { ClipIdentifier, CompositedClip, SourceClip } from '../classes/Clip';
import EventBus from '../classes/EventBus';
import EditorNode from '../classes/Node';
import Store from '../classes/Store';


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

    componentDidMount() {
        // EventBus.on(EventBus.EVENTS.APP.SET_STORE, () => this.forceUpdate());
        // EventBus.on(EventBus.EVENTS.APP.SET_STORE_UI, () => this.forceUpdate());
    }
    componentWillUnmount() {
    }


    render() {

        console.log("Rerendered");

        let selection = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION);

        let content = <h1>Cannot provide information on the current selection</h1>;
        if (selection instanceof EditorNode) {

            let registration = EditorNode.NodeRegister.get(selection.node_type);
            let props = [];


            let onDragOver = (e) => { e.preventDefault(); e.stopPropagation() };
            if (selection.node_type == "output") {
                onDragOver = () => { };
            }

            for (let prop in registration.properties) {
                let prop_detail = registration.properties[prop];
                let is_piped = false;
                for (let type of prop_detail.property_type) {
                    if (type.hasOwnProperty('Pipeable')) {
                        is_piped = true;
                        break;
                    }
                }
                if (is_piped) {
                    continue;
                }
                console.log(prop_detail);

                let value = selection.properties.get(prop);
                let display = null;

                if (prop_detail.property_type[0] == 'Clip') {
                    let icon = null;
                    let name = "Not selected";
                    let clip_identifier = ClipIdentifier.deserialise(value);
                    if (clip_identifier) {
                        let clip = Store.getCurrentStore().clips[clip_identifier.clip_type.toLowerCase()].get(clip_identifier.id);
                        if (clip) {
                            name = clip.name;
                            icon = <FontAwesomeIcon className="mr-2" icon={clip_identifier.clip_type.toLowerCase() == 'source' ? faPhotoVideo : faLayerGroup} />;
                        }
                    }

                    display = (
                        <div className="p-2 mx-2 bg-gray-400" onDrop={(e) => selection.onDropClip(prop, e)} onDragOver={onDragOver}>
                            <p>{icon} {name}</p>
                        </div>
                    );
                }
                else if (prop_detail.property_type[0].hasOwnProperty('Number')) {
                    let details = prop_detail.property_type[0]['Number'];
                    value = Math.round(value / details.step) * details.step;
                    display = (
                        <div>
                            <input key={Date.now()} className="bg-gray-600 p-2 w-full outline-none" defaultValue={value} type="number" step={details.step} min={details.min} max={details.max} onBlur={(e) => selection.changeProperty(prop, e.target.value)} onKeyDown={(e) => {
                                if (e.key == 'Enter') {
                                    e.preventDefault();
                                    let target: HTMLInputElement & EventTarget = (e.target as any);
                                    selection.changeProperty(prop, target.value);
                                }
                            }} />
                        </div>
                    );
                }
                else {
                    if (typeof value == 'object') {
                        value = JSON.stringify(value);
                    }
                    if (!value) {
                        value = 'Not set';
                    }
                    display = <p>{value}</p>;
                }
                props.push(
                    <div className="border p-3 border-gray-600 mt-2">
                        <p className="flex">
                            <span>{prop_detail.display_name}</span>
                            <span className="text-xs text-right flex-1">{prop_detail.description}</span>
                        </p>
                        {display}
                    </div>
                )
            }
            content = <>
                <div className="mb-4">
                    <h1 className="text-lg">{registration.display_name}</h1>
                    <p className="text-xs">{registration.description}</p>
                </div>
                <div className="">
                    <div className="border p-3 border-gray-600 mt-2">
                        <p className="flex">
                            <span>ID</span>
                            <span className="text-right flex-1">{selection.id}</span>
                        </p>
                    </div>
                    {props}
                </div>
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