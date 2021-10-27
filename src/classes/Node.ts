import { ID } from "./Communicator";
import Utils from "./Utils";



export enum PipeableType {
    Video = "Video",
    Audio = "Audio",
    Image = "Image"
}


export class Position {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.x, obj.y)) {
            throw new Error("Could not deserialise");
        }
        return new Position(obj.x, obj.y);
    }
}

export default class Node {
    position: Position;
    id: ID;
    node_type: string;
    properties: Map<string, any>;

    constructor(
        position: Position,
        id: ID,
        node_type: string,
        properties: Map<string, any>,
    ) {
        this.position = position;
        this.id = id;
        this.node_type = node_type;
        this.properties = properties;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.position, obj.id, obj.node_type, obj.properties)) {
            throw new Error("Could not deserialise");
        }
        return new Node(Position.deserialise(obj.position), obj.id, obj.node_type, obj.properties);
    }
}
