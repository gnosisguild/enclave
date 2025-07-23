// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import { Warning } from '@phosphor-icons/react'

interface EnvironmentErrorProps {
    missingVars: string[]
}

const EnvironmentError: React.FC<EnvironmentErrorProps> = ({ missingVars }) => {
    return (
        <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-slate-50 to-slate-100 p-4">
            <div className="max-w-2xl w-full bg-white rounded-2xl shadow-xl border border-red-200 p-8">
                <div className="text-center">
                    <div className="mx-auto flex items-center justify-center w-16 h-16 bg-red-100 rounded-full mb-6">
                        <Warning size={32} className="text-red-600" />
                    </div>

                    <h1 className="text-2xl font-bold text-gray-900 mb-4">
                        Environment Configuration Required
                    </h1>

                    <p className="text-gray-600 mb-6">
                        The following environment variables need to be configured before you can use the encrypted computation features:
                    </p>

                    <div className="bg-gray-50 rounded-lg p-4 mb-6">
                        <ul className="space-y-2 text-left">
                            {missingVars.map((varName) => (
                                <li key={varName} className="flex items-center space-x-2">
                                    <code className="bg-red-100 text-red-700 px-2 py-1 rounded font-mono text-sm">
                                        {varName}
                                    </code>
                                </li>
                            ))}
                        </ul>
                    </div>

                    <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 text-left">
                        <h3 className="font-semibold text-blue-900 mb-2">How to configure:</h3>
                        <ol className="space-y-1 text-sm text-blue-800 list-decimal list-inside">
                            <li>Create a <code className="bg-blue-100 px-1 rounded">.env</code> file in the client directory</li>
                            <li>Add the missing environment variables with their appropriate values</li>
                            <li>Restart the development server</li>
                        </ol>
                    </div>

                    <div className="mt-6">
                        <button
                            onClick={() => window.location.reload()}
                            className="w-full rounded-lg bg-blue-600 px-6 py-3 font-semibold text-white transition-all duration-200 hover:bg-blue-500 hover:shadow-md disabled:cursor-not-allowed disabled:bg-slate-300 disabled:text-slate-500"
                        >
                            Reload Page
                        </button>
                    </div>
                </div>
            </div>
        </div>
    )
}

export default EnvironmentError 