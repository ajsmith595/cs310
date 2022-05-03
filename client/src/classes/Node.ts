import Communicator, { ID } from "./Communicator";
import Utils from "./Utils";
import { v4 } from 'uuid';
import { NodeRegistration, NodeRegistrationOutput, NodeRegistrationInput } from "./NodeRegistration";
import Cache from "./Cache";
import EventBus from "./EventBus";



/**
 * Stores a position (x,y), primarily used for nodes
 */
export class Position {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = Math.round(x);
        this.y = Math.round(y);
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.x, obj.y)) {
            throw new Error("Could not deserialise");
        }
        return new Position(obj.x, obj.y);
    }

    serialise() {
        return {
            x: this.x,
            y: this.y
        };
    }


    multiply(t: number) {
        return new Position(this.x * t, this.y * t);
    }
    add(other: Position) {
        return new Position(this.x + other.x, this.y + other.y);
    }
}


/**
 * A node in the pipeline
 */
export default class EditorNode {
    position: Position; // its position in the node editor
    id: ID;
    node_type: string; // the identifier of its node type
    properties: Map<string, any>; // all the current properties set on the node
    group: ID; // the ID of its `group` which helps to determine what nodes are shown when a composited clip is opened in the node editor

    public static NodeRegister: Map<string, NodeRegistration> = new Map(); // A map of all node types in the application


    /**
     * Takes the node registrations supplied by the Rust backend, and populates the NodeRegister accordingly
     */
    public static deserialiseRegister(obj: any) {
        for (let node_type in obj) {
            EditorNode.NodeRegister.set(node_type, NodeRegistration.deserialise(obj[node_type]));
        }
    }

    constructor(
        position: Position,
        id: ID,
        node_type: string,
        properties: Map<string, any>,
        group: ID
    ) {
        this.position = position;
        this.id = id;
        this.node_type = node_type;
        this.properties = properties;
        this.group = group;
    }


    /**
     * Moves any I/O data for a node from one ID to another; used when the server creates a new ID for a new node, and we want to prevent the nodes from flashing
     */
    static moveCacheData(from_id: ID, to_id: ID) {
        let cacheInputs1 = this.cacheID(from_id) + "inputs";
        let cacheOutputs1 = this.cacheID(from_id) + "outputs";

        let cacheInputs2 = this.cacheID(to_id) + "inputs";
        let cacheOutputs2 = this.cacheID(to_id) + "outputs";

        Cache.put(cacheInputs2, Cache.get(cacheInputs1));
        Cache.put(cacheOutputs2, Cache.get(cacheOutputs1));
    }


    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.position, obj.id, obj.node_type, obj.properties, obj.group)) {
            throw new Error("Could not deserialise");
        }
        let properties = new Map();
        for (let p in obj.properties) {
            properties.set(p, obj.properties[p]);
        }
        return new EditorNode(Position.deserialise(obj.position), obj.id, obj.node_type, properties, obj.group);
    }

    serialise() {
        let obj: any = {};
        obj.position = this.position.serialise();
        obj.id = this.id;
        obj.node_type = this.node_type;
        obj.properties = {};
        obj.group = this.group;
        for (let [id, prop] of this.properties.entries()) {
            obj.properties[id] = prop;
        }
        return obj;
    }

    private static cacheID(id: ID) {
        return "node_" + id + "_";
    }

    private get cacheID() {
        return EditorNode.cacheID(this.id);
    }

    /**
     * If available, will return the currently obtained inputs for a particular node
     */
    getInputsSync() {
        let cacheID = this.cacheID + "inputs";
        return Cache.get(cacheID);
    }



    /**
     * Will return the inputs for a particular node; if they are not present in the cache, the Rust backend will be called to obtain the inputs
     */
    async getInputs(force = false) {
        let cacheID = this.cacheID + "inputs";
        if (Cache.get(cacheID) != null && !force) {
            return Cache.get(cacheID);
        }

        await new Promise((res, rej) => {
            Communicator.invoke('get_node_inputs', { node: this.serialise() }, (data) => {
                let inputs = new Map();
                for (let prop in data) {
                    inputs.set(prop, NodeRegistrationInput.deserialise(data[prop]));
                }
                Cache.put(cacheID, inputs);
                res(inputs);
            });
        });
    }

    // Same as corresponding functions for inputs
    getOutputsSync() {
        let cacheID = this.cacheID + "outputs";
        return Cache.get(cacheID);
    }


    // Same as corresponding functions for inputs
    async getOutputs(force = false) {
        let cacheID = this.cacheID + "outputs";
        if (Cache.get(cacheID) != null && !force) {
            return Cache.get(cacheID);
        }
        await new Promise((res, rej) => {
            Communicator.invoke('get_node_outputs', { node: this.serialise() }, (data) => {
                let outputs = new Map();
                for (let prop in data) {
                    outputs.set(prop, NodeRegistrationOutput.deserialise(data[prop]));
                }
                Cache.put(cacheID, outputs);
                res(outputs);
            })
        });
    }

    // Updates a node's position, and calls `save`
    savePosition(newPosition) {
        this.position = Position.deserialise(newPosition);
        this.save();
    }

    /**
     * Sends a message to the Rust backend to save any changes made to this node to the application state
     */
    async save() {
        Communicator.invoke('update_node', {
            node: this.serialise()
        });
        await new Promise((res, rej) => {
            setTimeout(res, 50);
        });

        await this.getInputs(true);
        await this.getOutputs(true);

        EventBus.dispatch(EventBus.EVENTS.NODE_EDITOR.FORCE_UPDATE, null);
    }

    /**
     * Creates a node of a particular type, with a particular group and position
     */
    static createNode(type: string, group: string, position: Position) {
        let register_entry = EditorNode.NodeRegister.get(type);

        let props = new Map();
        for (let [prop, property_details] of register_entry.default_properties.entries()) {
            let type = property_details.property_type;
            if (type.type == 'Number') {
                props.set(prop, type.getNumberRestrictions().default);
            }
        }

        return new EditorNode(position, v4(), type, props, group);
    }

    /**
     * Modifies a particular property, and if that property has been changed, it will send a message to the Rust backend notifying of the change
     */
    changeProperty(property, newValue) {
        //let register_entry = EditorNode.NodeRegister.get(this.node_type);
        let property_entry = this.getInputsSync().get(property);

        let hasChanged = false;
        if (property_entry.property_type.type == 'Number') {
            let restrictions = property_entry.property_type.getNumberRestrictions();

            let originalValue: number = parseFloat(newValue);
            if (isNaN(originalValue)) {
                originalValue = 0;
            }
            originalValue = parseFloat(originalValue.toPrecision(12));
            if (restrictions.min > newValue) {
                newValue = restrictions.min;
            }
            else if (restrictions.max < newValue) {
                newValue = restrictions.max;
            }
            newValue = Math.round(newValue / restrictions.step) * restrictions.step;

            newValue = newValue.toPrecision(12);
            if (originalValue != newValue) {
                hasChanged = true;
            }
            newValue = Number(newValue);
        }


        if (this.properties.get(property) != newValue || typeof newValue == 'object') {
            // if the value is an object, they'll still point to the same object, but we want to still update the display

            this.properties.set(property, newValue);
            hasChanged = true;
        }

        if (hasChanged)
            this.save();
    }
}
