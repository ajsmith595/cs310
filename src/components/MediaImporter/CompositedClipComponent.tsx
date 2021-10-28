import { faFileImport } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import Communicator from '../../classes/Communicator';
import Store from '../../classes/Store';
import StoreContext from '../../contexts/StoreContext';
import { CompositedClip } from '../../classes/Clip';
import { faEdit } from '@fortawesome/free-regular-svg-icons';



interface Props {
    // props
    clip: CompositedClip
}

interface State {
    editing: boolean
}

class CompositedClipComponent extends React.Component<Props, State> {
    private inputRef = React.createRef<HTMLInputElement>();
    constructor(props: Props) {
        super(props);

        this.state = {
            editing: false
        };
        this.changeClipName = this.changeClipName.bind(this);
        this.enableEditingMode = this.enableEditingMode.bind(this);
        this.disableEditingMode = this.disableEditingMode.bind(this);
    }


    changeClipName(newName, setStore) {
        Communicator.invoke('change_clip_name', {
            clipType: 'composited',
            id: this.props.clip.id,
            name: newName.trim()
        }, (data) => {
            setStore(Store.deserialise(data));
        });
    }

    enableEditingMode() {
        this.setState({
            editing: true
        });

        requestAnimationFrame(() => {
            this.inputRef.current.focus();
        });
    }
    disableEditingMode(setStore) {
        if (this.inputRef.current) {
            this.changeClipName(this.inputRef.current.value, setStore);
        }
        this.setState({
            editing: false
        })
    }

    render() {
        return <StoreContext.Consumer>
            {({ value, setValue }) => {
                let text = (
                    <div>
                        <h1 className="text-gray-200 text-xl inline" onDoubleClick={this.enableEditingMode}>{this.props.clip.name.replaceAll(' ', '\u00a0')}</h1>
                        <button className="inline pt-2 ml-3 text-sm text-blue-600" onClick={this.enableEditingMode}><FontAwesomeIcon icon={faEdit} /></button>
                    </div>
                );
                if (this.state.editing) {
                    text = <input ref={this.inputRef} type="text" className="text-gray-200 bg-transparent border-0 text-xl focus:outline-none w-full"
                        defaultValue={this.props.clip.name} onBlur={() => this.disableEditingMode(setValue)} onKeyDown={(e) => {
                            if (e.key == "Enter") {
                                this.disableEditingMode(setValue);
                            }
                        }} />;
                }

                let ellipsisFileLocation = (l: string) => {
                    const threshold = 45;
                    if (l.length > threshold) {
                        return l.substr(0, 3) + "..." + l.substr(l.length - (threshold - 7));
                    }
                    return l;
                }

                return <div className="gap-2 inline-flex w-1/2">
                    <div>
                        <img src="https://via.placeholder.com/1920x1080" className="max-h-16" />
                    </div>
                    <div className="flex items-center">
                        <div>
                            {text}
                        </div>
                    </div>
                </div>

            }}
        </StoreContext.Consumer>;
    }

}

export default CompositedClipComponent;