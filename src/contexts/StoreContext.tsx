import React from 'react';
import Store from '../classes/Store';

interface Context {
    value: Store,
    setValue: (val: Store) => void;
}

const StoreContext = React.createContext<Context>({
    value: new Store(),
    setValue: (newValue) => { },
})

export default StoreContext;