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
    <div className='crisp-editorial' data-palette='interfold' data-mode='light' data-density='comfortable'>
      <footer className='footer'>
        <span>© 2026 — Crisp Protocol</span>
        <span className='muted'>Secret-ballot voting with FHE + threshold MPC</span>
        <div className='links' style={{ alignItems: 'center' }}>
          <Link to='https://github.com/gnosisguild/enclave' target='_blank' aria-label='GitHub'>
            <GithubLogo size={18} />
          </Link>
          <Link to='https://x.com/EnclaveE3' target='_blank' aria-label='X'>
            <TwitterLogo size={18} />
          </Link>
          <Link to='https://t.me/enclave_e3' target='_blank' aria-label='Telegram'>
            <TelegramLogo size={18} />
          </Link>
          <Link to='https://warpcast.com/enclavee3' target='_blank' aria-label='Farcaster'>
            <CastleTurret size={18} />
          </Link>
          <Link to='https://theinterfold.com' target='_blank'>
            Secured with The Interfold
          </Link>
          <Link to='https://www.gnosisguild.org/' target='_blank' style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            Gnosis Guild
            <img src={GnosisGuildLogo} className='h-4 w-4' alt='Gnosis Guild' />
          </Link>
        </div>
      </footer>
    </div>
  )
}

export default Footer
