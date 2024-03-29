import Communicator, { ID } from "./Communicator";
import EventBus from "./EventBus";
import { PipeableType } from "./NodeRegistration";
import Store from "./Store";
import Utils from "./Utils";


//#region Metadata Info
interface VideoStreamInfo {
    width: number,
    height: number,
    framerate: number,
    bitrate: number,
}

interface AudioStreamInfo {
    sample_rate: number,
    number_of_channels: number,
    bitrate: number,
    language: string,
}

interface SubtitleStreamInfo {
    language: string,
}


interface SourceClipInfo {
    duration: number;
    video_streams: Array<VideoStreamInfo>;
    audio_streams: Array<AudioStreamInfo>;
    subtitle_streams: Array<SubtitleStreamInfo>;
}

//#endregion Metadata Info

export class SourceClip {
    public id: ID;
    public name: string;
    public status: any; // uploading, uploaded, local only, etc.
    public info?: SourceClipInfo; // metadata
    public original_file_location?: string; // if the file is from this device, then the file location on this device



    // Not utilised by client, but present for serialisation purposes
    public file_location: null;
    public original_device_id: null;
    public thumbnail_location: null;


    constructor(id: ID, name: string, status: any, info?: SourceClipInfo, original_file_location?: string) {
        this.id = id;
        this.name = name;
        this.info = info;
        this.status = status;

        this.original_file_location = original_file_location;

        this.file_location = null;
        this.original_device_id = null;
        this.thumbnail_location = null;
    }

    static deserialise(obj: any) {
        if (obj == null) return null;
        if (Utils.propsUndefined(obj.id, obj.name, obj.status, obj.info, obj.original_file_location, obj.file_location, obj.original_device_id, obj.thumbnail_location)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new SourceClip(obj.id, obj.name, obj.status, obj.info, obj.original_file_location);
    }

    serialise() {
        return {
            id: this.id,
            name: this.name,
            status: this.status,
            info: this.info,
            original_file_location: this.original_file_location,
            file_location: null,
            original_device_id: null,
            thumbnail_location: null,
        }
    }


    getDuration() {
        if (this.info && this.info.duration) {
            return this.info.duration;
        }
    }


    // The identifier used to determine what clip is being targeted, when doing drag-drops for example
    getIdentifier() {
        return new ClipIdentifier(this.id, 'Source');
    }


    private _type: PipeableType = null; // the stream types for this clip
    getType() {
        return this._type;
    }
    async fetchType() {
        return await new Promise((res, rej) => {

            // Fetch the stream types for this clip from the Rust side
            Communicator.invoke('get_clip_type', {
                clipType: 'Source',
                id: this.id
            }, (type) => {
                this._type = type;
                res(type);
            });
        })
    }


    getThumbnailCacheID() {
        return "source_clips_thumbnail_data_" + this.id;
    }
}

export class CompositedClip {
    public id: ID;
    public name: string;


    constructor(id: ID, name: string) {
        this.id = id;
        this.name = name;


        EventBus.on('composited-clip-length', (data) => {
            // If this is called, it means that the composited clip has been changed, and this is notifying everyone of the new length of the composited clip
            // Therefore, if this event is for us, we capture the duration it's transmitted and store it so we can provide metadata for the video preview later
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



    serialise() {
        return {
            id: this.id,
            name: this.name,
        }
    }



    getIdentifier() {
        return new ClipIdentifier(this.id, 'Composited');
    }


    // Gets the group ID for a particular composited clip by iterating through all the nodes and finding the matching output node.
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
        return await new Promise((res, rej) => {
            Communicator.invoke('get_clip_type', {
                clipType: 'Composited',
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

    serialise() {
        return {
            clip_type: this.clip_type,
            id: this.id
        };
    }
}