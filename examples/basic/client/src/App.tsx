import React, { useState } from 'react'
import Wizard from './pages/Wizard'
import { EnclaveDemo } from './pages/EnclaveDemo'
import './globals.css'

const App: React.FC = () => {
  const [currentPage, setCurrentPage] = useState<'wizard' | 'demo'>('wizard')

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 text-slate-900">
      {/* Navigation */}
      <nav className="p-4 bg-white shadow-sm">
        <div className="max-w-6xl mx-auto flex space-x-4">
          <button
            onClick={() => setCurrentPage('wizard')}
            className={`px-4 py-2 rounded ${currentPage === 'wizard'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
              }`}
          >
            Wizard
          </button>
          <button
            onClick={() => setCurrentPage('demo')}
            className={`px-4 py-2 rounded ${currentPage === 'demo'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
              }`}
          >
            SDK Demo
          </button>
        </div>
      </nav>

      {/* Page Content */}
      {currentPage === 'wizard' ? <Wizard /> : <EnclaveDemo />}
    </div>
  )
}

export default App
