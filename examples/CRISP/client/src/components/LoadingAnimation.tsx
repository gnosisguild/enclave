// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { useEffect, useState } from 'react'
import CircularTile from '@/components/CircularTile'

const LoadingAnimation = ({ className, isLoading }: { className?: string; isLoading: boolean }) => {
  const [rotations, setRotations] = useState([0, 0, 0, 0])

  // Determine if the screen width is medium or larger

  const getRandRotation = () => {
    const rand_index = Math.floor(Math.random() * 4)
    const rotation = [0, 90, 180, 270][rand_index]
    return rotation
  }

  useEffect(() => {
    const interval = setInterval(() => {
      if (isLoading) {
        setRotations([getRandRotation(), getRandRotation(), getRandRotation(), getRandRotation()])
      }
    }, 500)

    if (!isLoading) {
      clearInterval(interval)
    }

    return () => clearInterval(interval)
  }, [rotations, isLoading])

  return (
    <div className={`flex h-full items-center justify-center ${className}`}>
      <div className={`grid h-10 w-10 grid-cols-2 gap-1`}>
        {rotations.map((rotation, i) => {
          return <CircularTile key={i} className='!fill-slate-600 duration-500 ease-in-out' rotation={rotation} />
        })}
      </div>
    </div>
  )
}

export default LoadingAnimation
