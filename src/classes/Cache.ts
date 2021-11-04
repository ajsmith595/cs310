

const Cache = {
    cache() {
        if (!(document as any)._cache) {
            (document as any)._cache = new Map();
        }
        return (document as any)._cache;
    },
    put(key, value) {
        this.cache().set(key, value);
    },
    get(key) {
        return this.cache().get(key);
    },
    clear(key) {
        this.cache().delete(key);
    }
};
export default Cache;