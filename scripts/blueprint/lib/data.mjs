// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// The one data loader for the blueprint: the inventory read from the code, plus the plain-English
// facts in facts/, with every reference checked against the code.
import { readFileSync, readdirSync, existsSync } from 'node:fs'
import { join, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { inventory, ROOT } from './inventory.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
export const BLUEPRINT = resolve(HERE, '..')
// Where the build writes: out/, or BLUEPRINT_OUT when check.mjs tries the build.
export const OUT = process.env.BLUEPRINT_OUT ? resolve(process.env.BLUEPRINT_OUT) : join(BLUEPRINT, 'out')
export { ROOT }

// ---- reference checking -------------------------------------------------------------------
const fileCache = new Map()
export function fileLines(path) {
  if (!fileCache.has(path)) {
    const abs = join(ROOT, path)
    fileCache.set(path, existsSync(abs) ? readFileSync(abs, 'utf8').split('\n') : null)
  }
  return fileCache.get(path)
}

const WINDOW = 3
// A reference is `path@line@token`. It is `ok` when the token is within WINDOW lines of the line,
// `moved` when the token is elsewhere in the file (the line number is corrected), else `bad`.
export function checkRef(raw) {
  if (typeof raw !== 'string') return { status: 'bad', why: 'not a string', raw }
  const first = raw.indexOf('@')
  const second = raw.indexOf('@', first + 1)
  if (first < 0 || second < 0) return { status: 'bad', why: 'format', raw }
  const path = raw.slice(0, first).replace(/^\.?\//, '')
  const line = Number(raw.slice(first + 1, second))
  const token = raw.slice(second + 1).trim()
  const L = fileLines(path)
  if (!L) return { status: 'bad', why: 'no such file', raw, path }
  if (!token) return { status: 'bad', why: 'empty token', raw, path }
  if (/\.md$/.test(path)) return { status: 'bad', why: 'points at docs, not code', raw }
  if (Number.isFinite(line) && line >= 1) {
    for (let d = 0; d <= WINDOW; d++) {
      for (const l of [line - d, line + d])
        if (l >= 1 && l <= L.length && L[l - 1].includes(token)) return { status: 'ok', path, line: l, token }
    }
  }
  let best = -1
  L.forEach((text, i) => {
    if (text.includes(token) && (best < 0 || Math.abs(i + 1 - line) < Math.abs(best - line))) best = i + 1
  })
  if (best > 0) return { status: 'moved', path, line: best, token, was: line }
  return { status: 'bad', why: 'token not in file', raw, path }
}

// A reference into the harness docs (`agent/*.md`), for the reasons behind a design. Checked the
// same way as a code reference; kept apart so code facts never cite prose.
export function checkDocRef(raw) {
  if (typeof raw !== 'string' || !raw.startsWith('agent/') || !/\.md@/.test(raw))
    return { status: 'bad', why: 'not an agent/*.md reference', raw }
  const [path, lineStr, ...rest] = raw.split('@')
  const line = Number(lineStr)
  const token = rest.join('@').trim()
  const L = fileLines(path)
  if (!L) return { status: 'bad', why: 'no such file', raw }
  for (let d = 0; d <= WINDOW; d++)
    for (const l of [line - d, line + d])
      if (l >= 1 && l <= L.length && L[l - 1].includes(token)) return { status: 'ok', path, line: l, token }
  let best = -1
  L.forEach((text, i) => {
    if (text.includes(token) && (best < 0 || Math.abs(i + 1 - line) < Math.abs(best - line))) best = i + 1
  })
  if (best > 0) return { status: 'moved', path, line: best, token, was: line }
  return { status: 'bad', why: 'token not in that doc', raw }
}
// check.mjs sets BLUEPRINT_ACCEPT_MOVED to try the build before it fixes moved line numbers.
const acceptMoved = (c) => c.status === 'moved' && !!process.env.BLUEPRINT_ACCEPT_MOVED
export function strictDocRef(raw, where) {
  const c = checkDocRef(raw)
  if (c.status !== 'ok' && !acceptMoved(c)) throw new Error(`${where}: ${raw} -> ${c.why || 'moved to line ' + c.line}`)
  return { path: c.path, line: c.line, token: c.token }
}

// Whether a line is test code: a file whose path says test, or a line inside an inline
// `#[cfg(test)] mod name { … }` block or a `#[test]`-style function. Braces are counted per line,
// as in the inventory, which is enough for this codebase.
const testLines = new Map()
export function inTestCode(path, line) {
  if (/(^|\/)(tests?|test_[^/]*|[^/]*_tests?)(\.rs|\/)|\.(test|spec)\.[jt]sx?$|\.t\.sol$/.test(path)) return true
  if (!path.endsWith('.rs')) return false
  if (!testLines.has(path)) {
    const L = fileLines(path) || []
    const inTest = new Array(L.length).fill(false)
    for (let i = 0; i < L.length; i++) {
      const attr = /^\s*#\[cfg\(test\)\]/.test(L[i])
        ? /^\s*(pub(\(crate\))?\s+)?mod\s+\w+\s*\{/
        : /^\s*#\[(test|tokio::test|actix::test|actix_rt::test)\b/.test(L[i])
          ? /^\s*(pub(\(crate\))?\s+)?(async\s+)?fn\s/
          : null
      if (!attr) continue
      const j = [i + 1, i + 2, i + 3].find((n) => n < L.length && attr.test(L[n]))
      if (j === undefined) continue
      let depth = 0
      let opened = false
      for (let n = j; n < L.length; n++) {
        const code = L[n]
          .replace(/\/\/.*$/, '')
          .replace(/"(?:[^"\\]|\\.)*"/g, '""')
          .replace(/'.'/g, "''")
        for (const ch of code) {
          if (ch === '{') (depth++, (opened = true))
          else if (ch === '}') depth--
        }
        inTest[n] = true
        if (opened && depth <= 0) break
      }
    }
    testLines.set(path, inTest)
  }
  return !!testLines.get(path)[line - 1]
}

// For hand-written material (the curated diagrams): anything but `ok` stops the build.
export function strictRef(raw, where) {
  const c = checkRef(raw)
  if (c.status !== 'ok' && !acceptMoved(c)) throw new Error(`${where}: ${raw} -> ${c.status} ${c.why || 'line ' + c.line}`)
  return { path: c.path, line: c.line, token: c.token }
}

// ---- facts ----------------------------------------------------------------------------------
const str = (v) => (typeof v === 'string' ? v.trim() : '')

function loadFacts(report) {
  const dir = join(BLUEPRINT, 'facts')
  const out = []
  for (const f of readdirSync(dir)
    .filter((n) => /^\d\d-.*\.json$/.test(n))
    .sort()) {
    try {
      const j = JSON.parse(readFileSync(join(dir, f), 'utf8'))
      report.files[f] = { units: (j.units || []).length }
      for (const u of j.units || []) out.push({ ...u, _file: f })
    } catch (e) {
      report.files[f] = { error: String(e.message || e) }
    }
  }
  return out
}

// Replace every `ref` string in a facts unit by a checked reference; drop the bad ones.
function checkUnit(u, report) {
  const fix = (list) =>
    (Array.isArray(list) ? list : [])
      .map((item) => {
        if (!item || typeof item !== 'object') return null
        if (!item.ref) return { ...item, ref: null }
        const r = checkRef(item.ref)
        if (r.status === 'bad') {
          report.refs.bad++
          report.bad.push({ unit: u.id, file: u._file, ...r })
          return null
        }
        report.refs[r.status]++
        return { ...item, ref: { path: r.path, line: r.line, token: r.token } }
      })
      .filter(Boolean)
  return {
    one_line: str(u.one_line),
    explain: str(u.explain),
    analogy: str(u.analogy),
    actors: fix(u.actors),
    consumes: fix(u.consumes),
    emits: fix(u.emits),
    state: fix(u.state),
    key_types: fix(u.key_types),
    read_first: fix(u.read_first),
    rust_patterns: fix(u.rust_patterns),
    depends_on: Array.isArray(u.depends_on) ? u.depends_on.filter((d) => typeof d === 'string') : [],
    tests: str(u.tests),
    unverified: Array.isArray(u.unverified) ? u.unverified.filter((d) => typeof d === 'string') : [],
    source: u._file,
  }
}

// ---- merge ----------------------------------------------------------------------------------
export function loadData() {
  const report = { files: {}, refs: { ok: 0, moved: 0, bad: 0 }, bad: [], unmatched: [] }
  const inv = inventory()
  const units = {}
  const asRef = (s) => {
    const r = checkRef(s)
    return r.status === 'bad' ? null : { path: r.path, line: r.line, token: r.token }
  }
  for (const c of inv.crates) {
    units[c.id] = {
      id: c.id,
      kind: 'crate',
      layer: c.layer,
      path: c.path,
      loc: c.loc,
      files: c.files,
      tests: c.tests,
      deps: c.deps,
      actors: c.actors.map((a) => ({ name: a.name, test: a.test || undefined, ref: asRef(a.ref) })),
      handlers: c.handlers.map((h) => ({ actor: h.actor, msg: h.msg, test: h.test || undefined, ref: asRef(h.ref) })),
    }
  }
  for (const c of inv.contracts) {
    units[c.id] = {
      id: c.id,
      kind: 'contract',
      role: c.role,
      path: c.path,
      loc: c.loc,
      files: c.files.length,
      events: c.events.map((e) => ({ name: e.name, ref: asRef(e.ref) })),
      functions: c.functions.map((f) => ({ name: f.name, ref: asRef(f.ref) })),
    }
  }
  for (const c of inv.circuits)
    units[c.id] = {
      id: c.id,
      kind: 'circuit',
      phase: c.phase,
      path: c.path,
      loc: c.loc,
      files: c.files,
      main: c.main ? asRef(c.main) : null,
    }
  for (const c of inv.apps) units[c.id] = { id: c.id, kind: c.kind, app: true, path: c.path, loc: c.loc, files: c.files }

  // Reverse dependencies, so a crate can show who uses it.
  for (const u of Object.values(units)) if (u.deps) for (const d of u.deps) if (units[d]) (units[d].usedBy ||= []).push(u.id)

  // Match a facts unit by exact id, then by path, then by a normalized name when only one unit has
  // it (the `e3-config` crate and the `config` circuit share a normalized name).
  const norm = (s) =>
    String(s || '')
      .toLowerCase()
      .replace(/^e3-/, '')
      .replace(/[^a-z0-9]+/g, '')
  const byPath = new Map(Object.values(units).map((u) => [u.path.replace(/\/$/, ''), u]))
  const byNorm = new Map()
  for (const u of Object.values(units)) byNorm.set(norm(u.id), byNorm.has(norm(u.id)) ? null : u)
  for (const f of loadFacts(report)) {
    const target =
      units[f.id] ||
      byPath.get(
        String(f.path || '')
          .replace(/^\.?\//, '')
          .replace(/\/$/, ''),
      ) ||
      byNorm.get(norm(f.id))
    if (!target) {
      report.unmatched.push({ id: f.id, path: f.path, file: f._file })
      continue
    }
    target.facts = checkUnit(f, report)
  }
  report.units = Object.keys(units).length
  report.withFacts = Object.values(units).filter((u) => u.facts).length
  return { inv, units, report }
}
