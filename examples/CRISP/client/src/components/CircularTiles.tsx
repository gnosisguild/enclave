// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { memo, useEffect, useState } from 'react'
import CircularTile from './CircularTile'

const generateRotations = (count: number) => [...Array(count)].map(() => [0, 90, 180, 270][Math.floor(Math.random() * 4)])

const CircularTiles = ({ count = 1, className }: { count?: number; className?: string }) => {
  const [rotations, setRotations] = useState(() => generateRotations(count))

  useEffect(() => {
    setRotations(generateRotations(count))
  }, [count])

  return (
    <>
      {rotations.map((rotation, index) => (
        <CircularTile key={index} className={className} rotation={rotation} />
      ))}
    </>
  )
}

export default memo(CircularTiles)
