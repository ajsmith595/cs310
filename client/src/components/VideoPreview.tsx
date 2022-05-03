import { faCircleNotch, faMusic, faPause, faPlay, faStepBackward, faStepForward } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../classes/Communicator';
import Store from '../classes/Store';
import { Mutex } from 'async-mutex';
import Utils from '../classes/Utils';
import EventBus from '../classes/EventBus';

interface Props {
}

interface State {
    clip: string,
    videoURL: string,
    playing: boolean,
    buffering: boolean,
}


//#region Video Preview Data: Parsing Wrapper
namespace VideoPreviewInputData {
    type VideoPreviewClipStatus = "NotRequested" | "LengthRequested" | {
        "Data": [
            number,
            string,
            boolean,
            Array<VideoPreviewChunkStatus>
        ]
    }
    export enum VideoPreviewChunkStatus {
        NotRequested = "NotRequested",
        Requested = "Requested",
        Generating = "Generating",
        Generated = "Generated",
        Downloading = "Downloading",
        Downloaded = "Downloaded"
    }

    export interface VideoPreviewData {
        [k: string]: VideoPreviewClipStatus
    }
}


namespace VideoPreviewClipStatus {
    export enum ChunkStatus {
        NotRequested = "NotRequested",
        Requested = "Requested",
        Generating = "Generating",
        Generated = "Generated",
        Downloading = "Downloading",
        Downloaded = "Downloaded",
        Loaded = "Loaded" // unique to front end - to signify that the chunk is loaded into MSE.
    }
    export interface Data {
        duration: number;
        codec: string;
        is_video: boolean;
        chunkData: Array<ChunkStatus>;
    }

}

type VideoPreviewClipStatus = "NotRequested" | "LengthRequested" | VideoPreviewClipStatus.Data;

//#endregion Video Preview Data: Parsing Wrapper


class VideoPreview extends React.Component<Props, State> {

    static CHUNK_LENGTH: number = 1; // the length in seconds of each chunk


    video_element_ref: React.RefObject<HTMLVideoElement>; // a reference to the main video element
    media_source: MediaSource; // the media source which will bind to the video element, and will contain the source buffers which are used to render the video    
    video_preview_data: Map<string, VideoPreviewClipStatus>; // from the Rust process
    change_lock: Mutex; // Prevents simulatenous modification of the media source, which can cause unexpected errors


    constructor(props: Props) {
        super(props);
        this.media_source = new MediaSource();
        this.video_element_ref = React.createRef();
        this.change_lock = new Mutex();
        this.video_preview_data = new Map();

        this.state = {
            clip: null,
            videoURL: URL.createObjectURL(this.media_source),
            playing: false,
            buffering: false
        }

        this.onSourceBufferUpdateEnd = this.onSourceBufferUpdateEnd.bind(this);
    }

    get currentTimestamp() {
        return this.video_element_ref.current?.currentTime || 0;
    }
    get duration() {
        if (this.state.clip) {
            let clip = Store.getCurrentStore().clips.composited.get(this.state.clip);
            if (clip) {
                if (clip.getDuration()) return clip.getDuration() / 1000;
            }
        }
        return this.video_element_ref.current?.duration;
    }

    get currentPercentage(): number {
        return this.currentTimestamp / this.duration * 100;
    }

    componentDidMount() {
        Communicator.on('video-preview-data-update', async (data) => {
            // Emitted when the Rust side updates the video preview data
            await this.videoPreviewDataUpdate(data);
        });

        Communicator.invoke('get_video_preview_data', {}, async (data) => {
            // Gets the initial video preview data
            await this.videoPreviewDataUpdate(data);
        });
    }

    async videoPreviewDataUpdate(data: VideoPreviewInputData.VideoPreviewData) {
        let release = await this.change_lock.acquire();
        for (let k of this.video_preview_data.keys()) {
            if (data[k] === undefined) {
                this.video_preview_data.delete(k);
            }
        }

        let do_clip_refresh = false;

        // Basically, parsing in the video preview data
        for (let k in data) {
            let value = data[k];
            if (typeof value == "string") {
                this.video_preview_data.set(k, value);
            }
            else {
                let duration = value.Data[0];
                let codec = value.Data[1];
                let is_video = value.Data[2];
                EventBus.dispatch('composited-clip-length', [k, duration]); // Updates the global context with the new composited clip duration
                let data = value.Data[3];

                let existing = this.video_preview_data.get(k);
                if (existing == null || typeof existing == "string" || existing.chunkData.length != data.length || existing.codec != codec || existing.is_video != is_video) {
                    this.video_preview_data.set(k, {
                        duration,
                        codec,
                        is_video,
                        chunkData: (data as Array<any>)
                    })

                    if (k == this.state.clip) {
                        do_clip_refresh = true;
                    }
                }
                else {
                    for (let i = 0; i < existing.chunkData.length; i++) {
                        let existingChunkData = existing.chunkData[i];
                        let newChunkData = data[i];

                        // If it's `Downloaded`, but we've already `Loaded` it into the media source, we just ignore updating this chunk - the Rust side is not aware of what chunks are `Loaded` by MSE
                        if (!(existingChunkData == VideoPreviewClipStatus.ChunkStatus.Loaded && newChunkData == VideoPreviewInputData.VideoPreviewChunkStatus.Downloaded)) {
                            existing.chunkData[i] = newChunkData as any;
                        }
                    }
                }
            }
        }
        release();
        if (do_clip_refresh) {
            // If the currently selected clip has been changed, we reset the player
            await this.onClipChanged(this.state.clip);
        }
        await this.videoUpdate();
    }

