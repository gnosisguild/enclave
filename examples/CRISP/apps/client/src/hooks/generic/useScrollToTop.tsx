// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// hooks/useScrollToTop.ts
import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'

const useScrollToTop = () => {
  const location = useLocation()
  useEffect(() => {
    const scrollContainer = window
    if (scrollContainer) {
      scrollContainer.scrollTo(0, 0)
    }
  }, [location.pathname])
}

export default useScrollToTop
