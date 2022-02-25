import Communicator, { ID } from "./Communicator";
import Utils from "./Utils";
import { v4 } from 'uuid';
import Store from "./Store";
import { NodeRegistration, NodeRegistrationOutput, NodeRegistrationInput } from "./NodeRegistration";
import Cache from "./Cache";




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

export default class EditorNode {
    position: Position;
    id: ID;
    node_type: string;
    properties: Map<string, any>;
    group: ID;

    public static NodeRegister: Map<string, NodeRegistration> = new Map();


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

    private get cacheID() {
        return "node_" + this.id + "_";
    }

    getInputsSync() {
        let cacheID = this.cacheID + "inputs";
        return Cache.get(cacheID);
    }

    async getInputs(force = false) {
        let cacheID = this.cacheID + "inputs";
        if (Cache.get(cacheID) != null && !force) {
            return Cache.get(cacheID);
        }
        Communicator.invoke('get_node_inputs', { node: this.serialise() }, (data) => {
            let inputs = new Map();
            for (let prop in data) {
                inputs.set(prop, NodeRegistrationInput.deserialise(data[prop]));
            }
            Cache.put(cacheID, inputs);
            return inputs;
        });
    }

    getOutputsSync() {
        let cacheID = this.cacheID + "outputs";
        return Cache.get(cacheID);
    }

    async getOutputs(force = false) {
        let cacheID = this.cacheID + "outputs";
        if (Cache.get(cacheID) != null && !force) {
            return Cache.get(cacheID);
        }
        Communicator.invoke('get_node_outputs', { node: this.serialise() }, (data) => {
            let outputs = new Map();
            for (let prop in data) {
                outputs.set(prop, NodeRegistrationOutput.deserialise(data[prop]));
            }
            Cache.put(cacheID, outputs);
            return outputs;
        });
    }

    savePosition(newPosition) {
        this.position = Position.deserialise(newPosition);
        this.save();
    }

    save() {
        Communicator.invoke('update_node', {
            node: this.serialise()
        })
    }

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


    onDropClip(property, event: React.DragEvent) {
        if (this.node_type == "output") {
            return;
        }
        event.preventDefault();
        event.stopPropagation();
        let data = JSON.parse(event.dataTransfer.getData('application/json'));
        this.properties.set(property, data);
        this.save();
    }

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
