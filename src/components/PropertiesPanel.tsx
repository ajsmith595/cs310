import React from 'react';
import { ClipIdentifier, CompositedClip, SourceClip } from '../classes/Clip';
import Communicator from '../classes/Communicator';
import EventBus from '../classes/EventBus';
import EditorNode from '../classes/Node';
import ClipDropComponent from './shared/ClipDropComponent';


interface Props {
    cache?: Map<string, any>;
}

interface State {
    // state
}

class PropertiesPanel extends React.Component<Props, State> {
    constructor(props: Props) {
        super(props);
    }

    componentDidMount() {
        // EventBus.on(EventBus.EVENTS.APP.SET_STORE, () => this.forceUpdate());
        // EventBus.on(EventBus.EVENTS.APP.SET_STORE_UI, () => this.forceUpdate());
    }
    componentWillUnmount() {
    }

    getSourceClipInfo() {
        let clip: SourceClip = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION);

        let cache_id = "clip_info_" + clip.id;
        if (this.props.cache.has(cache_id)) {
            return this.props.cache.get(cache_id);
        }

        Communicator.invoke('get_file_info', {
            clipId: clip.id
        }, (data) => {
            this.props.cache.set(cache_id, data);
            console.log(data);
            this.forceUpdate();
        });
        return null;
    }

    render() {


        let selection = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION);


        let content = <h1>Cannot provide information on the current selection</h1>;
        if (selection instanceof EditorNode) {

            let registration = EditorNode.NodeRegister.get(selection.node_type);
            let props = [];

            for (let [prop, prop_detail] of registration.properties.entries()) {
                let is_piped = false;
                for (let type of prop_detail.property_type) {
                    if (type.type === 'Pipeable') {
                        is_piped = true;
                        break;
                    }
                }
                if (is_piped) {
                    continue;
                }

                let value = selection.properties.get(prop);
                let display = null;

                if (prop_detail.property_type[0].type === 'Clip') {
                    let clip_identifier = ClipIdentifier.deserialise(value);
                    display = <ClipDropComponent identifier={clip_identifier} onDropClip={(clip_identifier) => selection.changeProperty(prop, clip_identifier)} disable_drag={selection.node_type === 'output'} />;
                }
                else if (prop_detail.property_type[0].type === 'Number') {
                    let details = prop_detail.property_type[0].getNumberRestrictions();
                    value = Math.round(value / details.step) * details.step;
                    display = (
                        <div>
                            <input key={Date.now()} className="bg-gray-600 p-2 w-full outline-none" defaultValue={value} type="number" step={details.step} min={details.min} max={details.max} onBlur={(e) => selection.changeProperty(prop, e.target.value)} onKeyDown={(e) => {
                                if (e.key === 'Enter') {
                                    e.preventDefault();
                                    let target: HTMLInputElement & EventTarget = (e.target as any);
                                    selection.changeProperty(prop, target.value);
                                }
                            }} />
                        </div>
                    );
                }
                else {
                    if (typeof value === 'object') {
                        value = JSON.stringify(value);
                    }
                    if (!value) {
                        value = 'Not set';
                    }
                    display = <p>{value}</p>;
                }
                props.push(
                    <div className="border p-3 border-gray-600 mt-2">
                        <p>{prop_detail.display_name}</p>
                        {display}
                        <p className="text-xs">{prop_detail.description}</p>
                    </div>
                )
            }
            content = <>
                <div className="mb-4">
                    <h1 className="text-lg">{registration.display_name}</h1>
                    <p className="text-xs">{registration.description}</p>
                </div>
                <div className="">
                    <div className="border p-3 border-gray-600 mt-2">
                        <p className="flex">
                            <span>ID</span>
                            <span className="text-right flex-1">{selection.id}</span>
                        </p>
                    </div>
                    {props}
                </div>
            </>
        }
        else if (selection instanceof SourceClip) {
            let video_info = this.getSourceClipInfo();

            let video_info_display = <p className="text-center">Loading...</p>;
            if (video_info) {
                let video_stream_display = [];
                let audio_stream_display = [];

                for (let video_stream of video_info.video_streams) {

                    let bitrate = video_stream.bitrate;
                    let letters = ['', 'k', 'M', 'G', 'T', 'X'];
                    let i = 0;
                    while (bitrate > 1024) {
                        bitrate /= 1024;
                        i++;
                    }
                    bitrate = bitrate.toPrecision(4);

                    let fps = video_stream.fps.toPrecision(4);

                    video_stream_display.push(
                        <div className="mt-2 ">
                            <div className="border p-3 border-gray-600">
                                <p className="flex">
                                    <span>Dimensions</span>
                                    <span className="text-right flex-1">{video_stream.width} x {video_stream.height}</span>
                                </p>
                            </div>
                            <div className="border p-3 border-gray-600">
                                <p className="flex">
                                    <span>Framerate</span>
                                    <span className="text-right flex-1">{fps}</span>
                                </p>
                            </div>
                            <div className="border p-3  border-gray-600">
                                <p className="flex">
                                    <span>Bitrate <span className="text-gray-500">({letters[i]}bps)</span></span>
                                    <span className="text-right flex-1">{bitrate}</span>
                                </p>
                            </div>
                        </div>
                    );
                }

                for (let audio_stream of video_info.audio_streams) {
                    let bitrate = audio_stream.bitrate;
                    let letters = ['', 'k', 'M', 'G', 'T', 'X'];
                    let i = 0;
                    while (bitrate > 1024) {
                        bitrate /= 1024;
                        i++;
                    }
                    bitrate = bitrate.toPrecision(4);
                    let sample_rate = audio_stream.sample_rate;
                    let j = 0;
                    while (sample_rate >= 1000) {
                        sample_rate /= 1000;
                        j++;
                    }



                    audio_stream_display.push(
                        <div className="mt-2 ">
                            <div className="border p-3 border-gray-600">
                                <p className="flex">
                                    <span>Sample Rate <span className="text-gray-500">({letters[j]}Hz)</span></span>
                                    <span className="text-right flex-1">{sample_rate}</span>
                                </p>
                            </div>
                            <div className="border p-3  border-gray-600">
                                <p className="flex">
                                    <span>Bitrate <span className="text-gray-500">({letters[i]}bps)</span></span>
                                    <span className="text-right flex-1">{bitrate}</span>
                                </p>
                            </div>
                            <div className="border p-3 border-gray-600">
                                <p className="flex">
                                    <span>Number of Channels</span>
                                    <span className="text-right flex-1">{audio_stream.num_channels}</span>
                                </p>
                            </div>
                            <div className="border p-3 border-gray-600">
                                <p className="flex">
                                    <span>Language</span>
                                    <span className="text-right flex-1">{audio_stream.language}</span>
                                </p>
                            </div>
                        </div>
                    );
                }
                if (!audio_stream_display.length) {
                    audio_stream_display = [(
                        <p>No audio streams</p>
                    )];
                }

                let duration = video_info.duration / 1000;
                let hrs = Math.floor(duration / 3600);
                let mins = Math.floor((duration % 3600) / 60);
                let seconds = Math.round(duration % 60);

                let duration_str = mins.toString().padStart(2, '0') + ":" + seconds.toString().padStart(2, '0');
                if (hrs > 0) {
                    duration_str = hrs.toString().padStart(2, '0') + ":" + duration_str;
                }


                video_info_display = <div>
                    <div className="border p-3 border-gray-600 mt-2">
                        <p className="flex">
                            <span>Duration</span>
                            <span className="text-right flex-1">{duration_str}</span>
                        </p>
                    </div>
                    <div>
                        <h1>Video Streams</h1>
                        {video_stream_display}
                        <h1>Audio Streams</h1>
                        {audio_stream_display}
                    </div>
                </div>;
            }

            let thumbnail_data = this.props.cache.get("clips").source[selection.id].thumbnail_data;
            let img = <img src="https://via.placeholder.com/1920x1080" className="w-full" />;
            if (thumbnail_data) {
                img = <img src={"data:image/jpeg;base64," + thumbnail_data} className="w-full" />;
            }
            content = (<>
                <div className="mb-4">
                    {img}
                    <h1 className="text-lg">{selection.name}</h1>
                    <p className="text-xs">Location: {selection.file_location}</p>
                </div>
                <div className="">
                    <div className="border p-3 border-gray-600 mt-2">
                        <p className="flex">
                            <span>ID</span>
                            <span className="text-right flex-1">{selection.id}</span>
                        </p>
                    </div>
                </div>
                {video_info_display}
            </>);
        }
        else if (selection instanceof CompositedClip) {
            content = <>
                <p>Name: {selection.name}</p>
            </>;
        }
        return <div className="text-white overflow-y-auto max-h-full">
            {content}
        </div>;

    }
}

export default PropertiesPanel;