// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export const handleGenericError = (functionName: string, error: Error) => {
  console.error(`[${functionName}] - ${error.message}`)
  // throw new Error(`[${functionName}] -  ${error.message}`)
}
