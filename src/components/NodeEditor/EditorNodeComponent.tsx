import { faArrowDown, faChevronDown, faLayerGroup, faPhotoVideo, faTimesCircle, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { Connection, Handle, Position } from 'react-flow-renderer';
import Communicator from '../../classes/Communicator';
import EditorNode from '../../classes/Node';
import AnimateHeight from 'react-animate-height';
import EventBus from '../../classes/EventBus';
import { ClipIdentifier } from '../../classes/Clip';
import Store from '../../classes/Store';

interface Props {
    data: {
        node: EditorNode,
        deleteLinks: (property: string) => void;
        deleteNode: () => void;
        isValidConnection: (property: string, connection: Connection) => boolean;
    }
}

interface State {
    // state
    expanded: boolean;
    hovered_property: string;
}

export default class EditorNodeComponent extends React.Component<Props, State> {

    static EXPAND_DURATION = 300;

    constructor(props) {
        super(props);
        this.state = {
            expanded: true,
            hovered_property: null,
        }
    }

    componentDidMount() {

    }

    onDropClip(property, event: React.DragEvent) {
        this.props.data.node.onDropClip(property, event);
    }

    render() {
        let node_registration = EditorNode.NodeRegister.get(this.props.data.node.node_type);
        let properties = [];

        let width: string | number = 1, height: string | number = 1;
        if (this.state.expanded) {
            width = "100%";
            height = "100%";
        }
        for (let property in node_registration.properties) {
            let prop = node_registration.properties[property];
            for (let accepted_type of prop.property_type) {
                if (accepted_type.hasOwnProperty('Pipeable')) {
                    let btn = null;
                    if (this.state.hovered_property == property && Store.getCurrentStore().pipeline.containsLinkForNodeProperty(this.props.data.node.id, property)) {
                        btn = <button onClick={() => this.props.data.deleteLinks(property)} className="absolute" style={{ left: -6, top: -8 }}><FontAwesomeIcon className="text-red-600 rounded-full bg-white" icon={faTimesCircle} /></button>;
                    }


                    properties.push(
                        <div className={`relative p-2 transition-colors rounded-md ${this.state.hovered_property == property && this.state.expanded ? 'bg-gray-300' : ''}`}
                            onMouseEnter={() => this.setState({ hovered_property: property })}
                            onMouseLeave={() => this.setState({ hovered_property: null })}>

                            <Handle type='target' position={Position.Left} id={property}
                                style={{ width: width, height: height, borderRadius: 0, backgroundColor: 'transparent', border: 0 }}
                                isValidConnection={(connection) => this.props.data.isValidConnection(property, connection)}
                            >
                                {btn}
                            </Handle>

                            <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                                <p>{prop.display_name}</p>
                            </AnimateHeight>
                        </div>
                    )
                    break;
                }

                if (accepted_type == 'Clip') {
                    let clip_identifier = ClipIdentifier.deserialise(this.props.data.node.properties.get(property));
                    let name = "Not selected";
                    let icon = null;
                    if (clip_identifier) {
                        let clip = Store.getCurrentStore().clips[clip_identifier.clip_type.toLowerCase()].get(clip_identifier.id);
                        if (clip) {
                            name = clip.name;
                            icon = <FontAwesomeIcon className="mr-2" icon={clip_identifier.clip_type.toLowerCase() == 'source' ? faPhotoVideo : faLayerGroup} />;
                        }
                    }
                    let onDragOver = (e) => { e.preventDefault(); e.stopPropagation() };
                    if (this.props.data.node.node_type == "output") {
                        onDragOver = () => { };
                    }
                    properties.push(
                        <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                            <div className="px-2">
                                <p>{prop.display_name}</p>
                                <div className="p-2 bg-gray-400" onDrop={(e) => this.onDropClip(property, e)} onDragOver={onDragOver}>
                                    <p>{icon} {name}</p>
                                </div>
                            </div>
                        </AnimateHeight>
                    );
                }
            }
        }
        for (let output_type in this.props.data.node.outputs) {
            let output = this.props.data.node.outputs[output_type];
            properties.push(
                <div className={`relative p-2 rounded-md transition-colors ${this.state.hovered_property == output_type && this.state.expanded ? 'bg-gray-300' : ''}`}
                    onMouseEnter={() => this.setState({ hovered_property: output_type })}
                    onMouseLeave={() => this.setState({ hovered_property: null })}>
                    <Handle type="source" position={Position.Right} id={output_type}
                        style={{ width: width, height: height, borderRadius: 0, backgroundColor: 'transparent', border: 0 }}
                        isValidConnection={(connection) => this.props.data.isValidConnection(output_type, connection)}
                    />
                    <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                        <p className="text-right">{output.display_name}</p>
                    </AnimateHeight>

                </div>
            )
            break;
        }


        let border = "border-gray-900";
        if (EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION) == this.props.data.node) {
            border = "border-pink-600";
        }

        let delete_btn = (
            <button className="float-right text-red-600 mr-2" onClick={(e) => { this.props.data.deleteNode(); e.stopPropagation() }}>
                <FontAwesomeIcon icon={faTrash} />
            </button>
        );
        if (this.props.data.node.node_type == 'output') {
            delete_btn = null;
        }
        return (
            <div className={`bg-gray-200 rounded-md border-2 ${border}`} onClick={(e) => EventBus.dispatch(EventBus.EVENTS.APP.SET_SELECTION, this.props.data.node)}>
                <div className={`transition-colors duration-${EditorNodeComponent.EXPAND_DURATION} p-2 border-b-2 ${this.state.expanded ? "border-gray-500" : 'border-transparent'} `}>
                    <h1>{node_registration.display_name}
                        <button className="float-right mr-2" onClick={() => this.setState({ expanded: !this.state.expanded })}>
                            <FontAwesomeIcon icon={faChevronDown} className={`transition-transform duration-${EditorNodeComponent.EXPAND_DURATION} transform  ${!this.state.expanded ? 'rotate-90' : ''}`} />
                        </button>

                        {delete_btn}
                    </h1>
                    <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                        <small className={`block overflow-hidden`}>{node_registration.description}</small>
                    </AnimateHeight>
                </div>
                {properties}
                {/* <Handle type="source" position={Position.Right} />
                <Handle type="target" position={Position.Left} /> */}
            </div>
        );
    }
}