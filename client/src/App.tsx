import { faFolder, IconDefinition } from '@fortawesome/free-regular-svg-icons'
import { faCog, faFilm, faProjectDiagram } from '@fortawesome/free-solid-svg-icons'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import React, { ReactNode } from 'react'
import MediaImporter from './components/MediaImporter/MediaImporter'
import NodeEditor from './components/NodeEditor/NodeEditor'
import PropertiesPanel from './components/PropertiesPanel'
import VideoPreview from './components/VideoPreview'
import { appWindow } from '@tauri-apps/api/window';
import Store from './classes/Store'
import Communicator from './classes/Communicator'
import EditorNode from './classes/Node'
import NodeAddMenu from './components/NodeEditor/NodeAddMenu'
import EventBus from './classes/EventBus'
import { CompositedClip, SourceClip } from './classes/Clip'

function Section(props: { width: string, height: string, text: string, children: any, icon: IconDefinition, className?: string, rightContent?: ReactNode }) {
	let children = props.children;
	if (!React.Children.count(children)) {
		children = <div className="flex items-center w-full h-full">
			<p className="dark:text-white text-center w-full text-4xl">NOT YET IMPLEMENTED</p>
		</div>;
	}
	return (<div className={`${props.width} ${props.height} p-1 border-gray-800 dark:border-gray-500 border ${props.className || ''} align-top inline-flex flex-col`}>
		<div>
			<h1 className="font-bold text-black dark:text-white text-xs"><FontAwesomeIcon icon={props.icon} className="mr-2" />{props.text.toUpperCase()}
				<span className="float-right">{props.rightContent}</span>
			</h1>
			<hr className="border-gray-800 dark:border-gray-500 my-1" />
		</div>
		<div style={{ flex: "1 1 auto" }} className="min-h-0">
			{children}
		</div>
	</div>)
}


interface Props {
	// props
}


type Selectable = EditorNode | SourceClip | CompositedClip;

interface State {
	Store: Store,
	selection: Selectable,
	initialConnectionDone: boolean;
	connectionError: string;
}

class App extends React.Component<Props, State> {

	cache: Map<string, any> = new Map();
	nodeEditor: React.RefObject<NodeEditor>;

	constructor(props: Props) {
		super(props);

		this.state = {
			Store: null,
			selection: null,
			initialConnectionDone: false,
			connectionError: null
		}
		this.nodeEditor = React.createRef<NodeEditor>();

		this.onClick = this.onClick.bind(this);
		this.connectionStatusUpdate = this.connectionStatusUpdate.bind(this);


		this.setSelectionHandler = this.setSelectionHandler.bind(this);
		this.setStoreHandler = this.setStoreHandler.bind(this);
		this.setStoreUIHandler = this.setStoreUIHandler.bind(this);
		this.changeGroupHandler = this.changeGroupHandler.bind(this);
	}

	async onClick(e: React.MouseEvent<HTMLDivElement, MouseEvent>) {
		if (e.detail == 1) {
			appWindow.startDragging();
		} else if (e.detail == 2) { // Double click
			appWindow.toggleMaximize();
		}
	}

	connectionStatusUpdate(connectionStatus) {
		if (connectionStatus === 'InitialisingConnection') {
			this.setState({
				connectionError: null,
				initialConnectionDone: false
			});
		}
		else if (connectionStatus === 'Connected') {

			if (!this.state.initialConnectionDone) {
				Communicator.invoke('get_initial_data', null, (data) => {
					let node_register = data[1];
					EditorNode.deserialiseRegister(node_register);
					this.setState({
						Store: Store.deserialise(data[0])
					})
				});
			}

			this.setState({
				connectionError: null,
				initialConnectionDone: true
			});
		}
		else if (Object.keys(connectionStatus)[0] === 'InitialConnectionFailed') {
			this.setState({
				connectionError: connectionStatus['InitialConnectionFailed'],
				initialConnectionDone: false
			});
		}
		else if (Object.keys(connectionStatus)[0] === 'ConnectionFailed') {
			this.setState({
				connectionError: connectionStatus['ConnectionFailed'],
				initialConnectionDone: true
			});
		}
	}

