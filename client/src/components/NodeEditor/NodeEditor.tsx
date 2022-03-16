import React from 'react';
import ReactFlow, { Connection, Edge, ReactFlowProvider } from 'react-flow-renderer';
import EventBus from '../../classes/EventBus';
import EditorNode, { Position } from '../../classes/Node';
import { Link, LinkEndpoint } from '../../classes/Pipeline';
import Store from '../../classes/Store';
import EditorNodeComponent from './EditorNodeComponent';
import NodeEditorStateManager from './NodeEditorStateManager';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faExclamationCircle } from '@fortawesome/free-solid-svg-icons';
import { Transition, animated } from 'react-spring'
import { v4 } from 'uuid';
import CustomEdgeComponent from './CustomEdgeComponent';
import Communicator from '../../classes/Communicator';

interface Props {
    initial_group?: string;
}

type NotificationType = 'error' | 'warning' | 'success' | 'info';

interface State {
    loading: boolean,
    group: string,
    notifications: Array<{
        title: string,
        message: string,
        type: NotificationType,
        id: string
    }>
}

class NodeEditor extends React.Component<Props, State> {
    static NOTIFICATION_TIMEOUT = 5000;


    reactFlowRef: React.Ref<HTMLDivElement>;
    constructor(props: Props) {
        super(props);
        this.state = {
            loading: true,
            group: props.initial_group || "",
            notifications: []
        }

        this.addNode = this.addNode.bind(this);
        this.changeGroup = this.changeGroup.bind(this);
    }

    componentDidMount() {
        EventBus.on(EventBus.EVENTS.NODE_EDITOR.ADD_NODE, this.addNode);
        EventBus.on(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, this.changeGroup);
        EventBus.registerGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP, () => this.state.group);