    componentWillUnmount() {
        Communicator.clear('video-preview-data-update');
    }

    update_end_callbacks: Array<() => void> = []; // to keep track of when a particular update has been completed; since we can only wait for the callback, we can't say "wait until this operation is complete"


    /**
     * Loads in the supplied chunk from the filesystem, and pushes it into the current source buffer.
     */
    async loadChunk(directory: string, segment_id: number) {
        let file = directory + "\\segment" + segment_id.toString().padStart(6, '0') + ".ts";
        console.log(`Loading chunk: ${file}`);
        let contents = await Communicator.readFile(file);
        let buffer = new Uint8Array(contents);


        let res = null;
        try {
            this.source_buffer.abort(); // Previous MPEG-TS files need to be aborted, since they do not ever finish the `Parsing` stage
            await new Promise((resolve, reject) => {
                res = () => { resolve(null) };
                this.update_end_callbacks.push(res); // Wait for the source buffer to be updated
                try {
                    this.source_buffer.timestampOffset = segment_id * VideoPreview.CHUNK_LENGTH;
                    this.source_buffer.appendWindowStart = 0;
                    this.source_buffer.appendBuffer(buffer);
                    // Append the video data to the buffer
                } catch (e) {
                    reject(e);
                    // If anything goes wrong, just abort!
                }
            });

            // If we got to this point, the video data is now successfully in the source buffer.

            this.update_end_callbacks = this.update_end_callbacks.filter(e => e != res); // remove the callback - otherwise it'll cause some weird issues


            let clipData = this.video_preview_data.get(this.state.clip);
            if (typeof clipData != "string") {
                clipData.chunkData[segment_id] = VideoPreviewClipStatus.ChunkStatus.Loaded; // Set the relevant chunk as loaded
            }
            console.log("Loading chunk done!");
        } catch (e) {
            console.log("Error caught in loading chunk!");
            console.log(e);
        }
    }

    /**
     * Requests a set of segments from the Rust client, which will then notify the server to generate
     */
    async requestSegments(clip, start_segment, end_segment) {
        Communicator.invoke('request_video_preview', {
            clipId: clip,
            startChunk: start_segment,
            endChunk: end_segment,
        });
    }


    /**
     * Called when anything is changed about the video preview
     */
    async videoUpdate() {
        if (!this.state.clip) { // if there's no clip, don't do anything
            return;
        }

        const release = await this.change_lock.acquire();

        let clipData = this.video_preview_data.get(this.state.clip);
        if (!clipData || typeof clipData == "string") {
            // If the clip data is invalid, we cannot play any preview for it
            release();
            console.log("Cancelling. Clip data is as follows: ");
            console.log(clipData);
            return;
        }



        let output_directory: string = await new Promise((resolve, reject) => {
            Communicator.invoke("get_output_directory", null, (data) => { // Fetch the output directory from the Rust client
                resolve(data);
            });
        }) + "\\composited-clip-" + this.state.clip;


        let current_segment = Math.floor(this.currentTimestamp / VideoPreview.CHUNK_LENGTH); // Fetch this and the next 10 segments
        let next_segment = current_segment + 10;

        if (clipData.chunkData.length <= next_segment) {
            next_segment = clipData.chunkData.length - 1; // Limit to the maximum chunk number

            if (current_segment > next_segment) {
                release();
                return;
            }
        }


        for (let segment = 0; segment <= next_segment; segment++) {
            if (clipData.chunkData[segment] != VideoPreviewClipStatus.ChunkStatus.Loaded) {
                if (clipData.chunkData[segment] != VideoPreviewClipStatus.ChunkStatus.Downloaded) {
                    let clip = this.state.clip;
                    release();
                    await this.requestSegments(clip, segment, next_segment);
                    return;
                }

                if (!this.source_buffer) {
                    // If there's no source buffer, we can't load anything
                    release();
                    return;
                }
                await this.loadChunk(output_directory, segment);
                release();
                // Only ever load at most one chunk in an update
                return;
            }
        }

        release();
    }





    /**
     * Play/pause the video
     */
    playPause() {

        this.setState({
            playing: !this.state.playing
        });

        let vid = this.video_element_ref.current;
        this.state.playing ? vid.pause() : vid.play();
    }


    last_time_update = -Infinity;
    async timeUpdate(event: React.SyntheticEvent<HTMLVideoElement, Event>) {
        let time = this.currentTimestamp;

        this.forceUpdate();
        if (Math.abs(time - this.last_time_update) > 1) {
            await this.videoUpdate(); // Perform a video update every 1 second, to allow the relevant chunks to be requested
            this.last_time_update = time;
        }
    }


