import { PipeableType, PipeableTypeRestriction } from "./NodeRegistration";

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

    static pipeableTypeEqual(t1: PipeableType, t2: PipeableType) {
        if (t1.video != t2.video || t1.audio != t2.audio || t1.subtitles != t2.subtitles) return false;
        return true;
    }

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

    static pipeableTypeMeetsMinReq(t: PipeableType, restriction: PipeableTypeRestriction) {
        return t.video >= restriction.min.video && t.audio >= restriction.min.audio && t.subtitles >= restriction.min.subtitles;
    }
    static pipeableTypeAboveMaxReq(t: PipeableType, restriction: PipeableTypeRestriction) {
        return t.video > restriction.max.video || t.audio > restriction.max.audio || t.subtitles > restriction.max.subtitles;
    }

    static pipeableTypeMatchesRestrictions(t: PipeableType, restriction: PipeableTypeRestriction) {
        if (t.video > restriction.max.video || t.video < restriction.min.video
            || t.audio > restriction.max.audio || t.audio < restriction.min.audio
            || t.subtitles > restriction.max.subtitles || t.video < restriction.min.subtitles)
            return false;
        return true;
    }

    static getColourFromRestriction(t: PipeableTypeRestriction) {
        this.getColour(t.min);
    }
    static getColour(t: PipeableType) {
        if (t.video > 1) return this.Colours.Container;
        if (t.video == 1) return this.Colours.Video;
        if (t.audio > 0) return this.Colours.Audio;
        return this.Colours.Subtitles;
    }
    static Colours = {
        Container: "purple-500", // A container with any number of video, audio or subtitle streams
        Video: "blue-400", // A container with 1 video stream, and any number of audio or subtitle streams
        Audio: "green-400", // A container with any number of audio streams - can be split, or used as one audio stream. By default, if used as one, will simply add the audio sources together, taking the largest audio channel count
        Subtitles: "yellow-400", // A container with any number of subtitle streams
        Unknown: "gray-300" // 

    };
}
