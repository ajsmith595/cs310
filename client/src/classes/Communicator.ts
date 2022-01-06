import { emit, listen } from '@tauri-apps/api/event';
import { invoke, InvokeArgs } from '@tauri-apps/api/tauri';
import { readBinaryFile } from '@tauri-apps/api/fs';


type Callback = (data?: any) => void;
export type ID = string;

export default class Communicator {

    private static isInitialised = false;
    private static listenMap = new Map<string, Array<Callback>>();

    private static _invoke(event: string, e: any) {
        let callbacks = this.listenMap.get(event);
        if (callbacks) {
            for (let callback of callbacks) {
                callback(e.payload);
            }
        }
    }

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
    static off(event: string, callback: Callback) {
        if (this.listenMap.has(event)) {
            this.listenMap.set(event, this.listenMap.get(event).filter(e => e != callback)); // remove the callback from the emission
        }
    }

    static send(event: string, data?: any) {
        emit(event, data);
    }

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


    static async readFile(filename: string) {
        return await readBinaryFile(filename);
    }
}