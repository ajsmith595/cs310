import { ID } from "./Communicator";
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
        if (Utils.propsUndefined(obj.id, obj.name, obj.file_location, obj.thumbnail_location)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new SourceClip(obj.id, obj.name, obj.file_location, obj.thumbnail_location);
    }
}

export class CompositedClip {
    public id: ID;
    public name: string;
    public pipeline_id: ID;
    constructor(id: ID, name: string, pipeline_id: ID) {
        this.id = id;
        this.name = name;
        this.pipeline_id = pipeline_id;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.id, obj.name, obj.pipeline_id)) {
            throw new Error("Could not deserialise! Malformed object");
        }
        return new CompositedClip(obj.id, obj.name, obj.pipeline_id);
    }
}