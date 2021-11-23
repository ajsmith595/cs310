import { ID } from "./Communicator";
import EventBus from "./EventBus";
import Store from "./Store";
import Utils from "./Utils";

export class SourceClip {
    public id: ID;
    public name: string;
    public file_location: string;
    public thumbnail_location: string;
    constructor(id: ID, name: string, file_location: string, thumbnail_location: string) {
        this.id = id;
        this.name = name;
        this.file_location = file_location;
        this.thumbnail_location = thumbnail_location;
    }

    static deserialise(obj: any) {
        if (obj == null) return null;
        if (Utils.propsUndefined(obj.id, obj.name, obj.file_location, obj.thumbnail_location)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new SourceClip(obj.id, obj.name, obj.file_location, obj.thumbnail_location);
    }


    getIdentifier() {
        return {
            clip_type: 'Source',
            id: this.id
        }
    }
}

export class CompositedClip {
    public id: ID;
    public name: string;
    constructor(id: ID, name: string) {
        this.id = id;
        this.name = name;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.id, obj.name)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new CompositedClip(obj.id, obj.name);
    }



    getIdentifier() {
        return {
            clip_type: 'Composited',
            id: this.id
        }
    }

    getClipGroup() {
        let store: Store = EventBus.getValue(EventBus.GETTERS.APP.STORE);
        let nodes = store.nodes;
        for (let [id, node] of nodes.entries()) {
            if (node.node_type == 'output') {
                let clip = node.properties.get("clip");
                if (clip && clip.id == this.id) {
                    return node.group;
                }
            }
        }
    }
}

export class ClipIdentifier {
    public clip_type: 'Source' | 'Composited';
    public id: ID;

    constructor(id: ID, clip_type: 'Source' | 'Composited') {
        this.id = id;
        this.clip_type = clip_type;
    }

    static deserialise(obj: any) {
        if (obj == null) return null;
        if (Utils.propsUndefined(obj.id, obj.clip_type)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new ClipIdentifier(obj.id, obj.clip_type);
    }
}