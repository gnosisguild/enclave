// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export function normalizeCoefficient(coeff: string): string {
  if (coeff.startsWith('0x')) {
    return BigInt(coeff).toString()
  }
  return coeff.toString()
}

export function compareCoefficientsArrays(arr1: any[], arr2: any[]): boolean {
  if (arr1.length !== arr2.length) return false

  for (let i = 0; i < arr1.length; i++) {
    if (!arr1[i] || !arr2[i]) return false

    const coeff1 = arr1[i].coefficients
    const coeff2 = arr2[i].coefficients

    if (!coeff1 || !coeff2) return false
    if (coeff1.length !== coeff2.length) return false

    for (let k = 0; k < coeff1.length; k++) {
      const normalized1 = normalizeCoefficient(coeff1[k])
      const normalized2 = normalizeCoefficient(coeff2[k])

      if (normalized1 !== normalized2) return false
    }
  }

  return true
}
