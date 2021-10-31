import React from 'react';
import ReactFlow from 'react-flow-renderer';
import StoreContext from '../contexts/StoreContext';


interface Props {
    cache?: Map<string, any>;
}

interface State {
    // state
}

class NodeEditor extends React.Component<Props, State> {
    constructor(props: Props) {
        super(props);
    }

    render() {
        return (
            <StoreContext.Consumer>
                {({ value, setValue }) => {

                    let elements = [];
                    for (let [id, node] of value.nodes.entries()) {
                        elements.push({
                            id,
                            position: node.position,
                            data: {
                                label: <h1>{node.node_type}</h1>
                            }
                        });
                    }
                    for (let link of value.pipeline.links) {
                        elements.push({
                            id: link.id,
                            source: link.from.node_id,
                            target: link.to.node_id,
                            arrowHeadType: 'arrowclosed',
                        });
                    }
                    return (
                        <div style={{ width: "100%", height: "100%" }}>
                            <ReactFlow elements={elements} />
                        </div>
                    );
                }}
            </StoreContext.Consumer>
        )
    }

}

export default NodeEditor;