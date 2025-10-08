import { describe, expect, it } from 'vitest'

import { getTreeData } from '../src/token'

describe('Token data fetching', () => {
  const serverUrl = 'http://localhost:4000'
  it('should fetch token data from the CRISP server', async () => {
    await getTreeData(serverUrl, 0)
  })
})
