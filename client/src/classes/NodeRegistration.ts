import Utils from "./Utils";



/**
 * Specifies how many of each type of stream a media type has
 */
export interface PipeableType {
    video: number,
    audio: number,
    subtitles: number
};

/**
 * Specifies requirements for a particular input of a ndoe
 */
export interface PipeableTypeRestriction {
    min: PipeableType,
    max: PipeableType
}


/**
 * Specifies a set of restrictions on numerical values in the application
 */
interface NumberRestrictions {
    min: number,
    max: number,
    step: number,
    default: number,
}


/**
 * Specifies a particular node input/property's type and restrictions
 */

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

    // Can only be called if the property is a number type
    getNumberRestrictions() {
        if (this.type != 'Number') {
            throw new Error("Cannot get number restrictions from non-number type!");
        }
        return this.extra_data as NumberRestrictions;
    }
    // Can only be called if the property is pipeable (i.e. it's an input/can be piped into via the node editor)
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


/**
 * Specifies data related to a particular input of a node type
 */
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

/**
 * Specifies a particular node type. Excludes functions to generate inputs/properties/outputs - that is exclusively for the Rust side to handle
 */
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
