import { emit, listen } from '@tauri-apps/api/event';
import { invoke, InvokeArgs } from '@tauri-apps/api/tauri';
import { readBinaryFile } from '@tauri-apps/api/fs';


type Callback = (data?: any) => void;
export type ID = string;


/**
 * An abstraction over the Tauri API for handling IPC calls
 */

export default class Communicator {

    private static listenMap = new Map<string, Array<Callback>>();

    private static _invoke(event: string, e: any) {
        let callbacks = this.listenMap.get(event);
        if (callbacks) {
            for (let callback of callbacks) {
                callback(e.payload);
            }
        }
    }

    /**
     * Listen for a certain event; callback will be called when the event is emitted
     */
    static async on(event: string, callback: Callback) {
        if (!this.listenMap.has(event)) {
            this.listenMap.set(event, []);

            await listen(event, (e) => {
                this._invoke(event, e)
            });
        }
        if (!this.listenMap.get(event).includes(callback)) {
            this.listenMap.get(event).push(callback);
        }
    }
    /**
     * Opposite of `on` - will stop listening to a particular call
     */
    static off(event: string, callback: Callback) {
        if (this.listenMap.has(event)) {
            this.listenMap.set(event, this.listenMap.get(event).filter(e => e != callback)); // remove the callback from the emission
        }
    }

    /**
     * Clears all event handlers for a particular event
     */
    static clear(event: string) {
        if (this.listenMap.has(event)) {
            this.listenMap.delete(event);
        }
    }

    /**
     * Sends a message to the Rust backend
     */
    static send(event: string, data?: any) {
        emit(event, data);
    }

    /**
     * Invokes a certain Rust command in the backend; `callback` can be used to capture returned data
     */
    static invoke(event: string, args?: InvokeArgs, callback?: Callback) {
        let promise = invoke(event, args);
        if (callback) {
            promise.then(callback);
        }
        promise.catch(err => {
            console.log("Invoke error caught!");
            console.log(err);
        })
    }


    /**
     * Returns the contents of a file in the filesystem
     */
    static async readFile(filename: string) {
        return await readBinaryFile(filename);
    }
}