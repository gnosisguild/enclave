// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Build the blueprint: flat architecture diagrams that open into each other, down to the code.
// Hand-drawn top levels (curated.mjs: every edge carries a checked code reference) + generated
// deeper levels (generate.mjs: from the inventory, wiring and facts). Code drift does not stop the
// build: it uses the new line of a moved reference, drops a broken one, and prints a warning.
//
//   node scripts/blueprint/build.mjs           writes scripts/blueprint/out/interfold-blueprint.html
//   node scripts/blueprint/build.mjs --draft   allows hand-drawn edges without references
import { mkdirSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { OUT } from './lib/data.mjs'
import { buildData, pageHtml } from './lib/page.mjs'

const { data, warnings, testRefs, curatedRefs, missing } = buildData({ draft: process.argv.includes('--draft') })
const html = pageHtml(data)
mkdirSync(OUT, { recursive: true })
const file = join(OUT, 'interfold-blueprint.html')
writeFileSync(file, html)
const counts = Object.values(data.diagrams).reduce((a, d) => ((a[d.id.split(':')[0]] = (a[d.id.split(':')[0]] || 0) + 1), a), {})
if (warnings.length)
  console.warn(
    `blueprint: ${warnings.length} warning(s). The page is built. node scripts/blueprint/check.mjs lists them all, and --fix rewrites moved lines:\n  ${warnings.slice(0, 20).join('\n  ')}${warnings.length > 20 ? `\n  … and ${warnings.length - 20} more` : ''}`,
  )
if (testRefs.length)
  console.warn(`blueprint: ${testRefs.length} hand-drawn edge reference(s) point into test code:\n  ${testRefs.join('\n  ')}`)
console.log(
  `blueprint: ${Object.keys(data.diagrams).length} diagrams ${JSON.stringify(counts)} · curated refs ${curatedRefs}${missing ? ` · ${missing} edges still without refs (draft)` : ''} · code excerpts ${Object.keys(data.code).length} · ${(html.length / 1024).toFixed(0)} KB`,
)
console.log(`Open ${relative(process.cwd(), file) || file} in a browser.`)
