import { faPause, faPlay, faStepBackward, faStepForward } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { fs } from '@tauri-apps/api';
import React, { ChangeEvent } from 'react';
import { textChangeRangeIsUnchanged } from 'typescript';
import Communicator from '../classes/Communicator';
import Lock from '../classes/Lock';
import Store from '../classes/Store';


interface Props {
    cache?: Map<string, any>;
    // props
}

interface State {
    currentTime: number,
    clip: string,
    videoURL: string,
    playing: boolean
}


enum LoadedStatus {
    Unloaded,
    Loading,
    Loaded
}

class VideoPreview extends React.Component<Props, State> {

    video_element_ref: React.RefObject<HTMLVideoElement>;
    media_source: MediaSource;

    /**
     * Contains an array of `LoadedStatus` which can be used to determine what chunks have been loaded into the `media_source`
     */
    chunk_loading_statuses: Array<LoadedStatus>;
    /**
     * A map from clip IDs to the number of chunks that have been generated for that clip
     */
    clip_chunks_ready: Map<string, number>;


    change_lock: Lock;


    constructor(props: Props) {
        super(props);
        this.media_source = new MediaSource();
        this.video_element_ref = React.createRef();
        this.clip_chunks_ready = new Map();
        this.change_lock = new Lock();

        this.state = {
            currentTime: 0,
            clip: null,
            videoURL: URL.createObjectURL(this.media_source),
            playing: false,
        }
    }

    get currentTimestamp() {
        return this.video_element_ref.current?.currentTime;
    }
    get duration() {
        return this.video_element_ref.current?.duration;
    }

    get currentPercentage(): number {
        return this.currentTimestamp / this.duration * 100;
    }

    componentDidMount() {
        Communicator.on('generated-preview', async (data: { output_directory_path: string, segment_duration: number }) => {

        });
        Communicator.on('video-chunk-ready', async (data) => {
            let node_id: string = data[0];
            let segment_id: string = data[1];
            let clip_id = node_id.substring("composited-clip-file-".length);

            console.log(`Segment ${segment_id} of clip ${clip_id} is now ready - awaiting lock`);
            await this.change_lock.lock();
            this.clip_chunks_ready[clip_id] = segment_id;
            this.change_lock.release();

            console.log(`Segment ${segment_id} of clip ${clip_id} is now ready`);

            await this.videoUpdate();
        });
    }

    update_end_callbacks: Array<() => void> = [];
    async loadChunk(directory: string, segment_id: number) {
        let file = directory + "\\segment" + segment_id.toString().padStart(6, '0') + ".mp4";;
        let contents = await Communicator.readFile(file);
        let buffer = new Uint8Array(contents);

        while (this.chunk_loading_statuses.length <= segment_id) {
            this.chunk_loading_statuses.push(LoadedStatus.Unloaded);
        }
        this.chunk_loading_statuses[segment_id] = LoadedStatus.Loading;
        console.log("Loading chunk + adding callback!");
        let res = null;
        await new Promise((resolve, reject) => {
            res = () => { resolve(null) };
            this.update_end_callbacks.push(res);
            this.source_buffer.appendBuffer(buffer);
        });
        this.update_end_callbacks = this.update_end_callbacks.filter(e => e != res); // remove the callback
        console.log("Loading chunk done!");
        this.chunk_loading_statuses[segment_id] = LoadedStatus.Loaded;
    }


    async videoUpdate() {


        if (!this.state.clip) { // if there's no clip, don't do anything
            return;
        }

        console.log("Aquiring lock for videoUpdate");
        await this.change_lock.lock();
        console.log("Lock aquired for videoUpdate");




        console.log("Getting output directory...");
        let output_directory: string = await new Promise((resolve, reject) => {
            Communicator.invoke("get_output_directory", null, (data) => {
                resolve(data);
            });
        }) + "\\composited-clip-" + this.state.clip;



        let current_segment = Math.floor(this.currentTimestamp / 10); // TODO: SEGMENT LENGTH
        let next_segment = current_segment + 1;

        console.log("Loading up to chunk " + next_segment);
        for (let segment = 0; segment <= next_segment; segment++) {
            if (!this.chunk_loading_statuses[segment] || this.chunk_loading_statuses[segment] != LoadedStatus.Loaded) {
                if (!this.clip_chunks_ready[this.state.clip] || this.clip_chunks_ready[this.state.clip] < segment) {
                    console.log("Lock releasing for videoUpdate");
                    this.change_lock.release();
                    console.log("Lock released for videoUpdate");
                    return;
                }
                // if this chunk has not been generated, stop!                

                await this.loadChunk(output_directory, segment);
                console.log("Lock releasing for videoUpdate");
                this.change_lock.release();
                console.log("Lock released for videoUpdate");
                return;
            }
        }

        console.log("Lock releasing for videoUpdate");
        this.change_lock.release();
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
        if (Math.abs(time - this.last_time_update) > 1) {
            await this.videoUpdate();
        }
        this.last_time_update = time;
    }


