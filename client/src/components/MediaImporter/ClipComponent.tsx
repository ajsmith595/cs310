import { faFileImport } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import { CompositedClip, SourceClip } from '../../classes/Clip';
import { faEdit } from '@fortawesome/free-regular-svg-icons';
import EventBus from '../../classes/EventBus';
import Utils from '../../classes/Utils';
import { fs } from '@tauri-apps/api';



interface Props {
    cache?: Map<string, any>;
    clip: CompositedClip | SourceClip
}

interface State {
    editing: boolean,
    thumbnailData: string
}

class ClipComponent extends React.Component<Props, State> {
    private inputRef = React.createRef<HTMLInputElement>();
    constructor(props: Props) {
        super(props);

        this.state = {
            editing: false,
            thumbnailData: ""
        };
        this.changeClipName = this.changeClipName.bind(this);
        this.enableEditingMode = this.enableEditingMode.bind(this);
        this.disableEditingMode = this.disableEditingMode.bind(this);
        this.openInEditor = this.openInEditor.bind(this);
        this.onDragStart = this.onDragStart.bind(this);
        this.selectClip = this.selectClip.bind(this);
    }


    componentDidMount() {
        if (this.props.clip instanceof CompositedClip) return;

        if (!this.props.cache.get("clips")) {
            this.props.cache.set("clips", {});
        }
        if (!this.props.cache.get("clips").source) {
            this.props.cache.get("clips").source = {};
        }
        if (!this.props.cache.get("clips").source[this.props.clip.id]) {
            this.props.cache.get("clips").source[this.props.clip.id] = {
                thumbnail_data: null
            };
        }
        if (!this.props.cache.get("clips").source[this.props.clip.id].thumbnail_data) {
            if (this.props.clip.thumbnail_location) {
                fs.readBinaryFile(this.props.clip.thumbnail_location).then(data => {
                    let new_data = Utils.bytesToBase64(data);
                    this.props.cache.get("clips").source[this.props.clip.id].thumbnail_data = new_data;
                    this.setState({
                        thumbnailData: new_data
                    });
                }).catch(e => {
                    console.log("Thumbnail failure!");
                    console.log(e);
                });
            }
        }
        else {
            this.setState({
                thumbnailData: this.props.cache.get("clips").source[this.props.clip.id].thumbnail_data
            });
        }
    }


    changeClipName(newName) {
        let store: Store = EventBus.getValue(EventBus.GETTERS.APP.STORE);

        let map = store.clips[(this.props.clip instanceof SourceClip) ? 'source' : 'composited'];
        map.get(this.props.clip.id).name = newName.trim();
        EventBus.dispatch(EventBus.EVENTS.APP.SET_STORE, store);
    }

    enableEditingMode() {
        this.setState({
            editing: true
        });

        requestAnimationFrame(() => {
            this.inputRef.current.focus();
        });
    }
    disableEditingMode() {
        if (this.inputRef.current) {
            this.changeClipName(this.inputRef.current.value);
        }
        this.setState({
            editing: false
        })
    }

    selectClip() {
        EventBus.dispatch(EventBus.EVENTS.APP.SET_SELECTION, this.props.clip);
    }

    openInEditor() {
        if (this.props.clip instanceof SourceClip) return;

        let group = this.props.clip.getClipGroup();

        EventBus.dispatch(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, group);
    }


    onDragStart(e: React.DragEvent) {
        e.dataTransfer.setData('application/json', JSON.stringify(this.props.clip.getIdentifier()));
        e.dataTransfer.dropEffect = 'link';
    }

    render() {
        let text = (
            <div>
                <h1 className="text-gray-200 text-xl inline" onDoubleClick={this.enableEditingMode}>{this.props.clip.name.replaceAll(' ', '\u00a0')}</h1>
                <button className="inline pt-2 ml-3 text-sm text-blue-600" onClick={this.enableEditingMode}><FontAwesomeIcon icon={faEdit} /></button>
            </div>
        );
        if (this.state.editing) {
            text = <input ref={this.inputRef} type="text" className="text-gray-200 bg-transparent border-0 text-xl focus:outline-none w-full"
                defaultValue={this.props.clip.name} onBlur={() => this.disableEditingMode()} onKeyDown={(e) => {
                    if (e.key == "Enter") {
                        this.disableEditingMode();
                    }
                }} />;
        }

        let extraDisplay = null;
        if (this.props.clip instanceof SourceClip) {
            extraDisplay = <p className="text-gray-400 text-xs">{this.props.clip.file_location}</p>;
        }
        else {
            extraDisplay = <button className="text-xs p-1 bg-blue-600 text-white" onClick={this.openInEditor}>Open</button>;
        }


        let img = <img src="https://via.placeholder.com/1920x1080" className="max-h-16" />;
        if (this.props.clip instanceof SourceClip && this.state.thumbnailData) {
            img = <img src={"data:image/jpeg;base64," + this.state.thumbnailData} className="max-h-16" />;
        }

        let type_indicator = null;
        if (this.props.clip instanceof SourceClip) {
            let colour = Utils.Colours.Unknown;
            if (this.props.clip.file_location.endsWith("mp3")) {
                colour = Utils.Colours.Audio;
            }
            else {
                colour = Utils.Colours.Video;
            }
            type_indicator = <div className={`w-1/4 bg-${colour} rounded-full h-1`}></div>;
        }

        let isSelected = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION) == this.props.clip;
        return <div className={`gap-2 inline-flex w-1/2 cursor-pointer hover:bg-white hover:bg-opacity-10 transition-colors rounded border ${isSelected ? 'border-pink-600' : 'border-transparent'}`}
            draggable="true"
            onDragStart={this.onDragStart}
            onClick={this.selectClip}>
            <div>
                {img}
            </div>
            <div className="flex items-center">
                <div>
                    {text}
                    {extraDisplay}
                    {type_indicator}
                </div>
            </div>
        </div>

    }

}

export default ClipComponent;