    onSourceBufferUpdateEnd(e) {
        for (let callback of this.update_end_callbacks) {
            callback();
        }

    }

    source_buffer: SourceBuffer;


    async onClipChanged(clip: string) {

        // If the clip is changed, we need to request that clip's length. That way, we can then go ahead and request the relevant chunks, which can then be loaded in
        Communicator.invoke('request_video_length', {
            clipId: clip
        });

        const release = await this.change_lock.acquire();
        let current_data = this.video_preview_data.get(this.state.clip);
        if (typeof current_data != "string" && current_data != null) {
            for (let i = 0; i < current_data.chunkData.length; i++) {
                if (current_data.chunkData[i] == VideoPreviewClipStatus.ChunkStatus.Loaded) {
                    current_data.chunkData[i] = VideoPreviewClipStatus.ChunkStatus.Downloaded;
                }
            }
        }

        if (this.source_buffer) {
            this.source_buffer.removeEventListener('updateend', this.onSourceBufferUpdateEnd);
        }
        await new Promise((resolve, reject) => setTimeout(resolve, 1000));


        // Reset the player - create a new media source, get a new source buffer with the relevant codecs
        this.media_source = new MediaSource();
        this.setState({
            clip,
            videoURL: URL.createObjectURL(this.media_source)
        });


        let res = null;
        await new Promise((resolve, reject) => {
            res = resolve;
            this.media_source.addEventListener('sourceopen', res);
        });
        this.media_source.removeEventListener('sourceopen', res);


        current_data = this.video_preview_data.get(this.state.clip);
        if (!(current_data != null && typeof current_data != "string")) {
            release();
            return;
        }
        let codec = current_data.codec;

        if (!codec || codec.startsWith("ERROR")) {
            release();
            return;
        };
        let mime_type = codec;

        this.source_buffer = this.media_source.addSourceBuffer(mime_type);
        this.source_buffer.mode = 'sequence'; // When we add them, we will manually specify where they should be inserted - instead of them being added based on some metadata in the file itself
        this.source_buffer.addEventListener('updateend', this.onSourceBufferUpdateEnd);
        release();

        await this.videoUpdate();
    }



    render() {
        let options = [
            <option value={null}>Unselected</option>
        ];
        let store = Store.getCurrentStore();
        for (let [id, clip] of store.clips.composited.entries()) {
            options.push(<option value={id}>
                {clip.name}
            </option>)
        }
        let musicDisplay = null;

        let current_data = this.video_preview_data.get(this.state.clip);
        if (current_data && typeof current_data != "string" && !current_data.is_video) {
            musicDisplay = <FontAwesomeIcon icon={faMusic} className={`text-${Utils.Colours.Audio} text-6xl`} />;
        }
        let bufferingDisplay = null;
        if (this.state.buffering) {
            bufferingDisplay = <FontAwesomeIcon icon={faCircleNotch} className={`text-white animate-spin text-6xl`} />;
        }

        let loadedPercentage = 0;
        if (this.state.clip && this.duration != 0) {
            let chunksReady = 1;
            let durationReady = chunksReady * VideoPreview.CHUNK_LENGTH;
            loadedPercentage = Math.min(durationReady / this.duration * 100, 100);
        }

        return <div className="flex flex-col h-full">
            <div className="flex-grow overflow-auto flex justify-center items-center p-3 bg-black relative">
                <video onPlaying={() => this.setState({ buffering: false })} onWaiting={() => this.setState({ buffering: true })} className="max-h-full" ref={this.video_element_ref} src={this.state.videoURL} onTimeUpdate={e => this.timeUpdate(e)}></video>
                <div className='absolute flex items-center justify-center h-full'>
                    {musicDisplay}
                </div>
                <div className='absolute flex items-center justify-center h-full'>
                    {bufferingDisplay}
                </div>
            </div>
            <div className="relative">
                <div className="w-full absolute top-1">
                    <div className="bg-white z-10 h-1" style={{ width: loadedPercentage + "%" }}></div>
                </div>
                <input type="range" min={0} max={100} className="w-full bg-blue-600 h-1 mb-3 relative" onChange={(e) => {

                    let proportion = parseFloat(e.target.value) / 100;

                    let newTime = this.duration * proportion;
                    if (isNaN(newTime)) {
                        newTime = 0;
                    }
                    this.video_element_ref.current.currentTime = newTime;
                }} value={this.currentPercentage} />
                <div className="flex items-center justify-center gap-1">
                    <div className="flex-1">
                        <select className="px-4 py-2 w-full rounded-md text-white font-medium text-lg bg-gray-900 hover:bg-gray-800" value={this.state.clip} onChange={e => this.onClipChanged(e.target.value)}>
                            {options}
                        </select>
                    </div>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepBackward} /></button>
                    <button onClick={() => this.playPause()} className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={this.state.playing ? faPause : faPlay} /></button>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepForward} /></button>
                    <div className="flex-1"></div>
                </div>
            </div>
        </div>;
    }


}

export default VideoPreview;