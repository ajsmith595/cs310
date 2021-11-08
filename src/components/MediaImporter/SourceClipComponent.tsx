import { faFileImport } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import { SourceClip } from '../../classes/Clip';
import { faEdit } from '@fortawesome/free-regular-svg-icons';
import { fs } from '@tauri-apps/api';
import Utils from '../../classes/Utils';
import EventBus from '../../classes/EventBus';



interface Props {
    // props
    clip: SourceClip,
    cache?: Map<string, any>;
}

interface State {
    editing: boolean,
    thumbnailData: string
}

class SourceClipComponent extends React.Component<Props, State> {
    private inputRef = React.createRef<HTMLInputElement>();
    constructor(props: Props) {
        super(props);

        this.state = {
            editing: false,
            thumbnailData: null
        };
        this.changeClipName = this.changeClipName.bind(this);
        this.enableEditingMode = this.enableEditingMode.bind(this);
        this.disableEditingMode = this.disableEditingMode.bind(this);
        this.onDragStart = this.onDragStart.bind(this);
    }

    componentDidMount() {
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
        store.clips.source.get(this.props.clip.id).name = newName.trim();
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

    onDragStart(e: React.DragEvent) {
        e.dataTransfer.setData('application/json', JSON.stringify(this.props.clip.getIdentifier()));
        e.dataTransfer.dropEffect = 'link';
    }

    render() {
        let text = (
            <div>
                <h1 className="text-gray-200 text-lg inline" onDoubleClick={this.enableEditingMode}>{this.props.clip.name.replaceAll(' ', '\u00a0')}</h1>
                <button className="inline pt-2 ml-3 text-sm text-blue-600" onClick={this.enableEditingMode}><FontAwesomeIcon icon={faEdit} /></button>
            </div>
        );
        if (this.state.editing) {
            text = <input ref={this.inputRef} type="text" className="text-gray-200 bg-transparent border-0 text-lg focus:outline-none w-full"
                defaultValue={this.props.clip.name} onBlur={() => this.disableEditingMode()} onKeyDown={(e) => {
                    if (e.key == "Enter") {
                        this.disableEditingMode();
                    }
                }} />;
        }

        let ellipsisFileLocation = (l: string) => {
            const threshold = 45;
            if (l.length > threshold) {
                return l.substr(0, 3) + "..." + l.substr(l.length - (threshold - 7));
            }
            return l;
        }
        let img = <img src="https://via.placeholder.com/1920x1080" className="max-h-16" />;
        if (this.state.thumbnailData) {
            img = <img src={"data:image/jpeg;base64," + this.state.thumbnailData} className="max-h-16" />;
        }

        return <div className="gap-2 inline-flex w-1/2" draggable="true" onDragStart={this.onDragStart}>
            <div>
                {img}
            </div>
            <div className="flex items-center">
                <div>
                    {text}
                    <p className="text-gray-400 text-xs">{ellipsisFileLocation(this.props.clip.file_location)}</p>
                </div>
            </div>
        </div>

    }

}

export default SourceClipComponent;