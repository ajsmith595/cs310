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
	return (<div className={`${props.width} ${props.height} p-3 border-gray-800 dark:border-gray-500 border ${props.className || ''} align-top inline-flex flex-col`}>
		<div>
			<h1 className="font-bold text-black dark:text-white text-xl"><FontAwesomeIcon icon={props.icon} className="mr-2" />{props.text.toUpperCase()}
				<span className="float-right">{props.rightContent}</span>
			</h1>
			<hr className="border-gray-800 dark:border-gray-500 my-2" />
		</div>
		<div className="flex-grow">
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
	selection: Selectable
}

class App extends React.Component<Props, State> {

	cache: Map<string, any> = new Map();
	nodeEditor: React.RefObject<NodeEditor>;

	constructor(props: Props) {
		super(props);

		this.state = {
			Store: null,
			selection: null
		}
		this.nodeEditor = React.createRef<NodeEditor>();

		this.onClick = this.onClick.bind(this);
	}

	async onClick(e: React.MouseEvent<HTMLDivElement, MouseEvent>) {
		if (e.detail == 1) {
			appWindow.startDragging();
		} else if (e.detail == 2) { // Double click
			appWindow.toggleMaximize();
		}
	}

	componentDidMount() {
		Communicator.invoke('get_initial_data', null, (data) => {
			console.log(data);

			let node_register = data[1];
			console.log(node_register);

			for (let node_type in node_register) {
				EditorNode.NodeRegister.set(node_type, node_register[node_type]);
			}
			this.setState({
				Store: Store.deserialise(data[0])
			})
		});

		EventBus.registerGetter(EventBus.GETTERS.APP.CURRENT_SELECTION, () => {
			return this.state.selection;
		})

		EventBus.on(EventBus.EVENTS.APP.SET_SELECTION, (value: Selectable) => {
			this.setState({
				selection: value
			});
		});

		EventBus.registerGetter(EventBus.GETTERS.APP.STORE, () => {
			return this.state.Store;
		});
		EventBus.on(EventBus.EVENTS.APP.SET_STORE, (value: Store) => {
			this.setState({
				Store: value
			});
			Communicator.invoke('store_update', {
				store: value.serialise()
			});
		});
		EventBus.on(EventBus.EVENTS.APP.SET_STORE_UI, (value: any) => {
			this.setState({
				Store: value
			});
		});

		EventBus.on(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, () => {
			this.forceUpdate();
		});
	}

	componentWillUnmount() {
		EventBus.unregisterGetter(EventBus.GETTERS.APP.STORE);
		EventBus.unregisterGetter(EventBus.GETTERS.APP.CURRENT_SELECTION);
	}

	render() {

		if (this.state.Store) {
			let firstClip: CompositedClip = this.state.Store.clips.composited.values().next().value;

			return (
				<div className="h-screen w-screen flex flex-col">
					{/* <div style={{ userSelect: 'none' }} className="border-red-500 w-full" onMouseDown={(e) => this.onClick(e)}>TEST DRAG</div> */}
					<div className="dark:bg-gray-700 flex-grow">
						<Section width="w-1/2" height="h-2/5" text="media importer" icon={faFolder}>
							<MediaImporter cache={this.cache} />
						</Section>
						<Section width="w-1/2" height="h-2/5" text="video preview" icon={faFilm} className="border-l-0">
							{/* <VideoPreview /> */}
						</Section>
						<Section width="w-3/4" height="h-3/5" text="node editor" icon={faProjectDiagram} className="border-t-0" rightContent={<p>Group: {EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP)}</p>}>
							<div className="relative h-full w-full">
								<div className="absolute z-20 right-2 top-2">
									<NodeAddMenu />
								</div>
								<NodeEditor ref={this.nodeEditor} initial_group={firstClip.getClipGroup()} />
							</div>
						</Section>
						<Section width="w-1/4" height="h-3/5" text="properties" icon={faCog} className="border-t-0 border-l-0">
							<PropertiesPanel />
						</Section>
					</div>
				</div>
			)
		}
		return <h1>Loading...</h1>;
	}
}

export default App