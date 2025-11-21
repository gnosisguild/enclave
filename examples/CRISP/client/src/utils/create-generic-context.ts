// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'

export const createGenericContext = <T>() => {
  // Create a context with a generic parameter or undefined
  const genericContext = React.createContext<T | undefined>(undefined)

  // Check if the value provided to the context is defined or throw an error
  const useGenericContext = () => {
    const contextIsDefined = React.useContext(genericContext)
    if (!contextIsDefined) {
      throw new Error('useGenericContext must be used within a Provider')
    }
    return contextIsDefined
  }

  return [useGenericContext, genericContext.Provider] as const
}
