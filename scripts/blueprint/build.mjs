// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Build the blueprint: flat architecture diagrams that open into each other, down to the code.
// Hand-drawn top levels (curated.mjs: every edge carries a checked code reference; a bad one stops
// the build) + generated deeper levels (generate.mjs: from the inventory, wiring and facts).
//
//   node scripts/blueprint/build.mjs           writes scripts/blueprint/out/interfold-blueprint.html
//   node scripts/blueprint/build.mjs --draft   allows hand-drawn edges without references
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadData, checkRef, strictRef, inTestCode, fileLines, OUT, ROOT } from './lib/data.mjs'
import { wiring as readWiring } from './lib/wiring.mjs'
import { generate, actorDiagrams } from './generate.mjs'
import { CURATED } from './curated.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const draft = process.argv.includes('--draft')
const { inv, units } = loadData()
const raw = readWiring(inv)

// ---- code store ----------------------------------------------------------------------------
const code = { ...raw.code }
const rawKey = (r) => `${r.path}@${r.line}@${r.token}`
function windowFor(ref, span = 9) {
  if (!ref) return null
  const L = fileLines(ref.path)
  if (!L) return null
  const start = Math.max(1, ref.line - span)
  const end = Math.min(L.length, ref.line + span)
  const key = `${ref.path}:${start}`
  if (!code[key]) code[key] = { path: ref.path, start, lines: L.slice(start - 1, end), more: 0 }
  return key
}
const codeFor = {
  handler: (ref) => (ref ? raw.handlerCode[rawKey(ref)] || null : null),
  contract: (ref) => (ref ? raw.contractCode[rawKey(ref)] || null : null),
  circuit: (id) => raw.circuitCode[id] || null,
}
// Wiring references are strings; check them like every other reference.
const asObj = (s) => {
  const c = checkRef(s)
  if (c.status === 'bad') throw new Error(`wiring reference does not check: ${s}`)
  return { path: c.path, line: c.line, token: c.token }
}
const wiring = { ...raw, actors: {} }
for (const [crate, as] of Object.entries(raw.actors)) {
  wiring.actors[crate] = {}
  for (const [name, a] of Object.entries(as)) {
    wiring.actors[crate][name] = {
      ...a,
      fields: a.fields.map((f) => ({ ...f, ref: asObj(f.ref) })),
      sends: a.sends.map((s) => ({ ...s, ref: asObj(s.ref) })),
    }
  }
}

// ---- diagrams ------------------------------------------------------------------------------
const diagrams = { ...generate({ units, wiring, codeFor, windowFor }), ...actorDiagrams({ units, wiring, codeFor, windowFor }) }
let curatedRefs = 0
let missing = 0
const testRefs = []
for (const make of CURATED) {
  const d = make({ units, tag: inv.tag })
  for (const n of d.nodes) {
    if (n.ref) {
      const r = strictRef(n.ref, `${d.id} node ${n.id}`)
      n.code = windowFor(r)
      n.focus = r.line
      delete n.ref
      curatedRefs++
    }
    if (n.drill && !n.drill.includes(':') && !CURATED.some((m) => m.id === n.drill)) {
      if (!draft) throw new Error(`${d.id}: node ${n.id} opens ${n.drill}, which does not exist`)
      delete n.drill
    }
  }
  for (const e of d.edges) {
    if (!e.refs || !e.refs.length) {
      if (e.optional) continue
      missing++
      if (!draft) throw new Error(`${d.id}: edge ${e.from} -> ${e.to} (${e.label || ''}) has no code reference`)
      continue
    }
    e.refs = e.refs.map((s) => {
      const r = strictRef(s, `${d.id} edge ${e.from} -> ${e.to}`)
      if (inTestCode(r.path, r.line)) testRefs.push(`${d.id} edge ${e.from} -> ${e.to}: ${r.path}:${r.line}`)
      curatedRefs++
      return { code: windowFor(r), focus: r.line }
    })
  }
  diagrams[d.id] = d
}
// Every edge gets a stable key, `from>to` (with `#2` for a second edge between the same boxes).
for (const d of Object.values(diagrams)) {
  const seen = {}
  for (const e of d.edges || []) {
    const k = `${e.from}>${e.to}`
    seen[k] = (seen[k] || 0) + 1
    e.key = seen[k] > 1 ? `${k}#${seen[k]}` : k
  }
}

// Every drill target must exist.
for (const d of Object.values(diagrams))
  for (const n of d.nodes || [])
    if (n.drill && !diagrams[n.drill]) {
      if (!draft) throw new Error(`${d.id}: ${n.id} opens missing diagram ${n.drill}`)
      delete n.drill
    }

const data = { root: 'system', root_dir: ROOT, tag: inv.tag, diagrams, code }
const css = readFileSync(join(HERE, 'blueprint.css'), 'utf8')
const js = readFileSync(join(HERE, 'renderer.js'), 'utf8')
const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Interfold ${inv.tag} · blueprint</title>
<style>${css}</style>
</head>
<body data-theme="light">
<header id="bar">
  <div class="brand">Interfold <span>${inv.tag} · blueprint</span></div>
  <div id="crumbs"></div>
  <div class="search"><input id="q" type="search" placeholder="Find a box…  ( / )" autocomplete="off" spellcheck="false"><div id="results" hidden></div></div>
  <nav>
    <button id="up" title="Up one level (Esc)">Up</button>
    <button id="fit" title="Fit (0)">Fit</button>
    <button id="theme" title="Light or dark (T)">Theme</button>
  </nav>
</header>
<main id="stage">
  <div id="head"><div id="dtitle"></div><div id="dsub"></div></div>
  <svg id="canvas" preserveAspectRatio="xMidYMid meet" role="img" aria-label="Architecture diagram"></svg>
  <div id="tip" hidden></div>
  <aside id="code" hidden></aside>
  <div id="help">Boxes with ⧉ open the diagram inside · boxes with &lt;/&gt; and arrows open the code · Esc goes up · scroll to zoom, drag to pan</div>
</main>
<script>window.BLUEPRINT = ${JSON.stringify(data)};</script>
<script>${js}</script>
</body>
</html>
`
mkdirSync(OUT, { recursive: true })
const file = join(OUT, 'interfold-blueprint.html')
writeFileSync(file, html)
const counts = Object.values(diagrams).reduce((a, d) => ((a[d.id.split(':')[0]] = (a[d.id.split(':')[0]] || 0) + 1), a), {})
if (testRefs.length)
  console.warn(`blueprint: ${testRefs.length} hand-drawn edge reference(s) point into test code:\n  ${testRefs.join('\n  ')}`)
console.log(
  `blueprint: ${Object.keys(diagrams).length} diagrams ${JSON.stringify(counts)} · curated refs ${curatedRefs}${missing ? ` · ${missing} edges still without refs (draft)` : ''} · code excerpts ${Object.keys(code).length} · ${(html.length / 1024).toFixed(0)} KB`,
)
console.log(`Open ${relative(process.cwd(), file) || file} in a browser.`)
