// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useSignMessage } from 'wagmi'

import { useVoteManagementContext } from '@/context/voteManagement'
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx'
import { Poll } from '@/model/poll.model'
import { BroadcastVoteRequest } from '@/model/vote.model'
import { SIGNATURE_MESSAGE } from '@crisp-e3/sdk'

export const useVoteCasting = () => {
  const { user, roundState, votingRound, generateProof, broadcastVote, setTxUrl } = useVoteManagementContext()

  const { signMessageAsync } = useSignMessage()
  const { showToast } = useNotificationAlertContext()
  const navigate = useNavigate()
  const [isLoading, setIsLoading] = useState<boolean>(false)

  const handleProofGeneration = useCallback(
    async (vote: Poll, address: string, signature: string) => {
      if (!votingRound) throw new Error('No voting round available for proof generation')
      return generateProof(BigInt(vote.value), new Uint8Array(votingRound.pk_bytes), address, signature)
    },
    [generateProof, votingRound],
  )

  const castVoteWithProof = useCallback(
    async (pollSelected: Poll | null) => {
      if (!pollSelected) {
        console.log('Cannot cast vote: Poll option not selected.')
        showToast({ type: 'danger', message: 'Please select a poll option first.' })
        return
      }
      if (!user || !roundState) {
        console.error('Cannot cast vote: Missing user or round state.')
        showToast({ type: 'danger', message: 'Cannot cast vote. Ensure you are connected, and the round is active.' })
        return
      }

      setIsLoading(true)
      console.log('Processing vote...')

      // For now just sign and do not do nothing with the signature
      const signature = await signMessageAsync({ message: SIGNATURE_MESSAGE })

      try {
        const encodedProof = await handleProofGeneration(pollSelected, user.address, signature)
        if (!encodedProof) {
          throw new Error('Failed to generate proof.')
        }

        const voteRequest: BroadcastVoteRequest = {
          round_id: roundState.id,
          encoded_proof: encodedProof,
          address: user.address,
        }

        const broadcastVoteResponse = await broadcastVote(voteRequest)

        if (broadcastVoteResponse) {
          switch (broadcastVoteResponse.status) {
            case 'success': {
              const url = `https://sepolia.etherscan.io/tx/${broadcastVoteResponse.tx_hash}`
              setTxUrl(url)
              showToast({
                type: 'success',
                message: broadcastVoteResponse.message || 'Successfully voted',
                linkUrl: url,
              })
              navigate(`/result/${roundState.id}/confirmation`)
              break
            }
            case 'user_already_voted':
              showToast({
                type: 'danger',
                message: broadcastVoteResponse.message || 'User has already voted',
              })
              break
            case 'failed_broadcast':
              showToast({
                type: 'danger',
                message: 'Failed to broadcast the vote',
              })
              break
            default:
              showToast({
                type: 'danger',
                message: broadcastVoteResponse.message || 'Error broadcasting the vote',
              })
              break
          }
        } else {
          throw new Error('Received no response after broadcasting vote.')
        }
      } catch (error) {
        console.error('Vote processing failed:', error)
        showToast({ type: 'danger', message: `Vote failed: ${error instanceof Error ? error.message : String(error)}` })
      } finally {
        setIsLoading(false)
      }
    },
    [user, roundState, votingRound, generateProof, broadcastVote, setTxUrl, showToast, navigate, handleProofGeneration, signMessageAsync],
  )

  return { castVoteWithProof, isLoading }
}
