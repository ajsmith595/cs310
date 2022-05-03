import { ID } from "./Communicator";
import EditorNode from './Node';
import Pipeline from "./Pipeline";
import { CompositedClip, SourceClip } from "./Clip";
import Utils from "./Utils";
import EventBus from "./EventBus";


/**
 * Contains all clip data
 */
export class ClipStore {
    source: Map<ID, SourceClip>;
    composited: Map<ID, CompositedClip>;

    constructor(source: Map<ID, SourceClip>, composited: Map<ID, CompositedClip>) {
        this.source = source;
        this.composited = composited;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.source, obj.composited)) {
            throw new Error("Could not deserialise! Malformed object");
        }

        let source = new Map();
        for (let id in obj.source) {
            source.set(id, SourceClip.deserialise(obj.source[id]));
        }
        let composited = new Map();
        for (let id in obj.composited) {
            composited.set(id, CompositedClip.deserialise(obj.composited[id]));
        }
        return new ClipStore(source, composited);
    }

    serialise() {

        let source: any = {};
        for (let [k, v] of this.source.entries()) {
            source[k] = v;
        }
        let composited: any = {};
        for (let [k, v] of this.composited.entries()) {
            composited[k] = v;
        }
        return { source, composited };
    }
}


/**
 * The container for the persistent application state
 */
export default class Store {
    nodes: Map<ID, EditorNode>;
    clips: ClipStore;
    pipeline: Pipeline;

    constructor(
        nodes?: Map<ID, EditorNode>,
        clips?: ClipStore,
        pipeline?: Pipeline,

    ) {
        if (!nodes) {
            nodes = new Map();
            clips = new ClipStore(new Map(), new Map());
            pipeline = new Pipeline([]);
        }
        this.nodes = nodes;
        this.clips = clips;
        this.pipeline = pipeline;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.nodes, obj.clips, obj.pipeline)) {
            throw new Error("Could not deserialise! Malformed object");
        }

        let nodes = new Map();
        for (let id in obj.nodes) {
            nodes.set(id, EditorNode.deserialise(obj.nodes[id]));
        }

        return new Store(nodes, ClipStore.deserialise(obj.clips), Pipeline.deserialise(obj.pipeline));
    }
    serialise() {

        let nodes: any = {};
        for (let [k, v] of this.nodes.entries()) {
            nodes[k] = v.serialise();
        }
        return {
            nodes,
            clips: this.clips.serialise(),
            pipeline: this.pipeline,
        }
    }

    /**
     * A helper function to simplify code in many places
     */
    static getCurrentStore(): Store {
        return EventBus.getValue(EventBus.GETTERS.APP.STORE);
    }
}