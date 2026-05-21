// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useEffect } from 'react'
import CardContent from '@/components/Cards/CardContent'
import { useVoteManagementContext } from '@/context/voteManagement'

const ConfirmVote: React.FC<{ confirmationUrl: string }> = ({ confirmationUrl }) => {
  const { setTxUrl } = useVoteManagementContext()

  useEffect(() => {
    return () => {
      setTxUrl(undefined)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return (
    <CardContent>
      <div className='col' style={{ gap: 10 }}>
        <p className='mono muted'>WHAT JUST HAPPENED?</p>
        <p className='lede' style={{ maxWidth: 'none' }}>
          Your vote was encrypted and{' '}
          <a href={confirmationUrl} target='_blank' rel='noreferrer' className='linkish'>
            posted onchain
          </a>{' '}
          by a relayer. When the poll is over, the results will be tallied using Fully Homomorphic Encryption (FHE) and the results
          decrypted using threshold cryptography, without revealing your identity or choice.
        </p>
      </div>
      <div className='col' style={{ gap: 10 }}>
        <p className='mono muted'>WHAT DOES THIS MEAN?</p>
        <p className='lede' style={{ maxWidth: 'none' }}>
          Your participation has directly contributed to a transparent and fair decision-making process, showcasing the power of
          privacy-preserving technology in governance and beyond. The use of CRISP in this vote represents a significant step towards
          secure, anonymous, and tamper-proof digital elections and polls. This innovation ensures that every vote counts equally while
          safeguarding against the risks of fraud and collusion, enhancing the reliability and trustworthiness of digital decision-making
          platforms.
        </p>
      </div>
    </CardContent>
  )
}

export default ConfirmVote
