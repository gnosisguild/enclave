// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import GnosisGuildLogo from '@/assets/icons/gg.svg'
import { Link } from 'react-router-dom'
import { CastleTurret, GithubLogo, TelegramLogo, TwitterLogo } from '@phosphor-icons/react'

const Footer: React.FC = () => {
  return (
    <footer className='relative z-10 flex w-full border-t-2 border-slate-600/20 bg-slate-200 p-6'>
      <div className='mx-auto flex w-full max-w-screen-xl items-center justify-start gap-3 md:flex-row'>
        <Link to='https://github.com/gnosisguild/enclave' target='_blank'>
          <GithubLogo size={24} />
        </Link>
        <Link to='https://x.com/EnclaveE3' target='_blank'>
          <TwitterLogo size={24} />
        </Link>
        <Link to='https://t.me/enclave_e3' target='_blank'>
          <TelegramLogo size={24} />
        </Link>
        <Link to='https://warpcast.com/enclavee3' target='_blank'>
          <CastleTurret size={24} />
        </Link>
      </div>
      <div className='mx-auto flex w-full max-w-screen-xl flex-col items-center justify-end gap-1 md:flex-row'>
        <div className='flex items-center gap-1'>
          <p className='text-sm'>Secured with</p>
          <Link to='https://enclave.gg' target='_blank'>
            <p className='font-serif font-bold'>Enclave</p>
          </Link>
        </div>
        <div className='flex items-center gap-1'>
          <p className='text-sm'>built by</p>
          <div className='flex items-center gap-1 duration-300 ease-in-out hover:opacity-70'>
            <Link to='https://www.gnosisguild.org/' target='_blank' className='flex items-center gap-2 md:flex-row'>
              <p className='text-sm font-bold'>Gnosis Guild</p>
              <img src={GnosisGuildLogo} className='h-6 w-6' />
            </Link>
          </div>
        </div>
      </div>
    </footer>
  )
}

export default Footer
