// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Store the pictures and the page of a pull request on the blueprint-previews branch, or remove
// them. A comment shows an image only from a URL, and GitHub Pages serves the branch. The Blueprint
// Diff workflow runs it.
//
//   node scripts/blueprint/diff/previews.mjs <remote> <folder> <name> <file>...
//   node scripts/blueprint/diff/previews.mjs <remote> <folder> --remove
//
// The first form replaces <folder> on the branch with <folder>/<name>/<file>...; the second deletes
// <folder>. The branch is always one commit with no parent, so it keeps no history and stays the
// size of the open pull requests. Each change pushes a new such commit, with a lease on the commit
// it read: when another job pushed first, it fetches the branch again and retries. The first store
// starts the branch with a README. The last line of the output starts with `changed:` when the
// branch changed, else with `unchanged:`.
import { execFileSync, spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

const BRANCH = 'blueprint-previews'
const [remote, folder, name, ...files] = process.argv.slice(2)
const remove = name === '--remove'
if (!remote || !folder || !name || (!remove && !files.length) || /(^|\/)\.\.?(\/|$)/.test(`${folder}/${remove ? '' : name}`)) {
  console.error(
    'usage: node scripts/blueprint/diff/previews.mjs <remote> <folder> <name> <file>...\n       node scripts/blueprint/diff/previews.mjs <remote> <folder> --remove',
  )
  process.exit(2)
}
const work = mkdtempSync(join(tmpdir(), 'blueprint-previews-'))
const git = (...a) => execFileSync('git', a, { cwd: work, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
const tryGit = (...a) => spawnSync('git', a, { cwd: work, encoding: 'utf8' }).status === 0
git('init', '-q')
git('config', 'user.name', 'github-actions[bot]')
git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com')
let result = null
try {
  for (let attempt = 1; attempt <= 8 && !result; attempt++) {
    let tip = ''
    if (tryGit('fetch', '-q', '--depth', '1', remote, BRANCH)) {
      tip = git('rev-parse', 'FETCH_HEAD')
      git('checkout', '-q', '-f', '-B', BRANCH, 'FETCH_HEAD')
    } else if (remove) {
      result = `unchanged: ${BRANCH} does not exist`
      break
    } else {
      git('checkout', '-q', '--orphan', BRANCH)
      tryGit('rm', '-q', '-r', '-f', '--cached', '.')
      writeFileSync(
        join(work, 'README.md'),
        'Pictures and pages for the Blueprint Diff comments on pull requests, one folder per pull request.\nWritten by `.github/workflows/blueprint-diff.yml`, which also publishes this branch on GitHub Pages.\nNothing else uses it.\n',
      )
    }
    if (remove && !existsSync(join(work, folder))) {
      result = `unchanged: ${BRANCH} has no ${folder}`
      break
    }
    rmSync(join(work, folder), { recursive: true, force: true })
    if (!remove) {
      mkdirSync(join(work, folder, name), { recursive: true })
      for (const f of files) copyFileSync(f, join(work, folder, name, basename(f)))
    }
    git('add', '-A')
    // The same files again, as in a rerun: the branch has them already.
    if (!git('status', '--porcelain')) {
      result = `unchanged: ${BRANCH} has ${folder}/${name} already`
      break
    }
    const commit = git('commit-tree', git('write-tree'), '-m', remove ? `Remove ${folder}` : `Previews for ${folder} at ${name}`)
    if (tryGit('push', '-q', `--force-with-lease=refs/heads/${BRANCH}:${tip}`, remote, `${commit}:refs/heads/${BRANCH}`))
      result = remove ? `changed: removed ${BRANCH}:${folder}` : `changed: stored ${files.length} file(s) in ${BRANCH}:${folder}/${name}`
    else spawnSync('sleep', [String(1 + Math.floor(Math.random() * 4))])
  }
} finally {
  rmSync(work, { recursive: true, force: true })
}
if (!result) {
  console.error(`could not push to ${BRANCH} after 8 tries`)
  process.exit(1)
}
console.log(result)
