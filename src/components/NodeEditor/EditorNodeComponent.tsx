import { faArrowDown, faChevronDown, faTimesCircle } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { Handle, Position } from 'react-flow-renderer';
import Communicator from '../../classes/Communicator';
import EditorNode from '../../classes/Node';
import AnimateHeight from 'react-animate-height';
import StoreContext from '../../contexts/StoreContext';

interface Props {
    data: {
        node: EditorNode,
        deleteLinks: (property: string) => void;
    }
}

interface State {
    // state
    expanded: boolean;
    hovered_property: string;
    handle_hovered: boolean
}

export default class EditorNodeComponent extends React.Component<Props, State> {

    static EXPAND_DURATION = 300;

    constructor(props) {
        super(props);
        this.state = {
            expanded: true,
            hovered_property: null,
            handle_hovered: false
        }
    }

    componentDidMount() {

    }

    render() {
        return (

            <StoreContext.Consumer>
                {({ value, setValue }) => {
                    let node_registration = EditorNode.NodeRegister.get(this.props.data.node.node_type);
                    let handles = [];
                    for (let property in node_registration.properties) {
                        let prop = node_registration.properties[property];
                        for (let accepted_type of prop.property_type) {
                            if (accepted_type.hasOwnProperty('Pipeable')) {
                                let btn = null;
                                if (this.state.handle_hovered && this.state.hovered_property == property && value.pipeline.containsLinkForNodeProperty(this.props.data.node.id, property)) {
                                    btn = <button onClick={() => this.props.data.deleteLinks(property)} className="absolute" style={{ left: -6, top: -8 }}><FontAwesomeIcon className="text-red-600 rounded-full bg-white" icon={faTimesCircle} /></button>;
                                }
                                handles.push(
                                    <div className={`relative p-2 ${this.state.hovered_property == property && this.state.expanded ? 'bg-red-400' : ''}`}
                                        onMouseEnter={() => this.setState({ hovered_property: property })}
                                        onMouseLeave={() => this.setState({ hovered_property: null })}>

                                        <Handle type='target' position={Position.Left} id={property}
                                            onMouseEnter={() => this.setState({ handle_hovered: true })}
                                            onMouseLeave={() => this.setState({ handle_hovered: false })}
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
                        }
                    }
                    for (let output_type in this.props.data.node.outputs) {
                        let output = this.props.data.node.outputs[output_type];
                        handles.push(
                            <div className="relative p-2">
                                <Handle type="source" position={Position.Right} id={output_type} />

                                <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                                    <p className="text-right">{output.display_name}</p>
                                </AnimateHeight>
                            </div>
                        )
                        break;
                    }
                    return (
                        <div className="bg-gray-300 rounded-sm border-2  border-gray-900">
                            <div className={`transition-colors duration-${EditorNodeComponent.EXPAND_DURATION} p-2 border-b-2 ${this.state.expanded ? "border-gray-500" : 'border-transparent'} `}>
                                <h1>{node_registration.display_name}<button className="float-right" onClick={() => this.setState({ expanded: !this.state.expanded })}><FontAwesomeIcon icon={faChevronDown} className={`transition-transform duration-${EditorNodeComponent.EXPAND_DURATION} mr-2 transform  ${!this.state.expanded ? 'rotate-90' : ''}`} /></button></h1>
                                <AnimateHeight height={this.state.expanded ? 'auto' : 1} duration={EditorNodeComponent.EXPAND_DURATION}>
                                    <small className={`block overflow-hidden`}>{node_registration.description}</small>
                                </AnimateHeight>
                            </div>
                            {handles}
                            {/* <Handle type="source" position={Position.Right} />
                <Handle type="target" position={Position.Left} /> */}
                        </div>
                    );
                }}
            </StoreContext.Consumer>
        );
    }
}