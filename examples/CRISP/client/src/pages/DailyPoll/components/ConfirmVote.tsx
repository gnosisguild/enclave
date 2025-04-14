import React, { useEffect } from 'react'
import CardContent from '@/components/Cards/CardContent'
import { useVoteManagementContext } from '@/context/voteManagement'

const ConfirmVote: React.FC<{ confirmationUrl: string }> = ({ confirmationUrl }) => {
  const { setTxUrl } = useVoteManagementContext()

  useEffect(() => {
    return () => {
      setTxUrl(undefined)
    }
  }, [])

  return (
    <CardContent>
      <div className='space-y-4'>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>WHAT JUST HAPPENED?</p>
        <div className='space-y-2'>
          <p className='text-xl leading-8 text-slate-600'>
            Your vote was encrypted and{' '}
            <a href={confirmationUrl} target='_blank' className={`text-lime-600 underline`}>
              posted onchain
            </a>{' '}
            by a relayer. When the poll is over, the results will be tallied using Fully Homomorphic Encryption (FHE) and the results
            decrypted using threshold cryptography, without revealing your identity or choice.
          </p>
        </div>
      </div>
      <div className='space-y-4'>
        <p className='text-base font-extrabold uppercase text-slate-600/50'>WHAT DOES THIS MEAN?</p>
        <p className='text-xl leading-8 text-slate-600'>
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