        EventBus.on('node_editor_force_reload', () => {
            this.forceUpdate();
            console.log("force update invoked")
        });
    }

    componentWillUnmount() {
        EventBus.remove(EventBus.EVENTS.NODE_EDITOR.ADD_NODE, this.addNode);
        EventBus.remove(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, this.changeGroup);
        EventBus.unregisterGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP);
    }

    addNode(node: EditorNode) {
        Communicator.invoke('add_node', {
            node: node.serialise()
        });
        return true;
    }

    changeGroup(group: string) {
        console.log("Changing group to:" + group);
        this.setState({
            group
        });
    }

    addNotification(message: string, type: NotificationType) {
        setTimeout(() => {
            this.setState({
                notifications: this.state.notifications.slice(1)
            });
        }, NodeEditor.NOTIFICATION_TIMEOUT);
        let notification = {
            title: type.toUpperCase(),
            message,
            type,
            id: v4()
        };
        this.setState({
            notifications: [
                ...this.state.notifications,
                notification
            ]
        });
    }

    async prepareNodes(nodes: Array<EditorNode>) {
        let promises = [];
        for (let node of nodes) {
            promises.push(node.getOutputs());
            promises.push(node.getInputs());
        }
        for (let p of promises) {
            await p;
        }
        await new Promise(resolve => setTimeout(resolve, 200));
        this.setState({
            loading: !this.state.loading // force reload
        });
    }

    addLink(e: Edge<any> | Connection) {
        if (e.source === e.target) {
            return;
        }

        let store = Store.getCurrentStore();
        for (let link of store.pipeline.links) {
            if (link.from.node_id === e.source && link.from.property === e.sourceHandle
                && link.to.node_id === e.target && link.to.property === e.targetHandle) {
                return;
            }
        }
        let link = new Link(new LinkEndpoint(e.source, e.sourceHandle), new LinkEndpoint(e.target, e.targetHandle));



        if (store.pipeline.hasCyclesWithLink(store, link)) {
            this.addNotification('Link caused cycle in pipeline', 'error');
        }
        else {
            Communicator.invoke('add_link', {
                link
            });

            let ids_left_inputs = [];
            let ids_left_outputs = [];
            for (let [id, node] of store.nodes.entries()) {
                ids_left_inputs.push(id);
                ids_left_outputs.push(id);
                node.getInputs(true).then(e => {
                    ids_left_inputs = ids_left_inputs.filter(e => e != id);
                    if (ids_left_inputs.length == 0 && ids_left_outputs.length == 0)
                        this.forceUpdate();
                });
                node.getOutputs(true).then(e => {
                    ids_left_outputs = ids_left_outputs.filter(e => e != id);
                    if (ids_left_inputs.length == 0 && ids_left_outputs.length == 0)
                        this.forceUpdate();

                });
            }
        }
    }

    isValidConnection(connection: Connection) {
        return true;
        // let store = Store.getCurrentStore();

        // let link = new Link(new LinkEndpoint(connection.source, connection.sourceHandle), new LinkEndpoint(connection.target, connection.targetHandle));
        // if (store.pipeline.hasCyclesWithLink(store, link))
        //     return false;
        // return true;
    }

    deleteLinks(node_id, property = null) {
        Communicator.invoke('delete_links', {
            nodeId: node_id,
            property
        })
    }


    deleteNode(node_id) {
        Communicator.invoke('delete_node', {
            id: node_id
        });
    }

    addImportNode(event: React.DragEvent) {
        event.preventDefault();
        let data = JSON.parse(event.dataTransfer.getData('application/json'));

        let state = EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_INTERNAL_STATE);

        let bounds = event.currentTarget.getBoundingClientRect();
        let [mouseX, mouseY] = [event.clientX - bounds.left, event.clientY - bounds.top];



        let x = (mouseX - state.transform[0]) / state.transform[2];
        let y = (mouseY - state.transform[1]) / state.transform[2];

        let pos = new Position(x, y);

        let node = EditorNode.createNode('clip_import', this.state.group, pos);
        node.properties.set('clip', data);
        this.addNode(node);
    }

    render() {
        let store = Store.getCurrentStore();
        let elements = [];

        let nodesInPreparation = [];
        for (let [id, node] of store.nodes.entries()) {
            if (node.getOutputsSync() === null || node.getInputsSync() == null) {
                nodesInPreparation.push(node);
                continue;
            }
            if (node.group === this.state.group) {
                elements.push({
                    id,
                    position: node.position,
                    data: {
                        node: node,
                        deleteLinks: (property: string) => this.deleteLinks(node.id, property),
                        deleteNode: () => this.deleteNode(node.id),
                        isValidConnection: (property: string, connection: Connection) => this.isValidConnection(connection)
                    },
                    type: 'editor_node'
                });
            }
        }
        if (nodesInPreparation.length > 0) {
            this.prepareNodes(nodesInPreparation);
        }

        for (let link of store.pipeline.links) {

            let from_node = store.nodes.get(link.from.node_id);
            let to_node = store.nodes.get(link.to.node_id);
            if (!from_node.getOutputsSync() || !to_node.getInputsSync() || from_node.group !== this.state.group || to_node.group !== this.state.group)
                continue;



            let input = to_node.getInputsSync().get(link.to.property);
            if (input) {
                let to_node_type = input.property_type;
                let output = from_node.getOutputsSync().get(link.from.property);
                if (output) {
                    elements.push({
                        id: link.id,
                        source: link.from.node_id,
                        sourceHandle: link.from.property,
                        target: link.to.node_id,
                        targetHandle: link.to.property,
                        arrowHeadType: 'arrowclosed',
                        type: 'custom_edge',
                        data: {
                            sourceType: output.property_type,
                            targetType: to_node_type.getPipeableType()
                        }
                    });
                }
            }
        }


        return (
            <div style={{ width: "100%", height: "100%" }} className="border-2 border-gray-400 relative"
                onDrop={(e) => this.addImportNode(e)}
                onDragOver={(e) => e.preventDefault()} >
                <ReactFlowProvider>
                    <ReactFlow ref={this.reactFlowRef} elements={elements} nodeTypes={{
                        editor_node: EditorNodeComponent
                    }} edgeTypes={{ custom_edge: CustomEdgeComponent }}

                        onNodeDragStop={(_, node) => store.nodes.get(node.id).savePosition(node.position)}

                        onConnect={(e) => this.addLink(e)}
                    />
                    <NodeEditorStateManager />
                </ReactFlowProvider>
                <div className="absolute right-2 bottom-2 z-50">
                    <Transition items={this.state.notifications}
                        keys={item => item.id}
                        from={{ opacity: 0 }}
                        enter={{ opacity: 1 }}
                        leave={{ opacity: 0 }}
                    >
                        {(styles, notification) => (
                            <animated.div className="text-white backdrop-filter backdrop-blur mb-2" style={styles}>
                                <div className="bg-red-600 px-2 py-1 bg-opacity-75 rounded-t">
                                    <h1 className="text-sm"><FontAwesomeIcon icon={faExclamationCircle} /> {notification.title}</h1>
                                </div>
                                <div className="px-4 py-2 bg-gray-900 bg-opacity-75 rounded-b">
                                    <p>{notification.message}</p>
                                </div>
                            </animated.div>
                        )}
                    </Transition>
                </div>
            </div >
        );

    }

}

export default NodeEditor;