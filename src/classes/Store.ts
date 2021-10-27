import { ID } from "./Communicator";
import Node, { PipeableType } from './Node';
import Pipeline from "./Pipeline";
import { CompositedClip, SourceClip } from "./Clip";
import Utils from "./Utils";
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
}

export default class Store {
    nodes: Map<ID, Node>;
    clips: ClipStore;
    pipelines: Map<ID, Pipeline>;
    medias: Map<ID, PipeableType>;

    constructor(
        nodes?: Map<ID, Node>,
        clips?: ClipStore,
        pipelines?: Map<ID, Pipeline>,
        medias?: Map<ID, PipeableType>,

    ) {
        if (!nodes) {
            nodes = new Map();
            clips = new ClipStore(new Map(), new Map());
            pipelines = new Map();
            medias = new Map();
        }
        this.nodes = nodes;
        this.clips = clips;
        this.pipelines = pipelines;
        this.medias = medias;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.nodes, obj.clips, obj.pipelines, obj.medias)) {
            throw new Error("Could not deserialise! Malformed object");
        }

        let nodes = new Map();
        for (let id in obj.nodes) {
            nodes.set(id, Node.deserialise(obj.nodes[id]));
        }

        let pipelines = new Map();
        for (let id in obj.pipelines) {
            pipelines.set(id, Pipeline.deserialise(obj.pipelines[id]));
        }

        let medias = new Map();
        for (let id in obj.medias) {
            medias.set(id, obj.medias[id]);
        }
        return new Store(nodes, ClipStore.deserialise(obj.clips), pipelines, medias);
    }
}