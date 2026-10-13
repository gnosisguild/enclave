// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Check the blueprint against the checked-out code.
//
//   node scripts/blueprint/check.mjs                 moved and broken references, crates not on the map
//   node scripts/blueprint/check.mjs --fix           also rewrite the moved line numbers
//   node scripts/blueprint/check.mjs --since <ref>   also list the hand-drawn arrows whose code changed
//
// The generated diagrams follow the code by themselves. The hand-drawn diagrams (curated.mjs, with
// the evidence in refs/) and the facts in facts/ cite code as `path@line@token`. A reference moved
// when its token is now on another line of the same file, and it broke when the token is gone.
// A moved line is found again by its token: for a common token the nearest copy can be the wrong
// one, so read each fixed line once.
import { execFileSync, spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { checkRef, loadData, ROOT } from './lib/data.mjs'
import { LAYERS } from './lib/inventory.mjs'
import { CURATED } from './curated.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const args = process.argv.slice(2)
const fix = args.includes('--fix')
const since = args.includes('--since') ? args[args.indexOf('--since') + 1] : null

// ---- hand-drawn references, with where each one is used ------------------------------------
const { inv, units, report } = loadData()
const refs = []
for (const make of CURATED) {
  const d = make({ units, tag: inv.tag })
  for (const n of d.nodes) if (n.ref) refs.push({ raw: n.ref, where: `${d.id} diagram, box ${n.id}` })
  for (const e of d.edges) for (const r of e.refs || []) refs.push({ raw: r, where: `${d.id} diagram, arrow ${e.from} → ${e.to}` })
}
for (const r of refs) Object.assign(r, checkRef(r.raw))
const moved = refs.filter((r) => r.status === 'moved')
const broken = refs.filter((r) => r.status === 'bad')

// ---- facts references (the build fixes moved ones on the fly and drops broken ones) ----------
const factFiles = readdirSync(join(HERE, 'facts')).filter((f) => f.endsWith('.json'))
const factsMoved = []
for (const f of factFiles)
  for (const u of JSON.parse(readFileSync(join(HERE, 'facts', f), 'utf8')).units || [])
    for (const list of Object.values(u))
      if (Array.isArray(list))
        for (const item of list) {
          if (!item || typeof item.ref !== 'string') continue
          const c = checkRef(item.ref)
          if (c.status === 'moved') factsMoved.push({ raw: item.ref, line: c.line })
        }

// ---- workspace crates that the inventory does not list ------------------------------------
const members = (readFileSync(join(ROOT, 'Cargo.toml'), 'utf8').match(/members\s*=\s*\[([\s\S]*?)\]/) || ['', ''])[1]
const mapped = new Set(Object.values(LAYERS).flat())
const unmapped = [...members.matchAll(/"crates\/([^"]+)"/g)].map((m) => m[1]).filter((d) => !mapped.has(d))

// ---- the build, with moved lines accepted, in a temporary folder ---------------------------
function tryBuild(env) {
  const out = mkdtempSync(join(tmpdir(), 'blueprint-check-'))
  const r = spawnSync(process.execPath, [join(HERE, 'build.mjs')], {
    encoding: 'utf8',
    env: { ...process.env, BLUEPRINT_OUT: out, ...env },
  })
  rmSync(out, { recursive: true, force: true })
  return {
    ok: r.status === 0,
    text: (r.status === 0 ? r.stdout.split('\n')[0] : (r.stderr.match(/^Error: .*$/m) || [r.stderr.trim().split('\n').pop()])[0]).trim(),
  }
}
const build = tryBuild({ BLUEPRINT_ACCEPT_MOVED: '1' })

// ---- report ---------------------------------------------------------------------------------
const say = (title, items) => {
  console.log(`\n${title}: ${items.length}`)
  for (const x of items.slice(0, 50)) console.log(`  - ${x}`)
  if (items.length > 50) console.log(`  - … and ${items.length - 50} more`)
}
console.log(
  `Blueprint check at ${inv.tag}: ${refs.length} hand-drawn references, ${report.refs.ok + report.refs.moved + report.refs.bad} facts references.`,
)
say(
  'Broken hand-drawn references (fix by hand)',
  broken.map((r) => `${r.where}: ${r.raw}`),
)
say(
  'Moved hand-drawn references (--fix rewrites the line)',
  moved.map((r) => `${r.where}: ${r.path} line ${r.was} → ${r.line} (${r.token})`),
)
say('Facts references that moved (--fix rewrites them) or broke (the build drops them)', [
  ...factsMoved.map((m) => `moved: ${m.raw} → line ${m.line}`),
  ...report.bad.map((b) => `broken: ${b.unit}: ${b.raw}`),
])
say('Crates that the blueprint does not show (add them to LAYERS in lib/inventory.mjs)', unmapped)
if (since) {
  const changed = new Set(execFileSync('git', ['diff', '--name-only', since], { cwd: ROOT, encoding: 'utf8' }).split('\n').filter(Boolean))
  const recheck = [...new Set(refs.filter((r) => changed.has(r.path)).map((r) => `${r.where}: ${r.path}`))]
  say(`Hand-drawn arrows whose code changed since ${since} (read them again)`, recheck)
}
console.log(`\nBuild with the moved lines accepted: ${build.ok ? 'passes' : 'FAILS'}. ${build.text}`)

// ---- fix --------------------------------------------------------------------------------------
if (fix && (moved.length || factsMoved.length)) {
  const files = [
    join(HERE, 'curated.mjs'),
    ...readdirSync(join(HERE, 'refs'))
      .filter((f) => f.endsWith('.json'))
      .map((f) => join(HERE, 'refs', f)),
    ...factFiles.map((f) => join(HERE, 'facts', f)),
  ]
  const text = new Map(files.map((f) => [f, readFileSync(f, 'utf8')]))
  const quoted = (s) => [`'${s.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`, JSON.stringify(s)]
  const byHand = []
  // One rewrite per reference: the same reference can appear more than once.
  const unique = new Map([...moved.map((m) => [m.raw, m.line]), ...factsMoved.map((m) => [m.raw, m.line])])
  for (const [raw, line] of unique) {
    const r = { raw, line }
    const fixed = r.raw.replace(/@\d+@/, `@${r.line}@`)
    let done = false
    for (const [f, t] of text)
      for (const [from, to] of quoted(r.raw).map((q, i) => [q, quoted(fixed)[i]]))
        if (text.get(f).includes(from)) {
          text.set(f, text.get(f).split(from).join(to))
          done = true
        }
    if (!done) byHand.push(r.raw)
  }
  for (const [f, t] of text) if (t !== readFileSync(f, 'utf8')) writeFileSync(f, t)
  console.log(`\nRewrote ${unique.size - byHand.length} moved reference(s).`)
  if (byHand.length) console.log(`Fix these by hand (built from a template string):\n  ${byHand.join('\n  ')}`)
  const after = tryBuild({})
  console.log(`Build after the fix: ${after.ok ? 'passes' : 'FAILS'}. ${after.text}`)
  process.exit(after.ok && !broken.length ? 0 : 1)
}
process.exit(broken.length || !build.ok ? 1 : 0)
