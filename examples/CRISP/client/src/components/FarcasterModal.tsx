import { useVoteManagementContext } from '@/context/voteManagement'
import useLocalStorage from '@/hooks/generic/useLocalStorage'
import { AuthClientError, QRCode, StatusAPIResponse } from '@farcaster/auth-kit'
import { DeviceMobileCamera } from '@phosphor-icons/react'
import React, { useEffect } from 'react'
import { Link } from 'react-router-dom'

interface FarcasterModalProps {
  url?: string
  data: StatusAPIResponse | undefined
  error: AuthClientError | undefined
  onClose: () => void
}

const FarcasterModal: React.FC<FarcasterModalProps> = ({ url, data, error, onClose }) => {
  const { setUser } = useVoteManagementContext()
  const [farcasterAuth, setFarcasterUser] = useLocalStorage<StatusAPIResponse | null>('farcasterAuth', null)

  useEffect(() => {
    if (data && data.state === 'completed' && !farcasterAuth) {
      setUser(data)
      setFarcasterUser(data)
      onClose()
    }
  }, [data, setFarcasterUser])

  return (
    <div className='mt-4 space-y-10'>
      <div className='flex flex-col items-center justify-center'>
        <div className='flex flex-col space-y-2'>
          <h2 className='text-xl font-bold text-slate-600'>Verify your account with farcaster</h2>
          {!error && (
            <>
              <p className='text-sm'>Scan with your phone's camera to continue.</p>
              <Link to='https://warpcast.com/~/signup' target='_blank'>
                <p className='text-sm text-lime-600 underline'>Need to create an account?</p>
              </Link>
            </>
          )}
        </div>
        {!error && (
          <>
            {url && (
              <div className='my-8'>
                <QRCode uri={url} size={260} logoSize={40} />
              </div>
            )}
            {url && (
              <div className='flex items-center space-x-2'>
                <Link to={url} target='_blank' className='flex items-center space-x-2'>
                  <DeviceMobileCamera size={24} className='text-lime-600' />
                  <p className='text-base text-lime-600 underline'>I'm using my phone</p>
                </Link>
              </div>
            )}
          </>
        )}
        {error && (
          <p className='mt-4 text-center'>
            Your polling request has timed out after 5 minutes. This may occur if you haven't scanned the QR code using Farcaster, or if
            there was an issue during the process. Please ensure you have completed the QR scan and try again.
          </p>
        )}
      </div>
    </div>
  )
}

export default FarcasterModal
