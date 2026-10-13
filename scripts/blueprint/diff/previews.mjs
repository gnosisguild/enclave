// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Store the pictures of a pull request comment on the blueprint-previews branch: a comment shows
// an image only from a URL. The Blueprint Diff workflow runs it.
//
//   node scripts/blueprint/diff/previews.mjs <remote> <folder> <name> <file>...
//
// It replaces <folder> on the branch with <folder>/<name>/<file>..., commits and pushes. When
// another job pushed first, it fetches the branch again and retries. The first run starts the
// branch with a README.
import { execFileSync, spawnSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

const BRANCH = 'blueprint-previews'
const [remote, folder, name, ...files] = process.argv.slice(2)
if (!remote || !folder || !name || !files.length || /(^|\/)\.\.?(\/|$)/.test(`${folder}/${name}`)) {
  console.error('usage: node scripts/blueprint/diff/previews.mjs <remote> <folder> <name> <file>...')
  process.exit(2)
}
const work = mkdtempSync(join(tmpdir(), 'blueprint-previews-'))
const git = (...a) => execFileSync('git', a, { cwd: work, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
const tryGit = (...a) => spawnSync('git', a, { cwd: work, encoding: 'utf8' }).status === 0
git('init', '-q')
git('config', 'user.name', 'github-actions[bot]')
git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com')
let done = false
try {
  for (let attempt = 1; attempt <= 5 && !done; attempt++) {
    if (tryGit('fetch', '-q', '--depth', '1', remote, BRANCH)) git('checkout', '-q', '-f', '-B', BRANCH, 'FETCH_HEAD')
    else {
      git('checkout', '-q', '--orphan', BRANCH)
      tryGit('rm', '-q', '-r', '-f', '--cached', '.')
      writeFileSync(
        join(work, 'README.md'),
        'Pictures for the Blueprint Diff comments on pull requests, one folder per pull request.\nWritten by `.github/workflows/blueprint-diff.yml`. Nothing else uses this branch.\n',
      )
    }
    rmSync(join(work, folder), { recursive: true, force: true })
    mkdirSync(join(work, folder, name), { recursive: true })
    for (const f of files) copyFileSync(f, join(work, folder, name, basename(f)))
    git('add', '-A')
    // The same pictures again, as in a rerun: the branch has them already.
    if (!git('status', '--porcelain')) {
      done = true
      break
    }
    git('commit', '-q', '-m', `Pictures for ${folder} at ${name}`)
    done = tryGit('push', '-q', remote, `HEAD:refs/heads/${BRANCH}`)
    if (!done) spawnSync('sleep', [String(1 + Math.floor(Math.random() * 4))])
  }
} finally {
  rmSync(work, { recursive: true, force: true })
}
if (!done) {
  console.error(`could not push to ${BRANCH} after 5 tries`)
  process.exit(1)
}
console.log(`stored ${files.length} picture(s) in ${BRANCH}:${folder}/${name}`)
