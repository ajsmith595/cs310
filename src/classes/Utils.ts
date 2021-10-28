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
}