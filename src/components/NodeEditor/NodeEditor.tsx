import React from 'react';
import ReactFlow, { ReactFlowProvider, useStoreState } from 'react-flow-renderer';
import EventBus from '../../classes/EventBus';
import EditorNode from '../../classes/Node';
import Store from '../../classes/Store';
import NodeEditorContext from '../../contexts/NodeEditorContext';
import StoreContext from '../../contexts/StoreContext';
import EditorNodeComponent from './EditorNodeComponent';

interface Props {
}

interface State {
    loading: boolean,
    group: string
}

class NodeEditor extends React.Component<Props, State> {

    reactFlowRef: React.Ref<HTMLDivElement>;
    store: Store;
    setStore: (x: Store) => void;

    constructor(props: Props) {
        super(props);
        this.state = {
            loading: true,
            group: "",
        }

        this.addNode = this.addNode.bind(this);
    }

    componentDidMount() {
        EventBus.on(EventBus.EVENTS.NODE_EDITOR.ADD_NODE, this.addNode);
        EventBus.registerGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP, () => this.state.group);
    }

    componentWillUnmount() {
        EventBus.remove(EventBus.EVENTS.NODE_EDITOR.ADD_NODE, this.addNode);
        EventBus.unregisterGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP);
    }

    addNode(node: EditorNode) {
        node.save();

        this.store.nodes.set(node.id, node);
        this.setStore(this.store);

        return true;
    }

    async prepareNodes(nodes: Array<EditorNode>) {
        let promises = [];
        for (let node of nodes) {
            promises.push(node.getOutputs());
        }
        for (let p of promises) {
            await p;
        }
        await new Promise(resolve => setTimeout(resolve, 200));
        this.setState({
            loading: !this.state.loading // force reload
        });
    }

    deleteLinks(node_id, property) {
        let links = [];
        for (let link of this.store.pipeline.links) {
            if ((link.from.node_id == node_id && link.from.property == property)
                || (link.to.node_id == node_id && link.to.property == property)) {
                continue;
            }
            links.push(link);
        }
        this.store.pipeline.links = links;
        this.setStore(this.store);
    }

    render() {
        return (
            <StoreContext.Consumer>
                {({ value, setValue }) => {
                    this.store = value;
                    this.setStore = setValue;
                    let elements = [];

                    let nodesInPreparation = [];
                    for (let [id, node] of value.nodes.entries()) {
                        if (node.outputs == null) {
                            nodesInPreparation.push(node);
                            continue;
                        }
                        elements.push({
                            id,
                            position: node.position,
                            data: {
                                node: node,
                                deleteLinks: (property: string) => this.deleteLinks(node.id, property)
                            },
                            type: 'editor_node'
                        });
                    }
                    if (nodesInPreparation.length > 0) {
                        this.prepareNodes(nodesInPreparation);
                    }
                    for (let link of value.pipeline.links) {
                        elements.push({
                            id: link.id,
                            source: link.from.node_id,
                            sourceHandle: link.from.property,
                            target: link.to.node_id,
                            targetHandle: link.to.property,
                            arrowHeadType: 'arrowclosed',
                        });
                    }
                    return (
                        <div style={{ width: "100%", height: "100%" }}>
                            <ReactFlow ref={this.reactFlowRef} elements={elements} nodeTypes={{
                                editor_node: EditorNodeComponent
                            }} onNodeDragStop={(_, node) => value.nodes.get(node.id).savePosition(node.position)}
                                onNodeDragStart={(e, n) => {
                                }}
                            />
                        </div>
                    );
                }}
            </StoreContext.Consumer>
        )
    }

}

export default NodeEditor;