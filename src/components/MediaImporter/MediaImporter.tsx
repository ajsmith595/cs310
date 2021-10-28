import { faFileImport, faPlusSquare } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { SourceClip } from '../../classes/Clip';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import StoreContext from '../../contexts/StoreContext';
import CompositedClipComponent from './CompositedClipComponent';
import SourceClipComponent from './SourceClipComponent';



interface Props {
	// props
}

interface State {
	openTab: 'source' | 'composited',
}

class MediaImporter extends React.Component<Props, State> {
	private references: {
		composited: { [k: string]: React.RefObject<CompositedClipComponent> }
	} = { composited: {} };
	constructor(props: Props) {
		super(props);

		this.state = {
			openTab: 'source',
		};



		this.onImportMediaButtonClick = this.onImportMediaButtonClick.bind(this);
	}
	setOpenTab(t: 'source' | 'composited') {
		this.setState({
			openTab: t
		})
	}

	onImportMediaButtonClick(setStore) {
		this.setOpenTab('source');
		Communicator.invoke('import_media', null, (data) => {
			setStore(Store.deserialise(data));
		});
	}

	onCreateCompositedClipButtonClick(setStore) {
		this.setOpenTab('composited');
		Communicator.invoke('create_composited_clip', null, ([new_id, store]) => {
			setStore(Store.deserialise(store));

			requestAnimationFrame(() => {
				this.references.composited[new_id].current.enableEditingMode();
			});
		});
	}


	render() {

		let tabSelection = (type: 'source' | 'composited', title: string, className = "") => {
			return (
				<button
					className={
						"text-xs font-bold uppercase px-5 py-3 shadow-lg rounded block leading-normal flex-grow border " + className + " " +
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
					{title}
				</button>
			);
		}


		return <StoreContext.Consumer>
			{({ value, setValue }) => {

				let files = [];
				if (this.state.openTab == 'source') {
					for (let [id, source_clip] of value.clips.source) {
						files.push(
							<SourceClipComponent key={id} clip={source_clip} />
						);
					}
				}
				else {
					for (let [id, composited_clip] of value.clips.composited) {
						if (!this.references.composited[id]) {
							this.references.composited[id] = React.createRef();
						}
						files.push(
							<CompositedClipComponent key={id} clip={composited_clip} ref={this.references.composited[id]} />
						);
					}
				}


				return <div className="flex w-full h-full flex-col gap-2">
					<div className="flex">
						<button
							className={"text-lg px-4 font-bold uppercase shadow-lg rounded rounded-r-none block leading-normal border border-r-0 text-white"
								+ (this.state.openTab === 'composited'
									? "text-white bg-pink-600 dark:text-white border-red-800"
									: "text-pink-600 bg-white dark:text-gray-400  border-transparent")}
							onClick={() => this.onCreateCompositedClipButtonClick(setValue)}
							data-toggle="tab"
							role="tablist"
						><FontAwesomeIcon icon={faPlusSquare} /></button>
						{tabSelection('composited', 'Composited Clips', 'rounded-l-none')}
						{tabSelection('source', 'Source Clips', "rounded-r-none")}
						<button
							className={"text-lg px-4 font-bold uppercase shadow-lg rounded rounded-l-none block leading-normal border border-l-0 text-white"
								+ (this.state.openTab === 'source'
									? "text-white bg-pink-600 dark:text-white border-red-800"
									: "text-pink-600 bg-white dark:text-gray-400  border-transparent")}
							onClick={() => this.onImportMediaButtonClick(setValue)}
							data-toggle="tab"
							role="tablist"
						><FontAwesomeIcon icon={faFileImport} /></button>
					</div>
					<div className="flex-grow border border-gray-800 relative overflow-y-scroll">
						<div className="h-full w-full absolute">
							{files}
						</div>
					</div>
				</div>
			}}
		</StoreContext.Consumer>;
	}

}

export default MediaImporter;