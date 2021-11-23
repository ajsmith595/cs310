import { faLayerGroup, faLock, faPhotoVideo } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import React from "react";
import { ClipIdentifier } from "../../classes/Clip";
import Store from "../../classes/Store";

interface Props {
    identifier: ClipIdentifier;
    accepted_clip_type?: 'source' | 'composited' | 'both';
    disable_drag?: boolean;
    onDropClip: (clip_identifier: ClipIdentifier) => void;
}
interface State {

}


export default class ClipDropComponent extends React.Component<Props, State> {
    constructor(props) {
        super(props);

        this.onDropClip = this.onDropClip.bind(this);
        this.onDragOver = this.onDragOver.bind(this);

    }

    getClipIdentifier(event: React.DragEvent<HTMLDivElement>) {
        if (this.props.disable_drag)
            return false;
        let data = JSON.parse(event.dataTransfer.getData('application/json'));
        let clip_identifier: ClipIdentifier;
        try {
            clip_identifier = ClipIdentifier.deserialise(data);
        } catch (e) {
            return false; // it's not a clip identifier
        }
        if (this.props.accepted_clip_type !== 'both' && this.props.accepted_clip_type && clip_identifier.clip_type.toLowerCase() !== this.props.accepted_clip_type)
            return false; // if it's not the type we're looking for, ignore it
        return clip_identifier;
    }

    onDropClip(event: React.DragEvent<HTMLDivElement>) {
        let clip_identifier = this.getClipIdentifier(event);
        if (clip_identifier === false)
            return true;

        event.preventDefault();
        event.stopPropagation();

        this.props.onDropClip(clip_identifier);
    }
    onDragOver(event: React.DragEvent<HTMLDivElement>) {

        // let clip_identifier = this.getClipIdentifier(event);
        // if (clip_identifier === false)
        //     return true;

        // if we've got past all the checks, make the drop event happen
        event.preventDefault();
        event.stopPropagation();
    }

    render() {
        let name = "Not selected";
        let icon = null;

        if (this.props.identifier) {
            let clip = Store.getCurrentStore().clips[this.props.identifier.clip_type.toLowerCase()].get(this.props.identifier.id);
            if (clip) {
                name = clip.name;
                icon = <FontAwesomeIcon className="mr-2" icon={this.props.identifier.clip_type.toLowerCase() === 'source' ? faPhotoVideo : faLayerGroup} />;
            }
        }

        let locked_icon = null;
        if (this.props.disable_drag) {
            locked_icon = <FontAwesomeIcon className="text-xl float-right mr-1 text-gray-600" icon={faLock} />
        }

        return (
            <div className="p-2 bg-gray-400 text-black" onDrop={this.onDropClip} onDragOver={this.onDragOver}>
                <p>{icon} <span className="mr-3">{name}</span> {locked_icon}</p>
            </div>
        );
    }
}