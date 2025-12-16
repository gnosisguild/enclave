// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { hashMessage } from 'viem'
import { useSignMessage } from 'wagmi'

import { useVoteManagementContext } from '@/context/voteManagement'
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx'
import { Poll } from '@/model/poll.model'
import { BroadcastVoteRequest, VoteStateLite, VotingRound } from '@/model/vote.model'

import { encryptVote } from '@crisp-e3/sdk'

export type VotingStep = 'idle' | 'signing' | 'encrypting' | 'generating_proof' | 'broadcasting' | 'confirming' | 'complete' | 'error'

const extractCleanErrorMessage = (errorMessage: string | undefined): string => {
  if (!errorMessage) return 'Failed to broadcast the vote. Please try again.'

  if (errorMessage.includes('Internal error') || errorMessage.includes('-32603')) {
    return 'Transaction failed. The blockchain rejected the vote. Please try again.'
  }
  if (errorMessage.includes('insufficient funds')) {
    return 'Insufficient funds to process the transaction.'
  }
  if (errorMessage.includes('nonce')) {
    return 'Transaction conflict. Please try again.'
  }
  if (errorMessage.includes('gas')) {
    return 'Transaction failed due to gas issues. Please try again.'
  }
  if (errorMessage.includes('reverted')) {
    return 'Transaction was reverted by the contract.'
  }

  if (errorMessage.length > 100) {
    return 'Vote broadcast failed. Please try again.'
  }

  return errorMessage
}

export const useVoteCasting = (customRoundState?: VoteStateLite | null, customVotingRound?: VotingRound | null) => {
  const {
    user,
    roundState: contextRoundState,
    votingRound: contextVotingRound,
    generateProof,
    broadcastVote,
    setTxUrl,
    markVotedInRound,
    hasVotedInCurrentRound,
  } = useVoteManagementContext()

  const roundState = customRoundState ?? contextRoundState
  const votingRound = customVotingRound ?? contextVotingRound

  const { signMessageAsync } = useSignMessage()
  const { showToast } = useNotificationAlertContext()
  const navigate = useNavigate()
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const [votingStep, setVotingStep] = useState<VotingStep>('idle')
  const [lastActiveStep, setLastActiveStep] = useState<VotingStep | null>(null)
  const [stepMessage, setStepMessage] = useState<string>('')

  const handleProofGeneration = useCallback(
    async (vote: Poll, address: string, signature: string, messageHash: `0x${string}`, previousCiphertext?: Uint8Array) => {
      if (!votingRound) throw new Error('No voting round available for proof generation')
      return generateProof(BigInt(vote.value), new Uint8Array(votingRound.pk_bytes), address, signature, messageHash, previousCiphertext)
    },
    [generateProof, votingRound],
  )

  const resetVotingState = useCallback(() => {
    setVotingStep('idle')
    setLastActiveStep(null)
    setStepMessage('')
    setIsLoading(false)
  }, [])

  const castVoteWithProof = useCallback(
    async (pollSelected: Poll | null, isVoteUpdate: boolean = false) => {
      if (!pollSelected) {
        console.log('Cannot cast vote: Poll option not selected.')
        showToast({ type: 'danger', message: 'Please select a poll option first.' })
        return
      }
      if (!user || !roundState) {
        console.error('Cannot cast vote: Missing user or round state.')
        showToast({
          type: 'danger',
          message: 'Cannot cast vote. Ensure you are connected, and the round is active.',
          persistent: true,
        })
        return
      }

      setIsLoading(true)
      const actionText = isVoteUpdate ? 'Updating vote' : 'Processing vote'
      console.log(`${actionText}...`)

      try {
        // Step 1: Signing
        setVotingStep('signing')
        setLastActiveStep('signing')
        setStepMessage('Please sign the message in your wallet...')
        const message = `Vote for round ${roundState.id}`
        const messageHash = hashMessage(message)

        let signature: string
        try {
          signature = await signMessageAsync({ message })
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
        } catch (signError) {
          console.log('User rejected signature or signing failed')
          showToast({ type: 'danger', message: 'Signature cancelled or failed.' })
          resetVotingState()
          return
        }

        // Step 2: Encrypting vote
        setVotingStep('encrypting')
        setLastActiveStep('encrypting')
        setStepMessage('')

        // @todo get this from the contract or server
        const newEncryptionTemp = encryptVote({ yes: 0n, no: 0n }, new Uint8Array(votingRound!.pk_bytes))
        const previousCiphertext = isVoteUpdate ? newEncryptionTemp : undefined
        const encodedProof = await handleProofGeneration(pollSelected, user.address, signature, messageHash, previousCiphertext)
        if (!encodedProof) {
          throw new Error('Failed to encrypt vote.')
        }

        // Step 3: Generating proof
        setVotingStep('generating_proof')
        setLastActiveStep('generating_proof')

        // small delay for UX
        await new Promise((resolve) => setTimeout(resolve, 500))

        // Step 4: Broadcasting
        setVotingStep('broadcasting')
        setLastActiveStep('broadcasting')

        const voteRequest: BroadcastVoteRequest = {
          round_id: roundState.id,
          encoded_proof: encodedProof,
          address: user.address,
        }

        const broadcastVoteResponse = await broadcastVote(voteRequest)

        if (broadcastVoteResponse) {
          switch (broadcastVoteResponse.status) {
            case 'success': {
              setVotingStep('complete')
              setStepMessage('Vote submitted successfully!')

              const url = `https://sepolia.etherscan.io/tx/${broadcastVoteResponse.tx_hash}`
              setTxUrl(url)

              markVotedInRound(roundState.id)

              const successMessage = broadcastVoteResponse.is_vote_update ? 'Vote updated successfully!' : 'Vote submitted successfully!'
              showToast({
                type: 'success',
                message: successMessage,
                linkUrl: url,
              })
              navigate(`/result/${roundState.id}/confirmation`)
              break
            }
            case 'failed_broadcast':
              setVotingStep('error')
              showToast({
                type: 'danger',
                message: extractCleanErrorMessage(broadcastVoteResponse.message),
                persistent: true,
              })
              break
            default:
              setVotingStep('error')
              showToast({
                type: 'danger',
                message: extractCleanErrorMessage(broadcastVoteResponse.message),
                persistent: true,
              })
              break
          }
        } else {
          throw new Error('Received no response after broadcasting vote.')
        }
      } catch (error) {
        setVotingStep('error')
        console.error('Vote processing failed:', error)
        showToast({
          type: 'danger',
          message: `Vote failed: ${error instanceof Error ? error.message : String(error)}`,
          persistent: true,
        })
      } finally {
        setIsLoading(false)
      }
    },
    [
      user,
      roundState,
      broadcastVote,
      setTxUrl,
      showToast,
      navigate,
      handleProofGeneration,
      signMessageAsync,
      markVotedInRound,
      resetVotingState,
      votingRound,
    ],
  )

  return {
    castVoteWithProof,
    isLoading,
    votingStep,
    lastActiveStep,
    stepMessage,
    resetVotingState,
    hasVotedInCurrentRound,
  }
}
