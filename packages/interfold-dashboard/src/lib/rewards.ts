// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Committee seats, rewards, and failed-E3 compensation of one operator, behind the operator guide.
//
// When a committee is finalized, the refund manager emits one
// `RewardRecipientSnapshotted(e3Id, operator, recipient)` for each member. Its indexed operator topic
// lists every committee that selected an operator, and the wallet that receives the payments of that
// seat. Both payments are pull payments that only the recipient can claim:
// - A completed E3 pays a reward on the Interfold contract. `pendingReward(e3Id, recipient)` is the
//   amount that `claimReward(e3Id)` pays.
// - A failed E3 pays compensation to honest members on the refund manager, after
//   `Interfold.processE3Failure(e3Id)` calculates the refund. `claimHonestNodeReward(e3Id, operator)`
//   pays it. The contract has no view for that amount, so the read simulates the claim from the
//   recipient: the result is the amount, and a revert means there is nothing to claim.

import { BaseError, ContractFunctionRevertedError, getAbiItem, type Address } from 'viem'
import { CONTRACTS, DEPLOY_BLOCK, E3Stage, e3RefundManagerAbi, interfoldAbi, publicClient } from './chain'
import { getLogsChunked } from './e3'

const RECIPIENT_SNAPSHOTTED = getAbiItem({ abi: e3RefundManagerAbi, name: 'RewardRecipientSnapshotted' })

// viem types `readContract` and `simulateContract` on this client with a required
// `authorizationList`, so each call states its own result type. The ABIs still come from the
// typechain factories.
const readContract = publicClient.readContract as unknown as <T>(
  call:
    | {
        address: Address
        abi: typeof interfoldAbi
        functionName: 'e3RefundManager' | 'getE3Stage' | 'pendingReward'
        args?: readonly unknown[]
      }
    | { address: Address; abi: typeof e3RefundManagerAbi; functionName: 'getRefundDistribution'; args: readonly [bigint] },
) => Promise<T>
const simulateCompensationClaim = publicClient.simulateContract as unknown as (call: {
  account: Address
  address: Address
  abi: typeof e3RefundManagerAbi
  functionName: 'claimHonestNodeReward'
  args: readonly [bigint, Address]
}) => Promise<{ result: bigint }>

/** One committee that selected the operator, the wallet that receives its payments, and the refund manager that recorded it. */
export type CommitteeSeat = { e3Id: bigint; operator: Address; recipient: Address; refundManager: Address }

/**
 * `pending` is what the recipient can claim now: the reward of a completed E3, or the compensation of a
 * failed one. `failureProcessed` is null unless the E3 failed.
 */
export type SeatStatus = { stage: E3Stage; pending: bigint; failureProcessed: boolean | null }

/**
 * Every committee that selected `operator`, newest first. The scan covers the current refund manager
 * of the Interfold contract, so it does not list E3s that an earlier refund manager recorded.
 */
export async function fetchCommitteeSeats(operator: Address): Promise<CommitteeSeat[]> {
  const [refundManager, head] = await Promise.all([
    readContract<Address>({ address: CONTRACTS.Interfold, abi: interfoldAbi, functionName: 'e3RefundManager' }),
    publicClient.getBlockNumber(),
  ])
  const logs = await getLogsChunked<{ args: { e3Id: bigint; operator: Address; recipient: Address } }>(
    { address: refundManager, event: RECIPIENT_SNAPSHOTTED, args: { operator } },
    DEPLOY_BLOCK,
    head,
  )
  return logs.map(({ args }) => ({ e3Id: args.e3Id, operator: args.operator, recipient: args.recipient, refundManager })).reverse()
}

async function fetchCompensation(seat: CommitteeSeat): Promise<Pick<SeatStatus, 'pending' | 'failureProcessed'>> {
  const distribution = await readContract<{ calculated: boolean }>({
    address: seat.refundManager,
    abi: e3RefundManagerAbi,
    functionName: 'getRefundDistribution',
    args: [seat.e3Id],
  })
  if (!distribution.calculated) return { pending: 0n, failureProcessed: false }
  try {
    const { result } = await simulateCompensationClaim({
      account: seat.recipient,
      address: seat.refundManager,
      abi: e3RefundManagerAbi,
      functionName: 'claimHonestNodeReward',
      args: [seat.e3Id, seat.operator],
    })
    return { pending: result, failureProcessed: true }
  } catch (e) {
    // NotHonestNode, AlreadyClaimed, RewardPendingExpulsion, NoRefundAvailable: nothing to claim now.
    // Any other error (the RPC, for example) is not an answer, so it propagates.
    if (e instanceof BaseError && e.walk((cause) => cause instanceof ContractFunctionRevertedError)) {
      return { pending: 0n, failureProcessed: true }
    }
    throw e
  }
}

/** The E3 stage and what the seat's recipient can claim now, keyed by E3 id. */
export async function fetchSeatStatuses(seats: CommitteeSeat[]): Promise<Map<bigint, SeatStatus>> {
  const entries = await Promise.all(
    seats.map(async (seat): Promise<[bigint, SeatStatus]> => {
      const stage: E3Stage = await readContract<number>({
        address: CONTRACTS.Interfold,
        abi: interfoldAbi,
        functionName: 'getE3Stage',
        args: [seat.e3Id],
      })
      if (stage === E3Stage.Failed) return [seat.e3Id, { stage, ...(await fetchCompensation(seat)) }]
      const pending = await readContract<bigint>({
        address: CONTRACTS.Interfold,
        abi: interfoldAbi,
        functionName: 'pendingReward',
        args: [seat.e3Id, seat.recipient],
      })
      return [seat.e3Id, { stage, pending, failureProcessed: null }]
    }),
  )
  return new Map(entries)
}

const STAGE_LABELS: Record<E3Stage, string> = {
  [E3Stage.None]: 'Unknown',
  [E3Stage.Requested]: 'Requested',
  [E3Stage.CommitteeFinalized]: 'Key generation',
  [E3Stage.KeyPublished]: 'Accepting inputs',
  [E3Stage.CiphertextReady]: 'Decryption',
  [E3Stage.Complete]: 'Complete',
  [E3Stage.Failed]: 'Failed',
}

export function seatStageLabel(stage: E3Stage): string {
  return STAGE_LABELS[stage] ?? `Stage ${stage}`
}

export type SeatAction = 'claimReward' | 'claimCompensation' | 'processFailure'

/**
 * The transaction that `wallet` can send for a seat. Any wallet can process a failed E3; only the
 * recipient can claim, and only an amount above zero.
 */
export function seatAction(seat: CommitteeSeat, status: SeatStatus | undefined, wallet: Address | null): SeatAction | null {
  if (!status || !wallet) return null
  if (status.failureProcessed === false) return 'processFailure'
  if (status.pending === 0n || seat.recipient.toLowerCase() !== wallet.toLowerCase()) return null
  return status.stage === E3Stage.Failed ? 'claimCompensation' : 'claimReward'
}

/** The completed E3s whose reward `wallet` can claim now. `claimRewards(e3Ids)` claims them together. */
export function claimableE3s(seats: CommitteeSeat[], statuses: Map<bigint, SeatStatus>, wallet: Address | null): bigint[] {
  return seats.filter((seat) => seatAction(seat, statuses.get(seat.e3Id), wallet) === 'claimReward').map(({ e3Id }) => e3Id)
}
