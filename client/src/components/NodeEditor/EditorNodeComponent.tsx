import { faChevronDown, faTimesCircle, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { Connection, Handle, Position } from 'react-flow-renderer';
import EditorNode from '../../classes/Node';
import AnimateHeight from 'react-animate-height';
import EventBus from '../../classes/EventBus';
import Store from '../../classes/Store';
import ClipDropComponent from '../shared/ClipDropComponent';

interface Props {
    data: {
        node: EditorNode,
        deleteLinks: (property: string) => void;
        deleteNode: () => void;
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


    setNodeProperty(property: string, value: any) {
        this.props.data.node.properties.set(property, value);
        this.props.data.node.save();
    }

    render() {

        if (this.props.data.node.getInputsSync() == null || this.props.data.node.getOutputsSync() == null) return <p>Invalid EditorNodeComponent!</p>;

        let node_registration = EditorNode.NodeRegister.get(this.props.data.node.node_type);
        let properties = [];

        let width: string | number = 1, height: string | number = 1;
        if (this.state.expanded) {
            width = "100%";
            height = "100%";
        }

        for (let [property, prop] of this.props.data.node.getInputsSync().entries()) {
            let accepted_type = prop.property_type;

            if (accepted_type.type === 'Pipeable') {
                // For each pipeable input, we create a target handle so that other nodes can connect to it



                let btn = null;
                if (this.state.hovered_property === property && Store.getCurrentStore().pipeline.containsLinkForNodeProperty(this.props.data.node.id, property)) {
                    btn = <button onClick={() => this.props.data.deleteLinks(property)} className="absolute" style={{ left: -6, top: -8 }}><FontAwesomeIcon className="text-red-600 rounded-full bg-white" icon={faTimesCircle} /></button>;
                }
                // If it has a property connected to it, and it's being hovered, show a cross icon to remove the edge




                properties.push(
                    <div className={`relative p-2 border-l border-white transition-colors ${this.state.hovered_property === property && this.state.expanded ? 'bg-white bg-opacity-10  border-opacity-70' : 'border-opacity-40'}`}
                        onMouseEnter={() => this.setState({ hovered_property: property })}
                        onMouseLeave={() => this.setState({ hovered_property: null })}>

                        <Handle type='target' position={Position.Left} id={property}
                            style={{ width: width, height: height, borderRadius: 0, backgroundColor: 'transparent', border: 0 }}
                        >
                            {btn}
                        </Handle>

                        <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                            <p>{prop.display_name}</p>
                        </AnimateHeight>
                    </div>
                )
            }
            else if (accepted_type.type === 'Clip') {

                // If it's a clip property, this is special - we place those on the node in the node editor itself, rather than just in the properties panel like any other property.
                properties.push(
                    <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                        <div className="px-2">
                            <p>{prop.display_name}</p>
                            <ClipDropComponent identifier={this.props.data.node.properties.get(property)} onDropClip={(clip_id) => this.setNodeProperty(property, clip_id)} disable_drag={this.props.data.node.node_type === 'output'} />
                        </div>
                    </AnimateHeight>
                );
            }
        }
        for (let [output_type, output] of this.props.data.node.getOutputsSync().entries()) {
            properties.push(
                <div className={`relative p-2 border-r border-white transition-colors ${this.state.hovered_property === output_type && this.state.expanded ? 'bg-white bg-opacity-10  border-opacity-70' : 'border-opacity-40'}`}
                    onMouseEnter={() => this.setState({ hovered_property: output_type })}
                    onMouseLeave={() => this.setState({ hovered_property: null })}>
                    <Handle type="source" position={Position.Right} id={output_type}
                        style={{ width: width, height: height, borderRadius: 0, backgroundColor: 'transparent', border: 0 }}
                    />
                    <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                        <p className="text-right">{output.display_name}</p>
                    </AnimateHeight>

                </div>
            )
            break;
        }


        let border = "border-black border-opacity-80";
        if (EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION) === this.props.data.node) {
            border = "border-pink-600";
        }

        let delete_btn = (
            <button className="float-right text-red-600 hover:bg-white hover:bg-opacity-20 px-1 rounded transition-colors" onClick={(e) => { this.props.data.deleteNode(); e.stopPropagation() }}>
                <FontAwesomeIcon icon={faTrash} />
            </button>
        );
        if (this.props.data.node.node_type === 'output') {
            delete_btn = null;
        }
        return (
            <div className={`bg-gray-900 text-white rounded-md border ${border} pb-1`} onClick={(e) => EventBus.dispatch(EventBus.EVENTS.APP.SET_SELECTION, this.props.data.node)}>
                <div className={`transition-colors duration-${EditorNodeComponent.EXPAND_DURATION} p-2 border-b ${this.state.expanded ? "border-gray-800" : 'border-transparent'} `}>

                    <span className="text-sm">
                        <button className="mr-2" onClick={() => this.setState({ expanded: !this.state.expanded })}>
                            <FontAwesomeIcon icon={faChevronDown} className={`transition-transform duration-${EditorNodeComponent.EXPAND_DURATION} transform  ${!this.state.expanded ? '-rotate-90' : ''}`} />
                        </button>
                        {node_registration.display_name}

                        {delete_btn}
                    </span>
                </div>
                {properties}
            </div>
        );
    }
}