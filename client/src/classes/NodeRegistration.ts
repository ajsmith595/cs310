import Utils from "./Utils";




/*

REDO TYPES - instead of specifying types, we shall instead specify minimum + maximum number of streams of each type (video, audio, subtitle)
    We will then 'classify' these types based on what streams they have:
        - Video = exactly 1 video stream + any other streams
        - Audio = exactly 0 video streams + 1 or more audio streams + any subtitle streams
        - Subtitle = exactly 0 video streams + 0 audio streams + 1 or more subtitle streams
        - Container = anything that is not covered (so will be when we have more than 1 video stream)

*/

export interface PipeableType {
    video: number,
    audio: number,
    subtitles: number
};

export interface PipeableTypeRestriction {
    min: PipeableType,
    max: PipeableType
}
// export type PipeableTypeRestriction = FixedLengthArray<[PipeableType, PipeableType]>;


interface NumberRestrictions {
    min: number,
    max: number,
    step: number,
    default: number,
}

export class PropertyType {
    type: string;
    extra_data: any;

    constructor(type: string, extra_data: any) {
        this.type = type;
        this.extra_data = extra_data;
    }

    static deserialise(obj: any) {
        if (typeof obj == 'string') {
            return new PropertyType(obj, null);
        }
        let key = Object.keys(obj)[0];
        return new PropertyType(key, obj[key]);
    }

    getNumberRestrictions() {
        if (this.type != 'Number') {
            throw new Error("Cannot get number restrictions from non-number type!");
        }
        return this.extra_data as NumberRestrictions;
    }
    getPipeableType() {
        if (this.type != 'Pipeable') {
            throw new Error("Cannot get pipeable type from non-pipeable!");
        }
        if (!(this.extra_data instanceof Array)) {
            throw new Error("Cannot get pipeable type from non-array thingy");
        }

        return {
            min: this.extra_data[0],
            max: this.extra_data[1]
        } as PipeableTypeRestriction;
    }

}


export class NodeRegistrationInput {
    description: string;
    display_name: string;
    name: string;
    property_type: PropertyType;

    constructor(name: string, display_name: string, description: string, property_type: PropertyType) {
        this.name = name;
        this.display_name = display_name;
        this.description = description;
        this.property_type = property_type;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.name, obj.display_name, obj.description, obj.property_type)) {
            throw new Error("Could not deserialise");
        }
        let property_type = PropertyType.deserialise(obj.property_type);
        return new NodeRegistrationInput(obj.name, obj.display_name, obj.description, property_type);
    }
}


export class NodeRegistrationOutput {
    description: string;
    display_name: string;
    name: string;
    property_type: PipeableType;

    constructor(name: string, display_name: string, description: string, property_type: PipeableType) {
        this.name = name;
        this.display_name = display_name;
        this.description = description;
        this.property_type = property_type;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.name, obj.display_name, obj.description, obj.property_type)) {
            throw new Error("Could not deserialise");
        }
        return new NodeRegistrationInput(obj.name, obj.display_name, obj.description, obj.property_type);
    }
}


export class NodeRegistration {
    description: string;
    display_name: string;
    id: string;
    default_properties: Map<string, NodeRegistrationInput>;

    constructor(id: string, display_name: string, description: string, default_properties: Map<string, NodeRegistrationInput>) {
        this.id = id;
        this.display_name = display_name;
        this.description = description;
        this.default_properties = default_properties;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.description, obj.display_name, obj.id, obj.default_properties)) {
            throw new Error("Could not deserialise");
        }

        let props = new Map();
        for (let prop in obj.properties) {
            props.set(prop, NodeRegistrationInput.deserialise(obj.default_properties[prop]));
        }

        return new NodeRegistration(obj.id, obj.display_name, obj.description, props);
    }
}
