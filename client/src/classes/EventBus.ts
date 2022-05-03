function doc() {
    return document as any;
}

const NODE_EDITOR_BASE = "node_editor.";
const APP_BASE = "app.";


/**
 * A global event bus to allow different components to communicate
 */
const EventBus = {
    /**
     * Listen for a particular custom event
     */
    on(event, callback) {
        document.addEventListener(event, (e) => callback(e.detail));
    },
    /**
     * Dispatch a certain event to any event listeners
     */
    dispatch(event, data) {
        document.dispatchEvent(new CustomEvent(event, { detail: data }));
    },
    /**
     * Unlisten from a particular event
     */
    remove(event, callback) {
        document.removeEventListener(event, callback);
    },

    /**
     * Call a particular getter to obtain its value
     */
    getValue(value) {
        if (!doc()._getters) {
            return null;
        }
        if (!doc()._getters[value]) {
            return null;
        }
        return doc()._getters[value]();
    },
    /**
     * Register a particular getter so that if `getValue` is called, the passed function will be called to obtain a value
     */
    registerGetter(name: string, callback: () => any) {
        if (!doc()._getters) {
            doc()._getters = {};
        }
        if (doc()._getters[name]) {
            throw new Error("Getter '" + name + "' already registered!");
        }
        doc()._getters[name] = callback;
    },
    /**
     * Reverses `registerGetter`
     */
    unregisterGetter(name: string) {
        if (!doc()._getters) {
            doc()._getters = {};
        }
        delete doc()._getters[name];
    },

    /**
     * A set of all custom events passed around in the application
     */
    EVENTS: {
        NODE_EDITOR: {
            ADD_NODE: NODE_EDITOR_BASE + "add_node",
            CHANGE_GROUP: NODE_EDITOR_BASE + "change_group",
            FORCE_UPDATE: NODE_EDITOR_BASE + "force_update"
        },
        APP: {
            SET_SELECTION: APP_BASE + "set_selection"
        }
    },
    /**
     * A set of all getters used around the application
     */
    GETTERS: {
        NODE_EDITOR: {
            CURRENT_GROUP: NODE_EDITOR_BASE + "current_group",
            CURRENT_INTERNAL_STATE: NODE_EDITOR_BASE + "current_internal_state"
        },
        APP: {
            STORE: APP_BASE + "store",
            CURRENT_SELECTION: APP_BASE + "selection"
        }
    }
};

export default EventBus;