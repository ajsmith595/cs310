import Communicator, { ID } from "./Communicator";
import Utils from "./Utils";
import { v4 } from 'uuid';


export enum PipeableType {
    Video = "Video",
    Audio = "Audio",
    Image = "Image"
}


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
}

export class NodeRegistration {
    description: string;
    display_name: string;
    id: string;
    properties: {
        [k: string]: {
            description: string;
            display_name: string;
            name: string;
            property_type: Array<string>;
        }
    }
}

export default class EditorNode {
    position: Position;
    id: ID;
    node_type: string;
    properties: Map<string, any>;
    group: ID;

    public static NodeRegister: Map<string, NodeRegistration> = new Map();

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



    public outputs: {
        [k: string]: {
            description: string;
            display_name: string;
            name: string;
            property_type: Array<string>;
        }
    } = null;
    async getOutputs() {
        if (this.outputs != null) {
            return this.outputs;
        }
        Communicator.invoke('get_node_outputs', { node: this.serialise() }, (data) => {
            this.outputs = data;
            return data;
        });
    }

    async savePosition(newPosition) {
        this.position = Position.deserialise(newPosition);
        this.save();
    }

    async save() {
        Communicator.invoke('update_node', { node: this.serialise() }, (data) => {
            return true;
        });
    }

    static createNode(type: string, group: string, position: Position) {
        return new EditorNode(position, v4(), type, new Map(), group);
    }
}
