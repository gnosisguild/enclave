// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { execa } from 'execa'
import { readFile, writeFile, rm } from 'fs/promises'
import { resolve } from 'path'
import replaceInFile from 'replace-in-file'

try {
  const dist = resolve(process.cwd(), 'dist')

  await execa('wasm-pack', ['build', '../../crates/zk-inputs-wasm', '--target=web', `--out-dir=${dist}`, '--no-pack', '--out-name=index'])

  // Convert WASM binary to base64 for bundler compatibility.
  const wasmBinary = await readFile(`${dist}/index_bg.wasm`)
  const base64Src = `export default '${wasmBinary.toString('base64')}';\n`

  // Parallel cleanup and JS modification to prevent Next.js and other bundlers static analysis issues.
  await Promise.all([
    rm(`${dist}/index_bg.wasm`, { force: true }),
    rm(`${dist}/index_bg.wasm.d.ts`, { force: true }),
    rm(`${dist}/.gitignore`, { force: true }),
    replaceInFile({
      files: `${dist}/index.js`,
      from: /module_or_path\s*=\s*new URL\(['"]index_bg\.wasm['"],\s*import\.meta\.url\);\s*/g,
      to: '/* wasm URL disabled */\n',
    }),
    writeFile(`${dist}/index_base64.js`, base64Src),
  ])
} catch (error) {
  console.error(error)
  process.exit(1)
}
