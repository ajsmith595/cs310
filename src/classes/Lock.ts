export default class Lock {
    locked: boolean;
    callbacks_awaiting: Array<() => void>;

    constructor() {
        this.locked = false;
        this.callbacks_awaiting = [];
    }


    async lock() {
        if (this.locked) {
            await new Promise((resolve, reject) => {
                this.callbacks_awaiting.push(() => resolve(null));
            });
        }
        this.locked = true;
    }
    release() {
        this.locked = false;
        if (this.callbacks_awaiting.length > 0) {
            this.callbacks_awaiting.shift()();
        }
    }
}