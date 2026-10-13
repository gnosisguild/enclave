// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Internal wiring and code excerpts, read from the snapshot for the blueprint's deeper levels:
// - each production actor's struct, its fields that hold another actor (`Addr<X>`, `Recipient<M>`),
// - the direct sends `self.<field>.do_send|try_send|send(` inside that actor's impl blocks,
// - code excerpts: handler impl blocks, actor structs, contract functions, circuit entry points.
// Everything is found by pattern and brace matching, so it needs no checking beyond the build.
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs'
import { join, relative } from 'node:path'
import { ROOT } from './inventory.mjs'

const rel = (p) => relative(ROOT, p)
function walk(dir, ext, out = []) {
  if (!existsSync(dir)) return out
  for (const name of readdirSync(dir)) {
    if (name === 'target' || name === 'node_modules' || name.startsWith('.')) continue
    const p = join(dir, name)
    if (statSync(p).isDirectory()) walk(p, ext, out)
    else if (ext.some((e) => name.endsWith(e))) out.push(p)
  }
  return out
}

// Count braces per line, outside strings, chars and comments. Returns one depth delta per line.
function braceDeltas(lines) {
  let inStr = false
  let inBlock = false
  return lines.map((line) => {
    let d = 0
    for (let i = 0; i < line.length; i++) {
      const c = line[i]
      const n = line[i + 1]
      if (inBlock) {
        if (c === '*' && n === '/') {
          inBlock = false
          i++
        }
        continue
      }
      if (inStr) {
        if (c === '\\') i++
        else if (c === '"') inStr = false
        continue
      }
      if (c === '/' && n === '/') break
      if (c === '/' && n === '*') {
        inBlock = true
        i++
        continue
      }
      if (c === '"') {
        inStr = true
        continue
      }
      if (c === "'") {
        // A char literal ('x', '\n', '{'), or a lifetime ('a), which has no closing quote.
        if (line[i + 1] === '\\') {
          const close = line.indexOf("'", i + 2)
          if (close > 0) i = close
        } else if (line[i + 2] === "'") i += 2
        continue
      }
      if (c === '{') d++
      else if (c === '}') d--
    }
    return d
  })
}

