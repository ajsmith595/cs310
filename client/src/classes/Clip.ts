import Communicator, { ID } from "./Communicator";
import EventBus from "./EventBus";
import { PipeableType } from "./NodeRegistration";
import Store from "./Store";
import Utils from "./Utils";

export class SourceClip {
    public id: ID;
    public name: string;
    public file_location: string;
    public thumbnail_location: string;
    public info: any; // TODO: implement proper (de)serialiser for this
    public status: any;
    constructor(id: ID, name: string, file_location: string, thumbnail_location: string, info: any, status: any) {
        this.id = id;
        this.name = name;
        this.file_location = file_location;
        this.thumbnail_location = thumbnail_location;
        this.info = info;
        this.status = status;
    }

    static deserialise(obj: any) {
        if (obj == null) return null;
        if (Utils.propsUndefined(obj.id, obj.name, obj.file_location, obj.thumbnail_location, obj.info, obj.status)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new SourceClip(obj.id, obj.name, obj.file_location, obj.thumbnail_location, obj.info, obj.status);
    }


    getDuration() {
        if (this.info && this.info.duration) {
            return this.info.duration;
        }
    }


    getIdentifier() {
        return {
            clip_type: 'Source',
            id: this.id
        }
    }

    private _type: PipeableType = null;
    getType() {
        return this._type;
    }
    async fetchType() {
        await new Promise((res, rej) => {
            Communicator.invoke('get_clip_type', {
                clipType: 'source',
                id: this.id
            }, (type) => {
                this._type = type;
                res(type);
            });
        })
    }
}

export class CompositedClip {
    public id: ID;
    public name: string;
    constructor(id: ID, name: string) {
        this.id = id;
        this.name = name;

        Communicator.on('composited-clip-length', (data) => {
            let id = data[0];
            let duration_ms = data[1];

            if (id == this.id) {
                this._duration_ms = duration_ms;
            }
        });
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

    private _duration_ms: number = null;
    getDuration() {
        return this._duration_ms;
    }

    private _type: PipeableType = null;
    getType() {
        return this._type;
    }
    async fetchType() {
        await new Promise((res, rej) => {
            Communicator.invoke('get_clip_type', {
                clipType: 'composited',
                id: this.id
            }, (type) => {
                this._type = type;
                res(type);
            });
        })
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