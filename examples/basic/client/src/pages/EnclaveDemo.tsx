import React, { useState, useEffect } from 'react';
import { useEnclaveSDK } from '../hooks/useEnclaveSDK';

const CONTRACT_ADDRESSES = {
    enclave: '0x0000000000000000000000000000000000000000' as `0x${string}`,
    ciphernodeRegistry: '0x0000000000000000000000000000000000000000' as `0x${string}`,
};

export const EnclaveDemo: React.FC = () => {
    const {
        isInitialized,
        isConnecting,
        error,
        connectWallet,
        requestE3,
        activateE3,
        publishInput,
        addCiphernode,
        removeCiphernode,
        getE3,
        getCiphernode,
        onEnclaveEvent,
        off,
        getHistoricalEvents,
        EnclaveEventType,
        RegistryEventType,
    } = useEnclaveSDK({
        contracts: CONTRACT_ADDRESSES,
        rpcUrl: 'http://localhost:8545', // Update this to your RPC URL
        autoConnect: true
    });

    const [events, setEvents] = useState<string[]>([]);
    const [formData, setFormData] = useState({
        // Request E3 form
        filter: '',
        threshold: [1, 1] as [number, number],
        startWindow: [BigInt(0), BigInt(100)] as [bigint, bigint],
        duration: BigInt(3600),
        e3Program: '',
        e3ProgramParams: '0x',
        computeProviderParams: '0x',
        value: BigInt(0),
        // Other forms
        e3Id: BigInt(0),
        publicKey: '',
        data: '0x',
        node: '',
        siblingNodes: [] as bigint[]
    });

    const addEvent = (eventText: string) => {
        setEvents(prev => [`${new Date().toLocaleTimeString()}: ${eventText}`, ...prev.slice(0, 19)]);
    };

    // Set up event listeners
    useEffect(() => {
        if (!isInitialized) return;

        // Listen to E3 events
        const handleE3Requested = (event: any) => {
            addEvent(`E3 Requested: ID ${event.data.e3Id}`);
        };

        const handleE3Activated = (event: any) => {
            addEvent(`E3 Activated: ID ${event.data.e3Id}`);
        };

        const handleInputPublished = (event: any) => {
            addEvent(`Input Published: E3 ID ${event.data.e3Id}`);
        };

        const handleCiphernodeAdded = (event: any) => {
            addEvent(`Ciphernode Added: ${event.data.node}`);
        };

        const handleCiphernodeRemoved = (event: any) => {
            addEvent(`Ciphernode Removed: ${event.data.node}`);
        };

        // Register event listeners
        onEnclaveEvent(EnclaveEventType.E3_REQUESTED, handleE3Requested);
        onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, handleE3Activated);
        onEnclaveEvent(EnclaveEventType.INPUT_PUBLISHED, handleInputPublished);
        onEnclaveEvent(RegistryEventType.CIPHERNODE_ADDED, handleCiphernodeAdded);
        onEnclaveEvent(RegistryEventType.CIPHERNODE_REMOVED, handleCiphernodeRemoved);

        // Cleanup function to remove listeners
        return () => {
            off(EnclaveEventType.E3_REQUESTED, handleE3Requested);
            off(EnclaveEventType.E3_ACTIVATED, handleE3Activated);
            off(EnclaveEventType.INPUT_PUBLISHED, handleInputPublished);
            off(RegistryEventType.CIPHERNODE_ADDED, handleCiphernodeAdded);
            off(RegistryEventType.CIPHERNODE_REMOVED, handleCiphernodeRemoved);
        };
    }, [isInitialized, onEnclaveEvent, off, EnclaveEventType, RegistryEventType]);

    const handleRequestE3 = async () => {
        try {
            const hash = await requestE3({
                filter: formData.filter as `0x${string}`,
                threshold: formData.threshold,
                startWindow: formData.startWindow,
                duration: formData.duration,
                e3Program: formData.e3Program as `0x${string}`,
                e3ProgramParams: formData.e3ProgramParams as `0x${string}`,
                computeProviderParams: formData.computeProviderParams as `0x${string}`,
                value: formData.value
            });
            addEvent(`E3 request transaction: ${hash}`);
        } catch (err) {
            addEvent(`Error requesting E3: ${err}`);
        }
    };

    const handleActivateE3 = async () => {
        try {
            const hash = await activateE3(
                formData.e3Id,
                formData.publicKey as `0x${string}`
            );
            addEvent(`E3 activation transaction: ${hash}`);
        } catch (err) {
            addEvent(`Error activating E3: ${err}`);
        }
    };

    const handlePublishInput = async () => {
        try {
            const hash = await publishInput(
                formData.e3Id,
                formData.data as `0x${string}`
            );
            addEvent(`Input published transaction: ${hash}`);
        } catch (err) {
            addEvent(`Error publishing input: ${err}`);
        }
    };

    const handleAddCiphernode = async () => {
        try {
            const hash = await addCiphernode(
                formData.node as `0x${string}`
            );
            addEvent(`Ciphernode added transaction: ${hash}`);
        } catch (err) {
            addEvent(`Error adding ciphernode: ${err}`);
        }
    };

    const handleRemoveCiphernode = async () => {
        try {
            const hash = await removeCiphernode(
                formData.node as `0x${string}`,
                formData.siblingNodes
            );
            addEvent(`Ciphernode removed transaction: ${hash}`);
        } catch (err) {
            addEvent(`Error removing ciphernode: ${err}`);
        }
    };

    const handleGetE3 = async () => {
        try {
            const result = await getE3(formData.e3Id);
            addEvent(`E3 data: ${JSON.stringify(result, null, 2)}`);
        } catch (err) {
            addEvent(`Error getting E3: ${err}`);
        }
    };

    const handleGetCiphernode = async () => {
        try {
            const result = await getCiphernode(formData.node as `0x${string}`);
            addEvent(`Ciphernode data: ${JSON.stringify(result, null, 2)}`);
        } catch (err) {
            addEvent(`Error getting ciphernode: ${err}`);
        }
    };

    const handleGetHistoricalEvents = async () => {
        try {
            const logs = await getHistoricalEvents(EnclaveEventType.E3_REQUESTED);
            addEvent(`Found ${logs.length} historical E3 Requested events`);
        } catch (err) {
            addEvent(`Error getting historical events: ${err}`);
        }
    };

    if (error) {
        return (
            <div className="min-h-screen bg-gray-100 py-12 px-4 sm:px-6 lg:px-8">
                <div className="max-w-md mx-auto">
                    <div className="bg-red-50 border border-red-200 rounded-md p-4">
                        <div className="flex">
                            <div className="ml-3">
                                <h3 className="text-sm font-medium text-red-800">
                                    SDK Error
                                </h3>
                                <div className="mt-2 text-sm text-red-700">
                                    {error}
                                </div>
                                {!isInitialized && (
                                    <button
                                        onClick={connectWallet}
                                        disabled={isConnecting}
                                        className="mt-3 bg-red-600 text-white px-4 py-2 rounded-md text-sm font-medium hover:bg-red-700 disabled:opacity-50"
                                    >
                                        {isConnecting ? 'Connecting...' : 'Connect Wallet'}
                                    </button>
                                )}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="min-h-screen bg-gray-100 py-12 px-4 sm:px-6 lg:px-8">
            <div className="max-w-7xl mx-auto">
                <div className="text-center">
                    <h1 className="text-3xl font-extrabold text-gray-900">Enclave SDK Demo</h1>
                    <p className="mt-4 text-lg text-gray-600">
                        Interact with Enclave smart contracts using the TypeScript SDK
                    </p>

                    {!isInitialized && (
                        <div className="mt-6">
                            <button
                                onClick={connectWallet}
                                disabled={isConnecting}
                                className="bg-blue-600 text-white px-6 py-3 rounded-md text-lg font-medium hover:bg-blue-700 disabled:opacity-50"
                            >
                                {isConnecting ? 'Connecting...' : 'Connect Wallet'}
                            </button>
                        </div>
                    )}
                </div>

                {isInitialized && (
                    <div className="mt-12 grid grid-cols-1 lg:grid-cols-3 gap-8">
                        {/* Contract Interactions */}
                        <div className="lg:col-span-2 space-y-8">
                            {/* Request E3 */}
                            <div className="bg-white shadow rounded-lg p-6">
                                <h3 className="text-lg font-medium text-gray-900 mb-4">Request E3</h3>
                                <div className="grid grid-cols-1 gap-4">
                                    <input
                                        type="text"
                                        placeholder="Filter Address"
                                        value={formData.filter}
                                        onChange={(e) => setFormData(prev => ({ ...prev, filter: e.target.value }))}
                                        className="border border-gray-300 rounded-md px-3 py-2"
                                    />
                                    <input
                                        type="text"
                                        placeholder="E3 Program Address"
                                        value={formData.e3Program}
                                        onChange={(e) => setFormData(prev => ({ ...prev, e3Program: e.target.value }))}
                                        className="border border-gray-300 rounded-md px-3 py-2"
                                    />
                                    <button
                                        onClick={handleRequestE3}
                                        className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700"
                                    >
                                        Request E3
                                    </button>
                                </div>
                            </div>

                            {/* Activate E3 */}
                            <div className="bg-white shadow rounded-lg p-6">
                                <h3 className="text-lg font-medium text-gray-900 mb-4">Activate E3</h3>
                                <div className="grid grid-cols-1 gap-4">
                                    <input
                                        type="text"
                                        placeholder="E3 ID"
                                        value={formData.e3Id.toString()}
                                        onChange={(e) => setFormData(prev => ({ ...prev, e3Id: BigInt(e.target.value || 0) }))}
                                        className="border border-gray-300 rounded-md px-3 py-2"
                                    />
                                    <input
                                        type="text"
                                        placeholder="Public Key"
                                        value={formData.publicKey}
                                        onChange={(e) => setFormData(prev => ({ ...prev, publicKey: e.target.value }))}
                                        className="border border-gray-300 rounded-md px-3 py-2"
                                    />
                                    <button
                                        onClick={handleActivateE3}
                                        className="bg-green-600 text-white px-4 py-2 rounded-md hover:bg-green-700"
                                    >
                                        Activate E3
                                    </button>
                                </div>
                            </div>

                            {/* Other Operations */}
                            <div className="bg-white shadow rounded-lg p-6">
                                <h3 className="text-lg font-medium text-gray-900 mb-4">Other Operations</h3>
                                <div className="space-y-4">
                                    <button
                                        onClick={handlePublishInput}
                                        className="w-full bg-purple-600 text-white px-4 py-2 rounded-md hover:bg-purple-700"
                                    >
                                        Publish Input
                                    </button>
                                    <button
                                        onClick={handleAddCiphernode}
                                        className="w-full bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700"
                                    >
                                        Add Ciphernode
                                    </button>
                                    <button
                                        onClick={handleRemoveCiphernode}
                                        className="w-full bg-red-600 text-white px-4 py-2 rounded-md hover:bg-red-700"
                                    >
                                        Remove Ciphernode
                                    </button>
                                    <button
                                        onClick={handleGetE3}
                                        className="w-full bg-gray-600 text-white px-4 py-2 rounded-md hover:bg-gray-700"
                                    >
                                        Get E3 Data
                                    </button>
                                    <button
                                        onClick={handleGetCiphernode}
                                        className="w-full bg-gray-600 text-white px-4 py-2 rounded-md hover:bg-gray-700"
                                    >
                                        Get Ciphernode Data
                                    </button>
                                    <button
                                        onClick={handleGetHistoricalEvents}
                                        className="w-full bg-yellow-600 text-white px-4 py-2 rounded-md hover:bg-yellow-700"
                                    >
                                        Get Historical Events
                                    </button>
                                </div>
                            </div>
                        </div>

                        {/* Event Log */}
                        <div className="bg-white shadow rounded-lg p-6">
                            <h3 className="text-lg font-medium text-gray-900 mb-4">Real-time Events</h3>
                            <div className="h-96 overflow-y-auto border border-gray-200 rounded-md p-4">
                                {events.length === 0 ? (
                                    <p className="text-gray-500 text-sm">No events yet. Interact with contracts to see real-time events.</p>
                                ) : (
                                    events.map((event, index) => (
                                        <div key={index} className="text-sm text-gray-700 mb-2 p-2 bg-gray-50 rounded">
                                            {event}
                                        </div>
                                    ))
                                )}
                            </div>
                            <button
                                onClick={() => setEvents([])}
                                className="mt-4 w-full bg-gray-500 text-white px-4 py-2 rounded-md hover:bg-gray-600 text-sm"
                            >
                                Clear Events
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}; 