// The block that opens at the first `{` at or after `start` (0-based): returns its last line index.
function blockEnd(lines, deltas, start) {
  let depth = 0
  let opened = false
  for (let i = start; i < lines.length; i++) {
    if (!opened && /\{/.test(lines[i].replace(/"(?:[^"\\]|\\.)*"/g, '""').replace(/\/\/.*$/, ''))) opened = true
    depth += deltas[i]
    if (opened && depth <= 0) return i
  }
  return lines.length - 1
}

// `impl … for Name {` and `impl Name {` blocks of one file, with their line ranges.
function implBlocks(lines, deltas) {
  const out = []
  for (let i = 0; i < lines.length; i++) {
    if (!/^\s*impl\b/.test(lines[i])) continue
    let header = ''
    let j = i
    for (; j < Math.min(lines.length, i + 8); j++) {
      header += ' ' + lines[j]
      if (lines[j].includes('{')) break
    }
    header = header.replace(/\{.*$/, '').replace(/\bwhere\b[\s\S]*$/, '')
    let name = null
    const forM = header.match(/\bfor\s+([A-Za-z0-9_]+)/)
    if (forM) name = forM[1]
    else {
      const own = header.match(/impl(?:<[^>]*(?:<[^>]*>[^>]*)*>)?\s+([A-Za-z0-9_]+)/)
      if (own) name = own[1]
    }
    if (!name) continue
    const end = blockEnd(lines, deltas, i)
    out.push({ name, start: i, end, header: header.trim() })
    i = Math.max(i, j)
  }
  return out
}

const CAP = { handler: 80, struct: 50, fn: 80, circuit: 170 }
function excerpt(path, lines, start, end, cap) {
  const last = Math.min(end, start + cap - 1)
  return { path, start: start + 1, lines: lines.slice(start, last + 1), more: end - last }
}

// Field types that point at another actor.
// The generic argument that follows `prefix<` in `t`, with nested angle brackets kept whole.
function genericArg(t, prefix) {
  const at = t.indexOf(prefix + '<')
  if (at < 0) return null
  let depth = 0
  for (let i = at + prefix.length; i < t.length; i++) {
    if (t[i] === '<') depth++
    else if (t[i] === '>' && --depth === 0) return t.slice(at + prefix.length + 1, i)
  }
  return null
}

function fieldTarget(type) {
  const t = type.replace(/\s+/g, '')
  if (/^(?:Option<)?Addr</.test(t)) {
    const arg = genericArg(t, 'Addr')
    return arg ? { kind: 'addr', target: arg.replace(/<.*$/, '') } : null
  }
  if (/^(?:Option<)?Recipient</.test(t)) {
    const arg = genericArg(t, 'Recipient')
    return arg ? { kind: 'recipient', target: arg } : null
  }
  return null
}

export function wiring(inv) {
  const code = {}
  const keyOf = (ex) => `${ex.path}:${ex.start}`
  const put = (ex) => {
    const k = keyOf(ex)
    code[k] = ex
    return k
  }
  const actors = {}
  const handlerCode = {}

  for (const c of inv.crates) {
    const prod = new Set(c.actors.filter((a) => !a.test).map((a) => a.name))
    const files = walk(join(ROOT, c.path, 'src'), ['.rs'])
    const structs = {}
    const blocksByFile = []
    for (const f of files) {
      const lines = readFileSync(f, 'utf8').split('\n')
      const deltas = braceDeltas(lines)
      const path = rel(f)
      blocksByFile.push({ path, lines, blocks: implBlocks(lines, deltas) })
      lines.forEach((line, i) => {
        const m = line.match(/^\s*(?:pub(?:\([a-z]+\))?\s+)?struct\s+([A-Za-z0-9_]+)\b/)
        if (!m || !prod.has(m[1]) || structs[m[1]]) return
        if (/;\s*$/.test(line)) {
          structs[m[1]] = { ref: `${path}@${i + 1}@struct ${m[1]}`, code: put(excerpt(path, lines, i, i, 1)), fields: [] }
          return
        }
        const end = blockEnd(lines, deltas, i)
        const fields = []
        for (let k = i + 1; k < end; k++) {
          const fm = lines[k].match(/^\s*(?:pub(?:\([a-z]+\))?\s+)?([a-z_][a-z0-9_]*)\s*:\s*(.+?),?\s*$/)
          if (!fm) continue
          const t = fieldTarget(fm[2])
          if (t) fields.push({ name: fm[1], ...t, ref: `${path}@${k + 1}@${fm[1]}` })
        }
        structs[m[1]] = { ref: `${path}@${i + 1}@struct ${m[1]}`, code: put(excerpt(path, lines, i, end, CAP.struct)), fields }
      })
    }
    for (const name of prod) {
      const s = structs[name] || { ref: null, code: null, fields: [] }
      const sends = []
      for (const { path, lines, blocks } of blocksByFile) {
        for (const b of blocks.filter((b) => b.name === name)) {
          for (let k = b.start; k <= b.end; k++) {
            const m = lines[k].match(/self\.([a-z_][a-z0-9_]*)\.(do_send|try_send|send)\(/)
            if (!m) continue
            const field = s.fields.find((f) => f.name === m[1])
            if (!field) continue
            sends.push({ field: m[1], method: m[2], kind: field.kind, target: field.target, ref: `${path}@${k + 1}@self.${m[1]}.${m[2]}(` })
          }
        }
      }
      ;(actors[c.id] ||= {})[name] = { ...s, sends }
    }
    // Handler code: the impl block that each production handler line opens.
    for (const h of c.handlers.filter((h) => !h.test)) {
      const [path, line] = h.ref.split('@')
      const entry = blocksByFile.find((x) => x.path === path)
      if (!entry) continue
      const b = entry.blocks.find((b) => b.start === Number(line) - 1)
      if (b) handlerCode[h.ref] = put(excerpt(path, entry.lines, b.start, b.end, CAP.handler))
    }
  }

  // Contract functions: from `function name(` to the end of its body.
  const contractCode = {}
  for (const c of inv.contracts) {
    for (const fn of c.functions) {
      const [path, line] = fn.ref.split('@')
      const lines = readFileSync(join(ROOT, path), 'utf8').split('\n')
      const deltas = braceDeltas(lines)
      const i = Number(line) - 1
      contractCode[fn.ref] = put(excerpt(path, lines, i, blockEnd(lines, deltas, i), CAP.fn))
    }
  }

  // Circuit entry points: the whole main.nr, capped.
  const circuitCode = {}
  for (const c of inv.circuits) {
    if (!c.main) continue
    const [path] = c.main.split('@')
    const lines = readFileSync(join(ROOT, path), 'utf8').split('\n')
    circuitCode[c.id] = put(excerpt(path, lines, 0, lines.length - 1, CAP.circuit))
  }

  return { actors, handlerCode, contractCode, circuitCode, code }
}

if (process.argv[1] && process.argv[1].endsWith('wiring.mjs')) {
  const { inventory } = await import('./inventory.mjs')
  const w = wiring(inventory())
  let fields = 0
  let sends = 0
  for (const [crate, as] of Object.entries(w.actors)) {
    for (const [name, a] of Object.entries(as)) {
      fields += a.fields.length
      sends += a.sends.length
      if (a.sends.length || a.fields.length)
        console.log(
          `${crate.padEnd(22)} ${name.padEnd(32)} fields ${a.fields.map((f) => f.name + '→' + f.target).join(', ')} | sends ${a.sends.map((s) => s.field + '.' + s.method).join(', ')}`,
        )
      if (!a.ref) console.log(`  ! no struct found for ${crate} ${name}`)
    }
  }
  const lines = Object.values(w.code).reduce((n, ex) => n + ex.lines.length, 0)
  console.log(
    `fields ${fields} · sends ${sends} · handler excerpts ${Object.keys(w.handlerCode).length} · contract fns ${Object.keys(w.contractCode).length} · circuits ${Object.keys(w.circuitCode).length} · code lines ${lines} · ~${(JSON.stringify(w.code).length / 1024).toFixed(0)} KB`,
  )
}
