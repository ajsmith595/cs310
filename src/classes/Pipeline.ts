import { ID } from "./Communicator";
import Graph from "./Graph";
import Store from "./Store";
import Utils from "./Utils";
// import { DiGraph, Graph } from 'js-graph-algorithms';

export class LinkEndpoint {
    node_id: ID;
    property: string;

    constructor(node_id: ID, property: string) {
        this.node_id = node_id;
        this.property = property;
    }

    get id() {
        return this.node_id + "." + this.property;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.node_id, obj.property)) {
            throw new Error("Could not deserialise");
        }

        return new LinkEndpoint(obj.node_id, obj.property);
    }
}

export class Link {
    from: LinkEndpoint;
    to: LinkEndpoint;

    constructor(from: LinkEndpoint, to: LinkEndpoint) {
        this.from = from;
        this.to = to;
    }

    get id() {
        return this.from.id + "-" + this.to.id;
    }

    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.from, obj.to)) {
            throw new Error("Could not deserialise");
        }

        return new Link(LinkEndpoint.deserialise(obj.from), LinkEndpoint.deserialise(obj.to));
    }
}


export default class Pipeline {
    links: Array<Link>;
    target_node_id: ID;

    constructor(links: Array<Link>, target_node_id: ID) {
        this.links = links;
        this.target_node_id = target_node_id;
    }
    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.links, obj.target_node_id)) {
            throw new Error("Could not deserialise");
        }
        let links: Array<Link> = [];
        for (let o of obj.links) {
            links.push(Link.deserialise(o));
        }

        return new Pipeline(links, obj.target_node_id);
    }


    containsLinkForNodeProperty(node_id, property) {
        for (let link of this.links) {
            if ((link.from.node_id == node_id && link.from.property == property)
                || (link.to.node_id == node_id && link.to.property == property)) {
                return true;
            }
        }
        return false;
    }


    hasCyclesWithLink(store: Store, link_to_add: Link) {

        let g = new Graph();

        for (let [id, node] of store.nodes.entries()) {
            g.addNode(id);
        }

        let links_done = [];
        for (let link of this.links) {
            if (link.to.id == link_to_add.to.id) continue;
            let id = link.from.node_id + "_" + link.to.node_id;
            if (!links_done.includes(id)) {
                g.addEdge(link.from.node_id, link.to.node_id);
                links_done.push(id);
            }
        }
        let new_id = link_to_add.from.node_id + "_" + link_to_add.to.node_id;
        if (!links_done.includes(new_id)) {
            g.addEdge(link_to_add.from.node_id, link_to_add.to.node_id);
        }
        return !g.isAcyclic();

    }
}