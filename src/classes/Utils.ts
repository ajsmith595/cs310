import { PipeableType } from "./NodeRegistration";

export default class Utils {
    static propsUndefined(...props: any) {
        for (let prop of props) {
            if (typeof prop == "undefined") {
                return true;
            }
        }
        return false;
    }

    static bytesToBase64(arrayBuffer) {
        return btoa(
            new Uint8Array(arrayBuffer)
                .reduce((data, byte) => data + String.fromCharCode(byte), '')
        );
    }


    static getColour(t: PipeableType) {
        if (t == null) return this.Colours.Container;
        if (t == PipeableType.Video) return this.Colours.Video;
        if (t == PipeableType.Audio) return this.Colours.Audio;
        if (t == PipeableType.Subtitle) return this.Colours.Subtitles;
    }
    static Colours = {
        Container: "purple-500", // A container with any number of video, audio or subtitle streams
        Video: "blue-400", // A container with 1 video stream, and any number of audio or subtitle streams
        Audio: "green-400", // A container with any number of audio streams - can be split, or used as one audio stream. By default, if used as one, will simply add the audio sources together, taking the largest audio channel count
        Subtitles: "yellow-400", // A container with any number of subtitle streams
        Unknown: "gray-300" // 

    };
}
