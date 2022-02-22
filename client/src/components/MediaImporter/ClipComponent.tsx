import { faBox, faClosedCaptioning, faFileImport, faMusic, faVideo } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import { CompositedClip, SourceClip } from '../../classes/Clip';
import { faEdit, faFile } from '@fortawesome/free-regular-svg-icons';
import EventBus from '../../classes/EventBus';
import Utils from '../../classes/Utils';
import { fs } from '@tauri-apps/api';



interface Props {
    cache?: Map<string, any>;
    clip: CompositedClip | SourceClip
}

interface State {
    editing: boolean,
    thumbnailData: string,
    uploadProgress: number
}

class ClipComponent extends React.Component<Props, State> {
    private inputRef = React.createRef<HTMLInputElement>();
    constructor(props: Props) {
        super(props);

        this.state = {
            editing: false,
            thumbnailData: "",
            uploadProgress: 50
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

        Communicator.on('file-upload-progress', (data) => {
            let [id, percentage] = data;
            if (id == this.props.clip.id) {
                this.setState({
                    uploadProgress: percentage
                });
            }
        });

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


        let type_indicator = null;

        let type = this.props.clip.getType();
        if (type) {
            let colour = Utils.Colours.Unknown;
            let icon = faFile;
            if (type.video > 1) {
                colour = Utils.Colours.Container;
                icon = faBox;
            }
            else if (type.video == 1) {
                colour = Utils.Colours.Video;
                icon = faVideo;
            }
            else if (type.audio > 0) {
                colour = Utils.Colours.Audio;
                icon = faMusic;
            }
            else if (type.subtitles > 0) {
                colour = Utils.Colours.Subtitles;
                icon = faClosedCaptioning;
            }
            type_indicator = <FontAwesomeIcon icon={icon} className={`text-${colour} mr-2`} />;
        }
        else {
            this.props.clip.fetchType().then(e => this.forceUpdate());
        }


        let source_clip_location = null;

        if (this.props.clip instanceof SourceClip) {
            // source_clip_location = <span className="ml-2 text-gray-400 text-xs">({this.props.clip.file_location})</span>;
        }

        let text = (
            <div>

                <span className="text-gray-200 text-xs inline" onDoubleClick={this.enableEditingMode}>{type_indicator}{this.props.clip.name.replaceAll(' ', '\u00a0')}</span>
                <button className="inline ml-3 text-xs text-blue-600" onClick={this.enableEditingMode}><FontAwesomeIcon icon={faEdit} /></button>
                {source_clip_location}
            </div>
        );
        if (this.state.editing) {
            text = <div className="flex">{type_indicator}<input ref={this.inputRef} type="text" className="text-gray-200 bg-transparent border-0 text-xs focus:outline-none flex-1"
                defaultValue={this.props.clip.name} onBlur={() => this.disableEditingMode()} onKeyDown={(e) => {
                    if (e.key == "Enter") {
                        this.disableEditingMode();
                    }
                }} />
            </div>;
        }

        let extraDisplay = null;
        if (this.props.clip instanceof SourceClip) {
            extraDisplay = <p className="text-gray-400 text-xs">{this.props.clip.file_location}</p>;


            let width = this.state.uploadProgress + "%";
            extraDisplay = <div className='relative h-2 border border-black rounded'>
                <div className='bg-white left-0 absolute h-full rounded' style={{ width }}>
                </div>
            </div>
        }
        else {
            extraDisplay = <button className="text-xs p-1 bg-blue-600 text-white" onClick={this.openInEditor}>Open</button>;
        }


        let img = <img src="https://via.placeholder.com/1920x1080" className="max-h-16" />;
        if (this.props.clip instanceof SourceClip && this.state.thumbnailData) {
            img = <img src={"data:image/jpeg;base64," + this.state.thumbnailData} className="max-h-16" />;
        }

        let isSelected = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION) == this.props.clip;

        let durationString = '-';
        if (this.props.clip.getDuration()) {
            let ms = this.props.clip.getDuration();

            let s = ms / 1000;
            let hrs = Math.floor(s / 3600);
            let mins = Math.floor(s / 60) % 60;
            let seconds = Math.floor(s) % 60;

            durationString = hrs.toString().padStart(2, '0') + ":" + mins.toString().padStart(2, '0') + ":" + seconds.toString().padStart(2, '0');
        }

        let border = `border border-gray-800`;


        return <tr className={`gap-2 cursor-pointer ${isSelected ? 'bg-pink-600' : 'hover:bg-white hover:bg-opacity-10'} transition-colors`}
            draggable="true"
            onDragStart={this.onDragStart}
            onClick={this.selectClip}>
            {/* <div>
                {img}
            </div> */}
            <td className={border}>{text}</td>
            <td className={border}>{durationString}</td>
            <td className={border}>Status</td>
            {/* <div className="flex items-center">
                <div>
                    {text}
                    {extraDisplay}
                    {type_indicator}
                </div>



            </div> */}
        </tr>

    }

}

export default ClipComponent;