    source_buffer: SourceBuffer;
    async onClipChanged(clip: string) {

        console.log("Aquiring lock for onClipChanged");
        await this.change_lock.lock();
        console.log("Lock aquired for onClipChanged");

        this.chunk_loading_statuses = [];


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


        const MIME_TYPE = 'video/mp4; codecs="avc1.4D402A, mp4a.40.2, mp4a.40.2"';
        this.source_buffer = this.media_source.addSourceBuffer(MIME_TYPE);
        console.log("New source buffer:");
        console.log(this.source_buffer);

        this.source_buffer.addEventListener('updateend', (e) => {
            console.log("Update end callback!");
            for (let callback of this.update_end_callbacks) {
                callback();
            }
        })

        // reset everything!

        console.log("Lock releasing for onClipChanged");
        this.change_lock.release();
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

        return <div className="flex flex-col h-full">
            <div className="flex-grow overflow-auto flex justify-center items-center p-3 bg-black">
                <video className="max-h-full" ref={this.video_element_ref} src={this.state.videoURL} onTimeUpdate={e => this.timeUpdate(e)}></video>
            </div>
            <div className="">
                <div className="w-full bg-gray-200 h-1 mb-3 relative" onClick={(e) => {

                    let boundingBox = (e.target as HTMLElement).getBoundingClientRect();
                    let proportion = (e.clientX - boundingBox.left) / boundingBox.width;
                    let newTime = this.video_element_ref.current?.duration * proportion;
                    this.video_element_ref.current.currentTime = newTime;
                }}>
                    <div className="absolute pointer-events-none" style={{ left: this.currentPercentage + "%", transform: "translateX(-50%)" }}>
                        <div className="h-3 w-3 border border-black  rounded-full bg-white transform -translate-y-1/4"></div>
                    </div>
                    <div className="bg-blue-600 h-1 pointer-events-none" style={{ width: this.currentPercentage + "%" }}></div>
                </div>
                <div className="flex items-center justify-center gap-1">
                    <div className="flex-1">
                        <select className="px-4 py-2 w-full rounded-md text-white font-medium text-lg bg-gray-900 hover:bg-gray-800" value={this.state.clip} onChange={e => this.onClipChanged(e.target.value)}>
                            {options}
                        </select>
                    </div>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepBackward} /></button>
                    <button onClick={() => this.playPause()} className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={this.state.playing ? faPause : faPlay} /></button>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepForward} /></button>
                    <button className="flex-1" onClick={() => this.videoUpdate()}>Update!</button>
                </div>
            </div>
        </div>;
    }




    async regenStuff() {
        let e = "C:\\Users\\ajsmi\\AppData\\Roaming\\AdamSmith\\VideoEditor\\output";
        let directory = e + "\\composited-clip-" + this.state.clip;
        let contents = await fs.readDir(directory);

        if (this.state.clip != null && this.state.videoURL == null) {
            let isNew = true;
            let mediaSource = new MediaSource();
            this.setState({
                videoURL: URL.createObjectURL(mediaSource)
            });

            mediaSource.addEventListener('sourceopen', async () => {
                if (!isNew) {
                    return;
                }
                isNew = false;

                const MIME_TYPE = 'video/mp4; codecs="avc1.4D402A, mp4a.40.2, mp4a.40.2"';
                // const MIME_TYPE = 'video/mp4; codecs="avc1.4D402A"';
                let receiver = mediaSource.addSourceBuffer(MIME_TYPE);
                //let receiver = mediaSource.addSourceBuffer('video/mp4; codecs="avc1.4D402A"');

                let i = 5;
                receiver.addEventListener('updateend', (e) => {
                    if (!receiver.updating && mediaSource.readyState === 'open') {
                        // if (i < 0) {
                        //     console.log("Finished stream!");
                        //     mediaSource.endOfStream();
                        // }
                    }
                });
                receiver.addEventListener('error', e => {
                    console.log("ERROR!");
                    console.log(e);
                });

                console.log("Appending files...");

                let doFile = async (f) => {
                    console.log("Doing file: " + f);
                    let contents = await Communicator.readFile(f);
                    let buffer = new Uint8Array(contents);
                    receiver.appendBuffer(buffer);
                    await new Promise((resolve, _) => setTimeout(resolve, 400));
                }

                await doFile(directory + "\\init.mp4");

                const trackTotal = 3;
                let total = (contents.length - 1) / trackTotal;
                for (let i = 1; i <= total; i++) {
                    for (let trackNum = 1; trackNum <= trackTotal; trackNum++) {
                        let file = directory + "\\segment-" + trackNum + "." + i.toString().padStart(6, '0') + ".m4s";
                        await doFile(file);
                    }
                }
                console.log("Finished!");
                // for (let file of contents) {
                //     if (i < 0) {
                //         break;
                //     }
                //     i -= 1;
                //     console.log(file.name);
                //     console.log("Waiting..." + i);
                //     // let file = {
                //     //     path: directory + ".mp4"
                //     // }
                //     await new Promise((resolve, _) => setTimeout(resolve, 400));
                //     console.log("Getting contents...");

                // }
                // console.log("All files appended");
            })
        }
    }

}

export default VideoPreview;