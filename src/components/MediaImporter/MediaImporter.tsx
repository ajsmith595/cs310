import { faFileImport } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { SourceClip } from '../../classes/Clip';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import StoreContext from '../../contexts/StoreContext';
import SourceClipComponent from './SourceClipComponent';



interface Props {
	// props
}

interface State {
	openTab: 'source' | 'composited',
}

class MediaImporter extends React.Component<Props, State> {
	constructor(props: Props) {
		super(props);

		this.state = {
			openTab: 'source',
		};

		this.onImportMediaButtonClick = this.onImportMediaButtonClick.bind(this);
		this.changeClipName = this.changeClipName.bind(this);
	}
	setOpenTab(t: 'source' | 'composited') {
		this.setState({
			openTab: t
		})
	}

	onImportMediaButtonClick(setStore) {
		this.setOpenTab('source');
		Communicator.invoke('import_media', null, (data) => {
			console.log("new data");
			console.log(data);
			setStore(Store.deserialise(data));
		});
	}


	changeClipName(id, newName, setStore) {
		Communicator.invoke('change_clip_name', {
			clipType: 'source',
			id: id,
			name: newName
		}, (data) => {
			setStore(Store.deserialise(data));
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
							<SourceClipComponent clip={source_clip} />
						);
					}
				}


				return <div className="flex w-full h-full flex-col gap-2">
					<div className="flex">
						{tabSelection('composited', 'Composited Clips')}
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