import { faPause, faPlay, faStepBackward, faStepForward } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { fs } from '@tauri-apps/api';
import React from 'react';
import Communicator from '../classes/Communicator';
import Store from '../classes/Store';


interface Props {
    cache?: Map<string, any>;
    // props
}

interface State {
    currentTime: number,
    clip: string,
    videoURL: string,
}


enum LoadedStatus {
    Unloaded,
    Loading,
    Loaded
}

class VideoPreview extends React.Component<Props, State> {

    videoElementRef: React.RefObject<HTMLVideoElement>;
    chunks: Array<LoadedStatus>;
    chunkLength: number;
    mediaSource: MediaSource;
    ready: boolean;


    constructor(props: Props) {
        super(props);
        this.state = {
            currentTime: 0,
            clip: null,
            videoURL: null,
        }

        this.videoElementRef = React.createRef();
    }

    get currentTimestamp() {
        return this.videoElementRef.current?.currentTime;
    }
    get duration() {
        return this.videoElementRef.current?.duration;
    }

    get currentPercentage(): number {
        return this.currentTimestamp / this.duration * 100;
    }

    componentDidMount() {
        Communicator.on('generated-preview', async (data: { output_directory_path: string, segment_duration: number }) => {
            this.chunkLength = data.segment_duration;
            await this.loadInitialData(data);
        });
    }



    fileQueue: Array<string>;
    async addFileToBuffer(file: string) {
        console.log("Adding file to buffer: " + file);
        if (!this.ready) {
            this.fileQueue.push(file);
            console.log("Delaying " + file);
            return;
        }
        this.ready = false;
        try {
            let contents = await Communicator.readFile(file);
            let buffer = new Uint8Array(contents);
            this.mediaSource.sourceBuffers[0].appendBuffer(buffer);
        } catch (e) {
            console.log(`Error (${file}):`);
            console.log(e);
            this.ready = true;
        }
    }
    async bufferUpdated() {
        // this.mediaSource.endOfStream(); // TODO: change this to signal when the last chunk has been loaded
        this.ready = true;
        if (this.fileQueue.length > 0) {
            let file = this.fileQueue.shift();
            this.addFileToBuffer(file);
        }
    }

    directory: string;
    async loadInitialData({ output_directory_path, segment_duration }) {
        // this.directory = output_directory_path + "\\composited-clip-" + this.state.clip;
        this.directory = "file:///D:\\Videos\\OBS Recordings\\out";

        let numberOfFiles = (await fs.readDir(this.directory)).length;
        console.log("Number of files: " + numberOfFiles);
        this.mediaSource = new MediaSource();
        this.setState({
            videoURL: URL.createObjectURL(this.mediaSource)
        });

        await new Promise((resolve, reject) => {
            let callback = async () => {
                if (this.mediaSource.sourceBuffers.length == 0) {
                    const MIME_TYPE = 'video/mp4; codecs="avc1.4D402A, mp4a.40.2, mp4a.40.2"';

                    let receiver = this.mediaSource.addSourceBuffer(MIME_TYPE);
                    this.ready = true;
                    this.fileQueue = [];
                    this.chunks = [];

                    receiver.addEventListener('updateend', (e) => this.bufferUpdated());
                    receiver.addEventListener('error', e => {
                        console.log("ERROR!");
                        console.log(e);
                    });

                    await this.addFileToBuffer(this.directory + "\\init.mp4");
                    this.mediaSource.removeEventListener('sourceopen', callback);
                    resolve(null);
                }
                else {
                    this.mediaSource.removeEventListener('sourceopen', callback);
                    reject("Media Source has too many source buffers!");
                }
            };

            this.mediaSource.addEventListener('sourceopen', callback);
        });
    }


    playPause() {
        let vid = this.videoElementRef.current;
        vid.paused ? vid.play() : vid.pause();
    }

    timeUpdate(event: React.SyntheticEvent<HTMLVideoElement, Event>) {

        const trackTotal = 3;
        let currentTime = this.currentTimestamp;


        let currentSegmentIndex = Math.floor(currentTime / this.chunkLength);
        let nextSegmentIndex = currentSegmentIndex + 1;
        while (this.chunks.length - 1 < nextSegmentIndex) {
            // create extra entries until true
            this.chunks.push(LoadedStatus.Unloaded);
        }

        for (let i of [currentSegmentIndex, nextSegmentIndex]) {
            if (this.chunks[i] == LoadedStatus.Unloaded) {
                this.chunks[i] = LoadedStatus.Loading;
                // load segment i

                for (let trackNum = 1; trackNum <= trackTotal; trackNum++) {
                    let fileIndex = i + 1;
                    let file = this.directory + "\\segment-" + trackNum + "." + fileIndex.toString().padStart(6, '0') + ".m4s";
                    this.addFileToBuffer(file);
                }
            }
        }


        this.setState({
            currentTime: currentTime
        })
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
                <video className="max-h-full" ref={this.videoElementRef} src={this.state.videoURL} onTimeUpdate={e => this.timeUpdate(e)}></video>
            </div>
            <div className="">
                <div className="w-full bg-gray-200 h-1 mb-3 relative" onClick={(e) => {

                    let boundingBox = (e.target as HTMLElement).getBoundingClientRect();
                    let proportion = (e.clientX - boundingBox.left) / boundingBox.width;
                    let newTime = this.videoElementRef.current?.duration * proportion;
                    this.videoElementRef.current.currentTime = newTime;
                }}>
                    <div className="absolute pointer-events-none" style={{ left: this.currentPercentage + "%", transform: "translateX(-50%)" }}>
                        <div className="h-3 w-3 border border-black  rounded-full bg-white transform -translate-y-1/4"></div>
                    </div>
                    <div className="bg-blue-600 h-1 pointer-events-none" style={{ width: this.currentPercentage + "%" }}></div>
                </div>
                <div className="flex items-center justify-center gap-1">
                    <div className="flex-1">
                        <select className="px-4 py-2 w-full rounded-md text-white font-medium text-lg bg-gray-900 hover:bg-gray-800" value={this.state.clip} onChange={e => {
                            this.setState({
                                clip: e.target.value
                            });
                            console.log(e);
                        }
                        }>
                            {options}
                        </select>
                    </div>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepBackward} /></button>
                    <button onClick={() => this.playPause()} className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={this.videoElementRef.current?.paused ? faPlay : faPause} /></button>
                    <button className="px-4 py-2 text-lg font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75"><FontAwesomeIcon icon={faStepForward} /></button>
                    <button className="flex-1" onClick={() => this.regenStuff()}>Regenerate!</button>
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