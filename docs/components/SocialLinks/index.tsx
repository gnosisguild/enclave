// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { FarcasterLogo, TelegramLogo, XLogo } from './SocialIcons'
import { ReactElement } from 'react'

interface SocialLinksProps {
  name: string
  icon: ReactElement
  url: string
}

export const socialLinks: SocialLinksProps[] = [
  {
    name: 'twitter',
    icon: <XLogo size={24} />,
    url: 'https://x.com/EnclaveE3',
  },
  {
    name: 'farcaster',
    icon: <FarcasterLogo size={24} />,
    url: 'https://warpcast.com/enclavee3',
  },
  {
    name: 'telegram',
    icon: <TelegramLogo size={24} />,
    url: 'https://t.me/enclave_e3',
  },
]
