import { memo } from 'react'
import CircularTile from './CircularTile';

const CircularTiles = ({ count = 1, className }: { count?: number; className?: string }) => {
  return (
    <>
      {[...Array(count)].map((_i, index) => {
        const rand_index = Math.floor(Math.random() * 4)
        const rotation = [0, 90, 180, 270][rand_index]
        return (
          <CircularTile
            key={index}
            className={className}
            rotation={rotation}
          />
        )
      })}
    </>
  )
}

export default memo(CircularTiles)
