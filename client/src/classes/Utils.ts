import { PipeableType, PipeableTypeRestriction } from "./NodeRegistration";

export default class Utils {

    /**
     * Returns true if any of the supplied arguments are undefined
     */
    static propsUndefined(...props: any) {
        for (let prop of props) {
            if (typeof prop == "undefined") {
                return true;
            }
        }
        return false;
    }

    /**
     * Converts an array buffer to a base 64 string representation; allows images to be specified by passing a string, instead of needing a file
     */
    static bytesToBase64(arrayBuffer) {
        return btoa(
            new Uint8Array(arrayBuffer)
                .reduce((data, byte) => data + String.fromCharCode(byte), '')
        );
    }

    /**
     * Will return true if all the media types match in numbers
     */
    static pipeableTypeEqual(t1: PipeableType, t2: PipeableType) {
        if (t1.video != t2.video || t1.audio != t2.audio || t1.subtitles != t2.subtitles) return false;
        return true;
    }


    /**
     * Returns, if applicable, the downgraded pipeabletype, given the restriction. 
     * For example, if t = {video: 2, audio: 2, subtitles: 1}, and there is a max restriction of {video: 1, audio: 2, subtitles: 0}, then the output will be {video: 1, audio: 2, subtitles: 0}
     */
    static pipeableTypeDowngrade(t: PipeableType, restriction: PipeableTypeRestriction) {
        if (this.pipeableTypeMatchesRestrictions(t, restriction)) {
            return t;
        }
        return {
            video: Math.min(t.video, restriction.max.video),
            audio: Math.min(t.audio, restriction.max.audio),
            subtitles: Math.min(t.subtitles, restriction.max.subtitles),
        } as PipeableType;
    }

    /**
     * Returns true if the supplied type meets the minimum requirements for a particular restriction.
     */

    static pipeableTypeMeetsMinReq(t: PipeableType, restriction: PipeableTypeRestriction) {
        return t.video >= restriction.min.video && t.audio >= restriction.min.audio && t.subtitles >= restriction.min.subtitles;
    }
    /**
     * Indicates whether the supplied type needs to be downgraded
     */
    static pipeableTypeAboveMaxReq(t: PipeableType, restriction: PipeableTypeRestriction) {
        return t.video > restriction.max.video || t.audio > restriction.max.audio || t.subtitles > restriction.max.subtitles;
    }

    /**
     * Returns true if the supplied type is within both the minimum and maximum requirements of the restriction
     */
    static pipeableTypeMatchesRestrictions(t: PipeableType, restriction: PipeableTypeRestriction) {
        if (t.video > restriction.max.video || t.video < restriction.min.video
            || t.audio > restriction.max.audio || t.audio < restriction.min.audio
            || t.subtitles > restriction.max.subtitles || t.video < restriction.min.subtitles)
            return false;
        return true;
    }

    /**
     * Obtains the relevant colour for the media type - will be in the form of a TailwindCSS colour name
     */
    static getColour(t: PipeableType) {
        if (t.video > 1) return this.Colours.Container; // If there's more than one video stream, it's a `container`.
        if (t.video == 1) return this.Colours.Video; // If there's just one video stream, it's a standard video
        if (t.audio > 0) return this.Colours.Audio; // If there's any audio streams, but no video streams, it's simply audio
        return this.Colours.Subtitles; // otherwise, it must be a subtitle stream
    }
    static Colours = {
        Container: "purple-500", // A container with any number of video, audio or subtitle streams
        Video: "blue-400", // A container with 1 video stream, and any number of audio or subtitle streams
        Audio: "green-400", // A container with any number of audio streams - can be split, or used as one audio stream. By default, if used as one, will simply add the audio sources together, taking the largest audio channel count
        Subtitles: "yellow-400", // A container with any number of subtitle streams
        Unknown: "gray-300" // fallback option

    };
}
