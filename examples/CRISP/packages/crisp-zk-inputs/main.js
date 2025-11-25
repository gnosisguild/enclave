// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { initSync } from './dist/index.js'
import base64 from './dist/index_base64.js'

const binaryString = atob(base64)
const len = binaryString.length
const bytes = new Uint8Array(len)

for (let i = 0; i < len; i++) {
  bytes[i] = binaryString.charCodeAt(i)
}

initSync({ module: bytes })

export * from './dist/index.js'
