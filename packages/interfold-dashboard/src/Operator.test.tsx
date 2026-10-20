// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import assert from 'node:assert/strict'
import test from 'node:test'
import { renderToStaticMarkup } from 'react-dom/server'
import type { Address } from 'viem'
import { ManagePosition, positionActions, ticketBurnAmount } from './Operator'
import type { BondingConfig, OperatorStatus } from './lib/bonding'

const OPERATOR = '0x1111111111111111111111111111111111111111' as Address
const OWNER = '0x2222222222222222222222222222222222222222' as Address
const PROPOSED = '0x3333333333333333333333333333333333333333' as Address
const OTHER = '0x4444444444444444444444444444444444444444' as Address
const UNIT = 10n ** 18n

const config: BondingConfig = {
  ciphernodeBondToken: OTHER,
  ciphernodeBondSymbol: 'FOLD',
  ciphernodeBondDecimals: 18,
  ticketToken: OTHER,
  ticketBase: OTHER,
  ticketSymbol: 'USDS',
  ticketDecimals: 18,
  requiredCiphernodeBond: 1000n * UNIT,
  ticketPrice: UNIT,
  minTicketBalance: 5n,
  exitDelay: 604_800n,
}

function position(overrides: Partial<OperatorStatus> = {}): OperatorStatus {
  return {
    bondOwner: OWNER,
    pendingBondOwner: null,
    ciphernodeBond: 1000n * UNIT,
    ticketBalance: 5n * UNIT,
    availableTickets: 5n,
    bonded: true,
    registered: true,
    active: true,
    eligible: true,
    exitInProgress: false,
    pendingTicketExit: 0n,
    pendingBondExit: 0n,
    claimableTicket: 0n,
    claimableBond: 0n,
    ...overrides,
  }
}

test('the bond owner sends every position change, and the operator key can only deregister', () => {
  const owner = positionActions(position(), OPERATOR, OWNER)
  assert.deepEqual([owner.refresh, owner.burn, owner.unbond, owner.deregister, owner.proposeOwner], [true, true, true, true, true])

  const operatorKey = positionActions(position(), OPERATOR, OPERATOR)
  assert.deepEqual([operatorKey.deregister, operatorKey.burn, operatorKey.unbond, operatorKey.proposeOwner], [true, false, false, false])

  const stranger = positionActions(position(), OPERATOR, OTHER)
  assert.deepEqual([stranger.refresh, stranger.deregister, stranger.burn], [true, false, false])
  assert.equal(positionActions(position(), OPERATOR, null).refresh, false)
})

test('an exit in progress blocks further withdrawals and deregistration', () => {
  const actions = positionActions(position({ exitInProgress: true }), OPERATOR, OWNER)
  assert.deepEqual([actions.burn, actions.unbond, actions.deregister], [false, false, false])
})

test('claims follow the claimable amounts and the bond-owner rule', () => {
  assert.equal(positionActions(position(), OPERATOR, OWNER).claimAll, false)

  const claimable = position({ claimableTicket: 3n, claimableBond: 4n })
  assert.deepEqual(
    [positionActions(claimable, OPERATOR, OWNER).claimAll, positionActions(claimable, OPERATOR, OWNER).claimTickets],
    [true, false],
  )
  assert.deepEqual(
    [positionActions(claimable, OPERATOR, OTHER).claimAll, positionActions(claimable, OPERATOR, OTHER).claimTickets],
    [false, true],
  )
  assert.equal(positionActions(position({ claimableBond: 4n }), OPERATOR, OTHER).claimTickets, false)
})

test('only the proposed wallet can accept a bond-owner transfer', () => {
  const status = position({ pendingBondOwner: PROPOSED })
  assert.equal(positionActions(status, OPERATOR, PROPOSED).acceptOwner, true)
  assert.equal(positionActions(status, OPERATOR, OWNER).acceptOwner, false)
})

test('burns a whole number of tickets that the balance covers', () => {
  assert.equal(ticketBurnAmount('2', UNIT, 5n * UNIT), 2n * UNIT)
  assert.equal(ticketBurnAmount('6', UNIT, 5n * UNIT), null)
  assert.equal(ticketBurnAmount('0', UNIT, 5n * UNIT), null)
  assert.equal(ticketBurnAmount('1.5', UNIT, 5n * UNIT), null)
})

test('shows queued and claimable exits, and nothing for an operator without a position', () => {
  const props = { config, operator: OPERATOR, wallet: OWNER, busy: null, send: () => {}, errFor: () => null }
  const html = renderToStaticMarkup(
    <ManagePosition {...props} status={position({ pendingTicketExit: 2n * UNIT, claimableTicket: 2n * UNIT })} />,
  )
  assert.match(html, /Manage the position/)
  assert.match(html, /Claim to the bond owner/)
  assert.match(html, /2 USDS/)

  const empty = position({ bondOwner: null, registered: false, ciphernodeBond: 0n, ticketBalance: 0n, availableTickets: 0n })
  assert.equal(renderToStaticMarkup(<ManagePosition {...props} status={empty} />), '')
})
