import { faCircleNotch, faMusic, faPause, faPlay, faStepBackward, faStepForward } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { fs } from '@tauri-apps/api';
import React, { ChangeEvent } from 'react';
import { textChangeRangeIsUnchanged } from 'typescript';
import Communicator from '../classes/Communicator';
import Store from '../classes/Store';
import { Mutex } from 'async-mutex';
import Utils from '../classes/Utils';
import EventBus from '../classes/EventBus';


interface Props {
    cache?: Map<string, any>;
    // props
}

interface State {
    currentTime: number,
    clip: string,
    videoURL: string,
    playing: boolean,
    buffering: boolean,
}

interface ClipInfo {
    codec: string;
    no_video_streams: number;
    no_audio_streams: number;
}
namespace VideoPreviewInputData {
    type VideoPreviewClipStatus = "NotRequested" | "LengthRequested" | {
        "Data": [
            number,
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
        chunkData: Array<ChunkStatus>;
    }

}

type VideoPreviewClipStatus = "NotRequested" | "LengthRequested" | VideoPreviewClipStatus.Data;

class VideoPreview extends React.Component<Props, State> {

    video_element_ref: React.RefObject<HTMLVideoElement>;
    media_source: MediaSource;

    /**
     * Contains an array of `LoadedStatus` which can be used to determine what chunks have been loaded into the `media_source`
     */
    video_preview_data: Map<string, VideoPreviewClipStatus>;





    change_lock: Mutex;


    constructor(props: Props) {
        super(props);
        this.media_source = new MediaSource();
        this.video_element_ref = React.createRef();
        this.change_lock = new Mutex();
        this.video_preview_data = new Map();

        this.state = {
            currentTime: 0,
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
            await this.videoPreviewDataUpdate(data);
        });

        Communicator.invoke('get_video_preview_data', {}, async (data) => {
            await this.videoPreviewDataUpdate(data);
        });
    }

    async videoPreviewDataUpdate(data: VideoPreviewInputData.VideoPreviewData) {

        console.log("New data received");
        console.log(data);
        let release = await this.change_lock.acquire();
        for (let k of this.video_preview_data.keys()) {
            if (data[k] === undefined) {
                this.video_preview_data.delete(k);
            }
        }

        for (let k in data) {
            let value = data[k];
            if (typeof value == "string") {
                this.video_preview_data.set(k, value);
            }
            else {
                let duration = value.Data[0];
                EventBus.dispatch('composited-clip-length', [k, duration]);
                let data = value.Data[1];

                let existing = this.video_preview_data.get(k);
                if (existing == null || typeof existing == "string" || existing.chunkData.length != data.length) {
                    this.video_preview_data.set(k, {
                        duration,
                        chunkData: (data as Array<any>)
                    })
                }
                else {
                    for (let i = 0; i < existing.chunkData.length; i++) {
                        let existingChunkData = existing.chunkData[i];
                        let newChunkData = data[i];

                        if (!(existingChunkData == VideoPreviewClipStatus.ChunkStatus.Loaded && newChunkData == VideoPreviewInputData.VideoPreviewChunkStatus.Downloaded)) {
                            existing.chunkData[i] = newChunkData as any;
                        }
                    }
                }
            }
        }
        release();
        await this.videoUpdate();
    }

    componentWillUnmount() {
        Communicator.clear('video-preview-data-update');
    }


    async changeClipCodec() {
        await this.onClipChanged(this.state.clip);
    }

    update_end_callbacks: Array<() => void> = [];
    async loadChunk(directory: string, segment_id: number) {
        let file = directory + "\\segment" + segment_id.toString().padStart(6, '0') + ".mp4";;
        console.log(`Loading chunk: ${file}`);
        let contents = await Communicator.readFile(file);
        let buffer = new Uint8Array(contents);

        let clipData = this.video_preview_data.get(this.state.clip);
        if (typeof clipData != "string") {
            clipData.chunkData[segment_id] = VideoPreviewClipStatus.ChunkStatus.Loaded;
        }
        let res = null;
        await new Promise((resolve, reject) => {
            res = () => { resolve(null) };
            this.update_end_callbacks.push(res);
            this.source_buffer.timestampOffset = segment_id * 10;
            this.source_buffer.appendWindowStart = 0;
            this.source_buffer.appendBuffer(buffer);
        });
        this.update_end_callbacks = this.update_end_callbacks.filter(e => e != res); // remove the callback
        console.log("Loading chunk done!");
    }

    async requestSegments(clip, start_segment, end_segment) {
        console.log("Requesting chunks: " + start_segment + " => " + end_segment);
        Communicator.invoke('request_video_preview', {
            clipId: clip,
            startChunk: start_segment,
            endChunk: end_segment,
        });
    }


