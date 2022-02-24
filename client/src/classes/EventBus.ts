function doc() {
    return document as any;
}

const NODE_EDITOR_BASE = "node_editor.";
const APP_BASE = "app.";

const EventBus = {
    on(event, callback) {
        document.addEventListener(event, (e) => callback(e.detail));
    },
    dispatch(event, data) {
        document.dispatchEvent(new CustomEvent(event, { detail: data }));
    },
    remove(event, callback) {
        document.removeEventListener(event, callback);
    },
    getValue(value) {
        if (!doc()._getters) {
            return null;
        }
        if (!doc()._getters[value]) {
            return null;
        }
        return doc()._getters[value]();
    },
    registerGetter(name: string, callback: () => any) {
        if (!doc()._getters) {
            doc()._getters = {};
        }
        if (doc()._getters[name]) {
            throw new Error("Getter '" + name + "' already registered!");
        }
        doc()._getters[name] = callback;
    },
    unregisterGetter(name: string) {
        if (!doc()._getters) {
            doc()._getters = {};
        }
        delete doc()._getters[name];
    },

    EVENTS: {
        NODE_EDITOR: {
            ADD_NODE: NODE_EDITOR_BASE + "add_node",
            CHANGE_GROUP: NODE_EDITOR_BASE + "change_group"
        },
        APP: {
            SET_SELECTION: APP_BASE + "set_selection"
        }
    },
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