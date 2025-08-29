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
    <div
      className={`w-fit rounded-lg border-2 border-slate-600/20 bg-white p-2 py-1 text-center font-bold uppercase text-slate-800/50 shadow-md`}
    >
      {totalVotes} votes
    </div>
  )
}

export default VotesBadge
