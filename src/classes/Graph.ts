import { ID } from "./Communicator";

type Node = ID;

interface Edge {
	from: ID,
	to: ID,
}

export default class Graph {
	private nodes: Array<Node>;
	private edges: Array<Edge>;

	public constructor() {
		this.nodes = [];
		this.edges = [];
	}

	public addNode(node: Node) {
		this.nodes.push(node);
	}

	public addEdge(from: Node, to: Node) {
		this.edges.push({
			from,
			to
		});
	}

	public getIncomingNodes(node: Node, edges = this.edges) {
		return edges.filter(e => e.to = node).map(e => e.from);
	}
	public getOutgoingNodes(node: Node, edges = this.edges) {
		return edges.filter(e => e.from = node).map(e => e.to);
	}

	public isAcyclic() {

		//let nodes_copy = this.nodes.slice();
		let edges_copy = this.edges.slice(0);


		// we do a topological sort, and if it can't do that, then it's got a cycle

		let L = [];
		let Q = this.nodes.filter(e => this.getIncomingNodes(e).length == 0);
		// Q now contains all nodes with no incoming edges

		while (Q.length > 0) {
			let n = Q.pop();
			L.push(n);
			let outgoing = this.getOutgoingNodes(n, edges_copy);
			for (let m of outgoing) {
				edges_copy = edges_copy.filter(e => !(e.from == n && e.to == m));
				let incoming = this.getIncomingNodes(m, edges_copy);
				if (incoming.length == 0) {
					Q.push(m);
				}
			}
		}
		if (edges_copy.length > 0) {
			return false;
		}
		return true;
	}



}