// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'

type VotesBadgeProps = {
  totalVotes: number
}

const VotesBadge: React.FC<VotesBadgeProps> = ({ totalVotes }) => {
  return (
    <span className='tag'>
      {totalVotes} {totalVotes === 1 ? 'vote' : 'votes'}
    </span>
  )
}

export default VotesBadge
