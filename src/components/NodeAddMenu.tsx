
import { Menu, Transition } from '@headlessui/react'
import React from 'react';
import { Fragment, useEffect, useRef, useState } from 'react'
import { ReactFlowProvider } from 'react-flow-renderer';
import EventBus from '../classes/EventBus';
import EditorNode, { Position } from '../classes/Node'
import NodeEditorContext from '../contexts/NodeEditorContext';

export default function NodeAddMenu() {

    function dispatchAddNode(node_type) {
        let state = EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_INTERNAL_STATE);
        let x = (state.width / 2 - state.transform[0]) / state.transform[2];
        let y = (state.height / 2 - state.transform[1]) / state.transform[2];

        let pos = new Position(x, y);
        let node = EditorNode.createNode(node_type.id, EventBus.getValue(EventBus.GETTERS.NODE_EDITOR.CURRENT_GROUP), pos);

        EventBus.dispatch(EventBus.EVENTS.NODE_EDITOR.ADD_NODE, node);
    };

    let register = EditorNode.NodeRegister;
    let items = [];
    for (let [id, node_type] of register.entries()) {
        items.push(
            <Menu.Item>
                {({ active }) => (
                    <button
                        className={`${active ? 'bg-pink-600' : ''
                            } group flex rounded-md items-center w-full px-2 py-2 text-sm text-white`}
                        onClick={() => dispatchAddNode(node_type)}
                    >
                        {node_type.display_name}
                    </button>
                )
                }
            </Menu.Item >
        );
    }

    return (
        <Menu as="div" className="relative">
            <div>
                <Menu.Button className="inline-flex justify-center w-full px-4 py-2 text-sm font-medium text-white bg-gray-900 rounded-md hover:bg-gray-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-opacity-75">
                    Add Node
                </Menu.Button>
            </div>
            <Transition
                as={Fragment}
                enter="transition ease-out duration-100"
                enterFrom="transform opacity-0 scale-95"
                enterTo="transform opacity-100 scale-100"
                leave="transition ease-in duration-75"
                leaveFrom="transform opacity-100 scale-100"
                leaveTo="transform opacity-0 scale-95"
            >
                <Menu.Items className="absolute right-0 w-56 mt-2 origin-top-right bg-gray-900 divide-y divide-gray-100 rounded-md shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
                    <div className="px-1 py-1 ">
                        {items}
                    </div>
                </Menu.Items>
            </Transition>
        </Menu>
    )
}
