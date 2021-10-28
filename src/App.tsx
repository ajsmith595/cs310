import { faFolder, IconDefinition } from '@fortawesome/free-regular-svg-icons'
import { faCog, faFilm, faProjectDiagram } from '@fortawesome/free-solid-svg-icons'
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import React from 'react'
import MediaImporter from './components/MediaImporter/MediaImporter'
import NodeEditor from './components/NodeEditor'
import PropertiesPanel from './components/PropertiesPanel'
import VideoPreview from './components/VideoPreview'
import { appWindow } from '@tauri-apps/api/window';
import StoreContext from './contexts/StoreContext'
import Store from './classes/Store'
import Communicator from './classes/Communicator'

function Section(props: { width: string, height: string, text: string, children: any, icon: IconDefinition, className?: string }) {
	let children = props.children;
	if (!React.Children.count(children)) {
		children = <div className="flex items-center w-full h-full">
			<p className="dark:text-white text-center w-full text-4xl">NOT YET IMPLEMENTED</p>
		</div>;
	}
	return (<div className={`${props.width} ${props.height} p-3 border-gray-800 dark:border-gray-500 border ${props.className || ''} align-top inline-flex flex-col`}>
		<div>
			<h1 className="font-bold text-black dark:text-white text-xl"><FontAwesomeIcon icon={props.icon} className="mr-2" />{props.text.toUpperCase()}</h1>
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

interface State {
	Store: Store
}

class App extends React.Component<Props, State> {

	cache: Map<string, any> = new Map();

	constructor(props: Props) {
		super(props);

		this.state = {
			Store: null
		}

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
			this.setState({
				Store: Store.deserialise(data)
			})
		});
	}

	render() {

		if (this.state.Store) {

			return (
				<StoreContext.Provider value={{
					value: this.state.Store,
					setValue: (val) => this.setState({
						Store: val
					})
				}}>
					<div className="h-screen w-screen flex flex-col">
						{/* <div style={{ userSelect: 'none' }} className="border-red-500 w-full" onMouseDown={(e) => this.onClick(e)}>TEST DRAG</div> */}
						<div className="dark:bg-gray-700 flex-grow">
							<Section width="w-1/2" height="h-2/5" text="media importer" icon={faFolder}>
								<MediaImporter cache={this.cache} />
							</Section>
							<Section width="w-1/2" height="h-2/5" text="video preview" icon={faFilm} className="border-l-0">
								{/* <VideoPreview /> */}
							</Section>
							<Section width="w-3/4" height="h-3/5" text="node editor" icon={faProjectDiagram} className="border-t-0">
								{/* <NodeEditor /> */}
							</Section>
							<Section width="w-1/4" height="h-3/5" text="properties" icon={faCog} className="border-t-0 border-l-0">
								{/* <PropertiesPanel /> */}
							</Section>
						</div>
					</div>
				</StoreContext.Provider>
			)
		}
		return <h1>Loading...</h1>;
	}
}

export default App