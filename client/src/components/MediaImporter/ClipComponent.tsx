import { faBox, faClosedCaptioning, faExternalLinkSquareAlt, faFileImport, faMusic, faVideo } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../../classes/Communicator';
import Cache from '../../classes/Cache';
import { CompositedClip, SourceClip } from '../../classes/Clip';
import { faEdit, faFile } from '@fortawesome/free-regular-svg-icons';
import EventBus from '../../classes/EventBus';
import Utils from '../../classes/Utils';
import { fs } from '@tauri-apps/api';



interface Props {
    clip: CompositedClip | SourceClip
}

interface State {
    editing: boolean,
    thumbnailData: string, // allows thumbnails to be shown in the media importer (old)
    uploadProgress: number // contains the current upload progress of this clip (if a source clip)
}

class ClipComponent extends React.Component<Props, State> {
    private inputRef = React.createRef<HTMLInputElement>(); // Allows us to control the focus of the input to allow us to change the name of the clip


    constructor(props: Props) {
        super(props);

        this.state = {
            editing: false,
            thumbnailData: "",
            uploadProgress: 0
        };
        this.changeClipName = this.changeClipName.bind(this);
        this.enableEditingMode = this.enableEditingMode.bind(this);
        this.disableEditingMode = this.disableEditingMode.bind(this);
        this.openInEditor = this.openInEditor.bind(this);
        this.onDragStart = this.onDragStart.bind(this);
        this.selectClip = this.selectClip.bind(this);
    }



    componentDidMount() {
        if (this.props.clip instanceof CompositedClip) {

            EventBus.on(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, () => {
                setTimeout(() => this.forceUpdate(), 30);
            }); // If the group is changed, we need to force an update to ensure the `Open in Editor` buttons are correctly displaying

            return;
        }

        // If it's a source clip


        Communicator.on('file-upload-progress', (data) => {
            // Received when new file percentages are emitted by the Rust client
            let [id, percentage] = data;
            if (id == this.props.clip.id) {
                this.setState({
                    uploadProgress: percentage
                });
            }
        });


        let cacheID = this.props.clip.getThumbnailCacheID();

        // allows the thumbnail of the clip to be obtained in the background, if it is not available in the cache

        if (!Cache.get(cacheID)) {
            if (this.props.clip.thumbnail_location) {
                fs.readBinaryFile(this.props.clip.thumbnail_location).then(data => {
                    let new_data = Utils.bytesToBase64(data);
                    Cache.put(cacheID, new_data);
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
                thumbnailData: Cache.get(cacheID)
            });
        }
    }


    /**
     * Changes the appropriate clip's name, and saves the change to the Rust backend.
     */
    changeClipName(newName) {
        this.props.clip.name = newName;
        Communicator.invoke('update_clip', {
            clipId: this.props.clip.id,
            clipType: (this.props.clip instanceof SourceClip) ? 'Source' : 'Composited',
            clip: this.props.clip.serialise()
        });
    }

    /**
     * Enables the clip's name to be edited
     */
    enableEditingMode() {
        this.setState({
            editing: true
        });

        // Need to wait a frame for the input to be shown, then we can focus into the input
        requestAnimationFrame(() => {
            this.inputRef.current.focus();
        });
    }

    /**
     * Disables the editing mode, and saves the new clip name
     */
    disableEditingMode() {
        if (this.inputRef.current) {
            this.changeClipName(this.inputRef.current.value);
        }
        this.setState({
            editing: false
        })
    }

    /**
     * Selects the clip in the global context, so it can be used by the properties panel
     */
    selectClip() {
        EventBus.dispatch(EventBus.EVENTS.APP.SET_SELECTION, this.props.clip);
    }

    /**
     * Opens the relevant clip in the node editor, using the composited clip's group
     */
    openInEditor() {
        if (this.props.clip instanceof SourceClip) return;

        let group = this.props.clip.getClipGroup();

        EventBus.dispatch(EventBus.EVENTS.NODE_EDITOR.CHANGE_GROUP, group);
    }


    /**
     * Called when a particular clip starts being dragged; encodes the relevant clip data into the drag event so it can be picked up if the user drops it on a compatible target
     */
    onDragStart(e: React.DragEvent) {
        e.dataTransfer.setData('application/json', JSON.stringify(this.props.clip.getIdentifier().serialise()));
        e.dataTransfer.dropEffect = 'link';
    }

    render() {


        let type_indicator = null; // The icon displaying what type the clip is
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
            this.props.clip.fetchType().then(e => this.forceUpdate()); // If the type is not available, get the type, then refresh once it is available
        }

        let text = (
            <div className="flex-1">
                <span className="text-gray-200 text-xs inline" onDoubleClick={this.enableEditingMode}>{type_indicator}{this.props.clip.name.replaceAll(' ', '\u00a0')}</span>
                <button className="inline ml-3 text-xs text-blue-600" onClick={this.enableEditingMode}><FontAwesomeIcon icon={faEdit} /></button>
            </div>
        ); // the name of the clip, with the type indicator
        if (this.state.editing) {
            // if we're editing, instead show an input box to allow the user to modify the name of the clip


            text = <div className="flex flex-1">{type_indicator}<input ref={this.inputRef} type="text" className="text-gray-200 bg-transparent border-0 text-xs focus:outline-none flex-1"
                defaultValue={this.props.clip.name} onBlur={() => this.disableEditingMode()} onKeyDown={(e) => {
                    if (e.key == "Enter") {
                        this.disableEditingMode();
                    }
                }} />
            </div>;
        }


        let status = null; // The upload status of a source clip OR the button to open a composited clip in the editor
        if (this.props.clip instanceof SourceClip) {
            status = this.props.clip.status;
            if (status == 'Uploading') {
                status += " " + Math.round(this.state.uploadProgress) + "%";
            }
        }
        else {

            if (EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP) == this.props.clip.getClipGroup()) {
                status = <button className="text-xs disabled bg-green-600 text-white">In Editor</button>;
            }
            else {
                status = <button className="text-xs hover:bg-blue-500 bg-blue-600 text-white" onClick={this.openInEditor}>Open in Editor</button>;
            }

        }




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

        let isSelected = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION) == this.props.clip; // if it's selected, we highlight it

        return <tr className={`gap-2 cursor-pointer ${isSelected ? 'bg-pink-600' : 'hover:bg-white hover:bg-opacity-10'} transition-colors`}
            draggable="true"
            onDragStart={this.onDragStart}
            onClick={this.selectClip}>
            <td className={border}><div className='flex'>{text}</div></td>
            <td className={border}>{durationString}</td>
            <td className={border}>{status}</td>
        </tr>

    }

}

export default ClipComponent;

