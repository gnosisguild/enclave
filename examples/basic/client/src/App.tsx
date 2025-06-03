import React from 'react'
import Wizard from './components/Wizard'
import './globals.css'

const App: React.FC = () => {
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 to-slate-100 text-slate-900">
      <Wizard />
    </div>
  )
}

export default App