	setSelectionHandler(value: Selectable) {
		this.setState({
			selection: value
		});
	}
	setStoreHandler(value: Store) {
		this.setState({
			Store: value
		});
		Communicator.invoke('store_update', {
			store: value.serialise()
		});
	}
	setStoreUIHandler(value: Store) {
		this.setState({
			Store: value
		});
	}
	changeGroupHandler() {
		this.forceUpdate();
	}

	componentDidMount() {

		Communicator.invoke('get_connection_status', null, this.connectionStatusUpdate);
		Communicator.on('connection-status', this.connectionStatusUpdate);

		Communicator.on('store-update', (store) => {
			let _store = Store.deserialise(store);
			console.log(_store);
			this.setState({
				Store: _store
			});
		})


		// Getters
		EventBus.registerGetter(EventBus.GETTERS.APP.STORE, () => this.state.Store);
		EventBus.registerGetter(EventBus.GETTERS.APP.CURRENT_SELECTION, () => this.state.selection);

		// Events
		EventBus.on(EventBus.EVENTS.APP.SET_SELECTION, this.setSelectionHandler);
		EventBus.on(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, this.changeGroupHandler);
	}

	componentWillUnmount() {

		Communicator.off('connection-status', this.connectionStatusUpdate);

		EventBus.unregisterGetter(EventBus.GETTERS.APP.STORE);
		EventBus.unregisterGetter(EventBus.GETTERS.APP.CURRENT_SELECTION);

		EventBus.remove(EventBus.EVENTS.APP.SET_SELECTION, this.setSelectionHandler);
		EventBus.remove(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, this.changeGroupHandler);
	}

	render() {
		if (this.state.Store && this.state.initialConnectionDone) {
			let firstClip: CompositedClip = this.state.Store.clips.composited.values().next().value;
			let firstClipGroup = "";
			if (firstClip) {
				firstClipGroup = firstClip.getClipGroup();
			}

			return (
				<div className="h-screen w-screen flex flex-col dark:bg-gray-700">
					{/* <div style={{ userSelect: 'none' }} className="border-red-500 w-full" onMouseDown={(e) => this.onClick(e)}>TEST DRAG</div> */}
					<div className="dark:bg-gray-700 flex-grow max-h-full">
						<Section width="w-1/2" height="h-2/5" text="media importer" icon={faFolder}>
							<MediaImporter cache={this.cache} />
						</Section>
						<Section width="w-1/2" height="h-2/5" text="video preview" icon={faFilm} className="border-l-0">
							<VideoPreview />
						</Section>
						<Section width="w-3/4" height="h-3/5" text="node editor" icon={faProjectDiagram} className="border-t-0" rightContent={<p>Group: {EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP)}</p>}>
							<div className="relative h-full w-full">
								<div className="absolute z-20 right-2 top-2">
									<NodeAddMenu />
								</div>
								<NodeEditor ref={this.nodeEditor} initial_group={firstClipGroup} />
							</div>
						</Section>
						<Section width="w-1/4" height="h-3/5" text="properties" icon={faCog} className="border-t-0 border-l-0">
							<PropertiesPanel cache={this.cache} />
						</Section>
						{/* <div className="absolute right-1 top-1 bg-white bg-opacity-40 hover:bg-opacity-80 px-2 rounded">
							<p className="text-green-600">Connected to server</p>
						</div> */}
					</div>
				</div>
			);
		}

		let content = null;


		if (!this.state.initialConnectionDone) {

			if (this.state.connectionError) {
				content = <>
					<p>Connection failed</p>
					<small>(trying again in 5 seconds)</small>
					<p>Error: {this.state.connectionError}</p>
				</>
			}
			else {
				content = <>
					<p>Connecting</p>
					<FontAwesomeIcon icon={faCog} className="animate-spin text-3xl" />
				</>
			}
		}
		else if (!this.state.Store) {
			content = <>
				<p>Connected to server, loading user interface</p>
				<FontAwesomeIcon icon={faCog} className="animate-spin text-3xl" />
			</>
		}

		return (
			<div className="flex h-screen w-screen">
				<div className="m-auto w-1/2 text-center">
					{content}
				</div>
			</div>
		);
	}
}

export default App