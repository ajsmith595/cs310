import { useEffect } from "react";
import { useStoreState } from "react-flow-renderer";
import EventBus from "../../classes/EventBus";



function NodeEditorStateManager(props) {

    const state = useStoreState((state) => state);
    useEffect(() => {
        EventBus.registerGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_INTERNAL_STATE, () => {
            return state;
        });

        return () => {
            EventBus.unregisterGetter(EventBus.GETTERS.NODE_EDITOR.CURRENT_INTERNAL_STATE);
        }
    });


    return null;
}

export default NodeEditorStateManager;