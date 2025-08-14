// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useState } from 'react'
import { PrismLight as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'
import PaperIcon from '@/assets/icons/paper.svg'
import PaperPurpleIcon from '@/assets/icons/paperPurple.svg'
import FingerprintIcon from '@/assets/icons/fingerprint.svg'
import FingerprintWhiteIcon from '@/assets/icons/fingerprintWhite.svg'

interface CodeTextDisplayProps {}

const selectedClass = 'border-slate-600/80 flex space-x-2 rounded-lg border-2 bg-white px-4 py-2'
const unSelectedClass = 'flex space-x-2 rounded-lg border-2 border-slate-600/20 bg-[#B7BBC1] px-4 py-2'

const CodeTextDisplay: React.FC<CodeTextDisplayProps> = () => {
  const text = `import React from 'react'

  interface CardContentProps {
    children: React.ReactNode
  }
  
  const CardContent: React.FC<CardContentProps> = ({ children }) => {
    return (
      <div className='min-h-[716px] w-full max-w-[900px] space-y-10 rounded-[24px] border-2 border-slate-600/20 bg-white p-12 shadow-2xl'>
        {children}
      </div>
    )
  }
  
  export default CardContent
  `

  const [isCipher, setIsCipher] = useState<boolean>(true)

  return (
    <div className='rounded-lg shadow'>
      <div className='flex space-x-2 rounded-t-lg border-x-2 border-t-2 border-slate-600/20  bg-slate-600/10 px-4 py-2'>
        <button className={isCipher ? unSelectedClass : selectedClass} onClick={() => setIsCipher(false)}>
          <img src={isCipher ? PaperIcon : PaperPurpleIcon} />
          <p className={`${isCipher ? 'text-white/80' : 'text-indigo-400'} text-base font-semibold`}>Plain Text</p>
        </button>
        <button className={isCipher ? selectedClass : unSelectedClass} onClick={() => setIsCipher(true)}>
          <img src={isCipher ? FingerprintIcon : FingerprintWhiteIcon} />
          <p className={`${!isCipher ? 'text-white/80' : 'text-indigo-400'} text-base font-semibold`}>Cypher Text</p>
        </button>
      </div>
      <div className=' rounded-b-lg border-2 border-slate-600/20 p-5'>
        {text ? (
          isCipher ? (
            <SyntaxHighlighter language='javascript' style={oneLight}>
              {text}
            </SyntaxHighlighter>
          ) : (
            text
          )
        ) : (
          'Loading...'
        )}
      </div>
    </div>
  )
}

export default CodeTextDisplay
