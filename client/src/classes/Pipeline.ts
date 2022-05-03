import { ID } from "./Communicator";
import Graph from "./Graph";
import Store from "./Store";
import Utils from "./Utils";


/**
 * Specifies what node and input/output one end of the link connects to
 */
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


/**
 * Specifies a link between two nodes
 */
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


/**
 * Contains the set of links that define the pipeline - the nodes are stored in the more global `Store`.
 */
export default class Pipeline {
    links: Array<Link>;

    constructor(links: Array<Link>) {
        this.links = links;
    }
    static deserialise(obj: any) {
        if (Utils.propsUndefined(obj.links)) {
            throw new Error("Could not deserialise");
        }
        let links: Array<Link> = [];
        for (let o of obj.links) {
            links.push(Link.deserialise(o));
        }

        return new Pipeline(links);
    }


    /**
     * Returns true if a particular node and input/output combination has any link connected to it
     */
    containsLinkForNodeProperty(node_id, property) {
        for (let link of this.links) {
            if ((link.from.node_id == node_id && link.from.property == property)
                || (link.to.node_id == node_id && link.to.property == property)) {
                return true;
            }
        }
        return false;
    }


    /**
     * Checks if adding a particular node will cause a cycle in the graph
     */
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