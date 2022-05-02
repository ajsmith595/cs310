import { faFileImport, faLayerGroup, faPhotoVideo, faPlus, faPlusSquare } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { ClipIdentifier, CompositedClip, SourceClip } from '../../classes/Clip';
import Communicator from '../../classes/Communicator';
import EventBus from '../../classes/EventBus';
import Store from '../../classes/Store';
import { v4 } from 'uuid';
import EditorNode, { Position } from '../../classes/Node';
import ClipComponent from './ClipComponent';


interface Props {
}

interface State {
	openTab: 'source' | 'composited',
}

class MediaImporter extends React.Component<Props, State> {
	private references: {
		composited: { [k: string]: React.RefObject<ClipComponent> }
	} = { composited: {} };
	constructor(props: Props) {
		super(props);

		this.state = {
			openTab: 'source',
		};



		this.onImportMediaButtonClick = this.onImportMediaButtonClick.bind(this);
	}

	componentDidMount(): void {
	}

	setOpenTab(t: 'source' | 'composited') {
		this.setState({
			openTab: t
		})
	}

	onImportMediaButtonClick() {
		this.setOpenTab('source');
		Communicator.invoke('import_media'); // will open a file dialog to allow users to import media
	}

	onCreateCompositedClipButtonClick() {
		this.setOpenTab('composited');
		Communicator.invoke("create_composited_clip");
	}


	render() {

		let tabSelection = (type: 'source' | 'composited', title: string, className = "") => {
			return (
				<button
					className={
						"text-xs font-bold uppercase px-1 py-1 shadow-lg rounded block leading-normal flex-grow border " + className + " " +
						(this.state.openTab === type
							? "text-white bg-pink-600 dark:text-white dark:bg-gray-800 border-red-800"
							: "text-pink-600 bg-white dark:text-gray-400 dark:bg-gray-800 border-transparent")
					}
					onClick={e => {
						e.preventDefault();
						this.setOpenTab(type);
					}}
					data-toggle="tab"
					role="tablist"
				>
					<FontAwesomeIcon className="mr-2" icon={type == 'source' ? faPhotoVideo : faLayerGroup} />
					{title}
				</button>
			);
		}



		let files = [];
		let store = Store.getCurrentStore();
		if (this.state.openTab == 'source') {
			for (let [id, source_clip] of store.clips.source) {
				files.push(
					<ClipComponent key={"source_" + id} clip={source_clip} />
				);
			}
		}
		else {
			for (let [id, composited_clip] of store.clips.composited) {
				if (!this.references.composited[id]) {
					this.references.composited[id] = React.createRef();
				}
				files.push(
					<ClipComponent key={"composited" + id} clip={composited_clip} ref={this.references.composited[id]} />
				);
			}
		}


		return <div className="flex w-full h-full flex-col gap-2">
			<div className="flex">
				<button
					className={"text-xl px-4 font-bold uppercase shadow-lg rounded rounded-r-none block leading-normal border border-r-0 text-white"
						+ (this.state.openTab === 'composited'
							? "text-white bg-pink-600 dark:text-white border-red-800"
							: "text-pink-600 bg-white dark:text-gray-400  border-transparent")}
					onClick={() => this.onCreateCompositedClipButtonClick()}
					data-toggle="tab"
					role="tablist"
				><FontAwesomeIcon icon={faPlus} /></button>
				{tabSelection('composited', 'Composited Clips', 'rounded-l-none')}
				{tabSelection('source', 'Source Clips', "rounded-r-none")}
				<button
					className={"text-lg px-4 font-bold uppercase shadow-lg rounded rounded-l-none block leading-normal border border-l-0 text-white"
						+ (this.state.openTab === 'source'
							? "text-white bg-pink-600 dark:text-white border-red-800"
							: "text-pink-600 bg-white dark:text-gray-400  border-transparent")}
					onClick={() => this.onImportMediaButtonClick()}
					data-toggle="tab"
					role="tablist"
				><FontAwesomeIcon icon={faFileImport} /></button>
			</div>
			<div className="flex-grow relative overflow-y-scroll">
				<table className='table-fixed w-full text-xs absolute border-collapse text-white'>
					<thead>
						<tr className="bg-black bg-opacity-20">
							<td className='w-2/3 text-left  border border-gray-800 small-caps'>file</td>
							<td className='w-1/6 text-left  border border-gray-800 small-caps'>duration</td>
							<td className='w-1/6 text-left  border border-gray-800 small-caps'>{this.state.openTab == 'composited' ? '' : 'status'}</td>
						</tr>
					</thead>
					<tbody>
						{files}
					</tbody>
				</table>
			</div>
		</div>
	}

}

export default MediaImporter;