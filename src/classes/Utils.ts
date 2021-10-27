export default class Utils {
    static propsUndefined(...props: any) {
        for (let prop of props) {
            if (typeof prop == "undefined") {
                return true;
            }
        }
        return false;
    }
}