    async videoUpdate() {


        if (!this.state.clip) { // if there's no clip, don't do anything
            return;
        }

        console.log("Aquiring lock for videoUpdate");
        const release = await this.change_lock.acquire();
        console.log("Lock aquired for videoUpdate");


        let clipData = this.video_preview_data.get(this.state.clip);
        if (!clipData || typeof clipData == "string") {
            release();
            console.log("Cancelling. Clip data is as follows: ");
            console.log(clipData);
            return;
        }



        console.log("Getting output directory...");
        let output_directory: string = await new Promise((resolve, reject) => {
            Communicator.invoke("get_output_directory", null, (data) => {
                resolve(data);
            });
        }) + "\\composited-clip-" + this.state.clip;



        let current_segment = Math.floor(this.currentTimestamp / 10); // TODO: SEGMENT LENGTH
        let next_segment = current_segment + 1;

        if (clipData.chunkData.length <= next_segment) {
            next_segment = clipData.chunkData.length - 1;

            if (current_segment > next_segment) {
                return;
            }
        }

        console.log("Loading up to chunk " + next_segment);

        for (let segment = 0; segment <= next_segment; segment++) {
            if (clipData.chunkData[segment] != VideoPreviewClipStatus.ChunkStatus.Loaded) {
                if (clipData.chunkData[segment] != VideoPreviewClipStatus.ChunkStatus.Downloaded) {
                    console.log("Lock releasing for videoUpdate");
                    let clip = this.state.clip;
                    release();
                    await this.requestSegments(clip, segment, next_segment);
                    console.log("Lock released for videoUpdate");
                    return;
                }
                // if this chunk has not been generated, stop!                

                await this.loadChunk(output_directory, segment);
                console.log("Lock releasing for videoUpdate");
                release();
                console.log("Lock released for videoUpdate");
                return;
            }
        }

        console.log("Lock releasing for videoUpdate");
        release();
        console.log("Lock released for videoUpdate");
    }





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
            await this.videoUpdate();


            this.last_time_update = time;
        }
    }


    onSourceBufferUpdateEnd() {
        console.log("Update end callback!");
        for (let callback of this.update_end_callbacks) {
            callback();
        }

    }

    source_buffer: SourceBuffer;
    async onClipChanged(clip: string) {




        Communicator.invoke('request_video_length', {
            clipId: clip
        });

        console.log("Aquiring lock for onClipChanged");
        const release = await this.change_lock.acquire();
        let current_data = this.video_preview_data.get(this.state.clip);
        if (typeof current_data != "string" && current_data != null) {
            for (let i = 0; i < current_data.chunkData.length; i++) {
                if (current_data.chunkData[i] == VideoPreviewClipStatus.ChunkStatus.Loaded) {
                    current_data.chunkData[i] = VideoPreviewClipStatus.ChunkStatus.Downloaded;
                }
            }
        }
        console.log("Lock acquired for onClipChanged");

        if (this.source_buffer) {
            this.source_buffer.removeEventListener('updateend', this.onSourceBufferUpdateEnd);
        }
        await new Promise((resolve, reject) => setTimeout(resolve, 1000));

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



        let mime_type = 'video/mp4; codecs="avc1.64001F, mp4a.40.2"';
        // if (this.clip_codecs.get(clip)) {
        //     mime_type = this.clip_codecs.get(clip).codec;
        // }
        // else {
        //     release();
        //     console.log("WARNING: no codec found!");
        //     console.log(this.clip_codecs);
        //     console.log(clip);
        //     return;
        // }




        this.source_buffer = this.media_source.addSourceBuffer(mime_type);
        this.source_buffer.mode = 'sequence';
        console.log("New source buffer:");
        console.log(this.source_buffer);

        this.source_buffer.addEventListener('updateend', this.onSourceBufferUpdateEnd);

        // reset everything!

        console.log("Lock releasing for onClipChanged");
        release();
        console.log("Lock released for onClipChanged");

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
        // if (this.clip_codecs.get(this.state.clip) && this.clip_codecs.get(this.state.clip).no_video_streams == 0) {
        //     musicDisplay = <FontAwesomeIcon icon={faMusic} className={`text-${Utils.Colours.Audio} text-6xl`} />;
        // }
        let bufferingDisplay = null;
        if (this.state.buffering) {
            bufferingDisplay = <FontAwesomeIcon icon={faCircleNotch} className={`text-white animate-spin text-6xl`} />;
        }

        let loadedPercentage = 0;
        if (this.state.clip && this.duration != 0) {
            // let chunksReady = this.clip_chunks_ready.get(this.state.clip);
            let chunksReady = 1;
            let durationReady = chunksReady * 10;
            loadedPercentage = durationReady / this.duration * 100;
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
                    <button className="flex-1" onClick={() => {
                        console.log(this.video_preview_data);
                    }}>Debug</button>
                </div>
            </div>
        </div>;
    }


}

export default VideoPreview;