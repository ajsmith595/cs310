import { faClosedCaptioning } from '@fortawesome/free-regular-svg-icons';
import { faCog, faFile, faMusic, faVideo, faVideoSlash } from '@fortawesome/free-solid-svg-icons';
import * as fontAwesomeSolid from '@fortawesome/free-solid-svg-icons';
import * as fontAwesomeRegular from '@fortawesome/free-regular-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { ClipIdentifier, CompositedClip, SourceClip } from '../classes/Clip';
import Communicator from '../classes/Communicator';
import EventBus from '../classes/EventBus';
import EditorNode from '../classes/Node';
import Utils from '../classes/Utils';
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

        if (clip.info) {
            return clip.info;
        }
        return null;
    }

    renderEditorNode(selection: EditorNode) {

        let registration = EditorNode.NodeRegister.get(selection.node_type);
        let props = [];

        for (let [prop, prop_detail] of selection.inputs.entries()) {
            let is_piped = prop_detail.property_type.type == 'Pipeable';
            if (is_piped)
                continue;

            let value = selection.properties.get(prop);
            let display = null;

            if (prop_detail.property_type.type === 'Clip') {
                let clip_identifier = ClipIdentifier.deserialise(value);
                display = <ClipDropComponent identifier={clip_identifier} onDropClip={(clip_identifier) => selection.changeProperty(prop, clip_identifier)} disable_drag={selection.node_type === 'output'} />;
            }
            else if (prop_detail.property_type.type === 'Number') {
                let details = prop_detail.property_type.getNumberRestrictions();
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
        return (<>
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
        </>);
    }

    renderSourceClip(selection: SourceClip) {
        let clip_info = this.getSourceClipInfo();

        let media_info_display = <p className="text-center">Loading media information <FontAwesomeIcon icon={faCog} className="fa-spin" /></p>;

        let segment = (key: JSX.Element, value: JSX.Element, class_name = '') => {
            return (<div className={`border p-1 border-gray-600 ${class_name}`}>
                <p className="flex">
                    <span>{key}</span>
                    <span className="text-right flex-1">{value}</span>
                </p>
            </div>);
        }

        let letters = ['', 'k', 'M', 'G', 'T', 'X'];
        let track = 0;

        let get_type_indicator = (stream_type: 'audio' | 'video' | 'subtitle') => {
            let colour = Utils.Colours.Unknown;
            let icon = faFile;
            switch (stream_type) {
                case "audio":
                    colour = Utils.Colours.Audio;
                    icon = faMusic;
                    break;
                case "video":
                    colour = Utils.Colours.Video;
                    icon = faVideo;
                    break;
                case "subtitle":
                    colour = Utils.Colours.Subtitles;
                    icon = faClosedCaptioning;
                    break;
            }

            return <FontAwesomeIcon icon={icon} className={`text-${colour} mr-2`} />;
        };


        if (clip_info) {
            let display = [];
            for (let video_stream of clip_info.video_streams) {

                let bitrate = video_stream.bitrate;
                let i = 0;
                while (bitrate > 1024) {
                    bitrate /= 1024;
                    i++;
                }
                let bitrate_string = bitrate.toPrecision(4);

                let fps = video_stream.framerate.toPrecision(4);


                display.push(
                    <div className="mt-2 mb-5">
                        <div>{get_type_indicator('video')}Track {track + 1}</div>
                        <div className="ml-2 text-xs">
                            {segment(
                                (<>Dimensions </>),
                                <>{video_stream.width} x {video_stream.height}</>
                            )}
                            {segment(
                                (<>Framerate</>),
                                <>{fps}</>
                            )}
                            {segment(
                                (<>Bitrate <span className="text-gray-500">({letters[i]}bps)</span></>),
                                <>{bitrate_string}</>
                            )}
                        </div>
                    </div>
                );

                track++;
            }

            for (let audio_stream of clip_info.audio_streams) {
                let bitrate = audio_stream.bitrate;
                let i = 0;
                while (bitrate > 1024) {
                    bitrate /= 1024;
                    i++;
                }
                let bitrate_string = bitrate.toPrecision(4);
                let sample_rate = audio_stream.sample_rate;
                let j = 0;
                while (sample_rate >= 1000) {
                    sample_rate /= 1000;
                    j++;
                }

                display.push(
                    <div className="mt-2 mb-5">
                        <div>{get_type_indicator('audio')}Track {track + 1}</div>
                        <div className="ml-2 text-xs">
                            {segment(
                                (<>Sample Rate <span className="text-gray-500">({letters[j]}Hz)</span></>),
                                <>{sample_rate}</>
                            )}
                            {segment(
                                (<>Bitrate <span className="text-gray-500">({letters[i]}bps)</span></>),
                                <>{bitrate_string}</>
                            )}
                            {segment(
                                (<>Number of Channels</>),
                                <>{audio_stream.number_of_channels}</>
                            )}
                            {segment(
                                (<>Language</>),
                                <>{audio_stream.language}</>
                            )}
                        </div>
                    </div>
                );


                track++;
            }

            let duration = clip_info.duration / 1000;
            let hrs = Math.floor(duration / 3600);
            let mins = Math.floor((duration % 3600) / 60);
            let seconds = Math.round(duration % 60);

            let duration_str = mins.toString().padStart(2, '0') + ":" + seconds.toString().padStart(2, '0');
            if (hrs > 0) {
                duration_str = hrs.toString().padStart(2, '0') + ":" + duration_str;
            }


            media_info_display = <div>
                <div className="border border-gray-500 mt-2">
                    <h1 className='small-caps text-xl px-2 font-bold'>track information</h1>
                    <hr className="border-gray-500" />
                    <div className="p-1">
                        {segment(<>Duration</>, <>{duration_str}</>, 'text-xs')}
                        {display}
                    </div>
                </div>
            </div>;
        }

        let thumbnail_data = this.props.cache.get("clips").source[selection.id].thumbnail_data;
        let img = <img src="https://via.placeholder.com/1920x1080" className="w-full" />;
        if (thumbnail_data) {
            img = <img src={"data:image/jpeg;base64," + thumbnail_data} className="w-full" />;
        }
        return (<>
            <div className="mb-4">
                {img}
                <h1 className="text-lg">{selection.name}</h1>
                <p className="text-xs">Location: {selection.file_location}</p>
            </div>
            <div className="">
                {segment}
                <div className="border p-3 border-gray-600 mt-2">
                    <p className="flex">
                        <span>ID</span>
                        <span className="text-right flex-1">{selection.id}</span>
                    </p>
                </div>
            </div>
            {media_info_display}
        </>);
    }

    renderCompositedClip(selection: CompositedClip) {
        return (<>
            <p>Name: {selection.name}</p>
        </>);
    }

    render() {


        let selection = EventBus.getValue(EventBus.GETTERS.APP.CURRENT_SELECTION);


        let content = <h1>Cannot provide information on the current selection</h1>;
        if (selection instanceof EditorNode) {
            content = this.renderEditorNode(selection);
        }
        else if (selection instanceof SourceClip) {
            content = this.renderSourceClip(selection);
        }
        else if (selection instanceof CompositedClip) {
            content = this.renderCompositedClip(selection);
        }
        return <div className="text-white overflow-y-auto max-h-full">
            {content}
        </div>;

    }
}

export default PropertiesPanel;