// SPDX-License-Identifier: LGPL-3.0-only

import assert from 'node:assert/strict'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import test from 'node:test'
import requiredArtifacts from '../../crates/zk/prover/required-artifacts.json'
import { RELEASE_REQUIRED_PAIRS } from './artifacts'
import { checkVerifierPairs, VerifierGenerator } from './verifiers'

// `check:verifiers` must fail when a requested circuit is not discovered, instead of checking less.
test('verifier generation rejects a requested circuit it cannot find', async (t) => {
  const root = fixture(t)
  const generator = new VerifierGenerator(root, { dryRun: true, circuits: ['dkg_aggregator', 'no_such_circuit'] })
  await assert.rejects(generator.generate(), /Cannot find requested circuits: no_such_circuit/)
})

test('verifier generation cannot select a preserved disabled circuit', async (t) => {
  const root = fixture(t)
  writeFileSync(join(root, 'circuits/bin/recursive_aggregation/dkg_aggregator/DISABLED'), 'Historical fixture circuit.\n')
  const generator = new VerifierGenerator(root, { dryRun: true, circuits: ['dkg_aggregator'] })
  await assert.rejects(generator.generate(), /Cannot find requested circuits: dkg_aggregator/)
})

// Exercise the existing public generation boundary with synthetic bb output, never real keys/proofs.
function fixture(t: { after: (fn: () => void) => void }): string {
  const root = mkdtempSync(join(tmpdir(), 'interfold-verifier-transform-'))
  const oldPath = process.env.PATH
  t.after(() => {
    process.env.PATH = oldPath
    rmSync(root, { recursive: true, force: true })
  })
  const write = (path: string, text: string, mode = 0o644) => {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), text, { mode })
  }
  write('bin/nargo', '#!/bin/sh\necho fixture\n', 0o755)
  write(
    'bin/bb',
    `#!/usr/bin/env node
const fs = require('node:fs')
const args = process.argv.slice(2)
if (args[0] === '--version') console.log('fixture')
else if (args[0] === 'write_solidity_verifier') fs.copyFileSync(${JSON.stringify(join(root, 'raw.sol'))}, args[args.indexOf('-o') + 1])
else process.exit(1)
`,
    0o755,
  )
  process.env.PATH = `${root}/bin:${resolve(__dirname, '../../node_modules/.bin')}:${oldPath}`
  write('circuits/bin/.active-preset.json', '{"preset":"insecure-64","committee":"minimum"}')
  write('dist/circuits/insecure-64/minimum/.build-stamp.json', '{"preset":"insecure-64","committee":"minimum"}')
  write('circuits/bin/recursive_aggregation/dkg_aggregator/Nargo.toml', '[package]\nname = "dkg_aggregator"\n')
  write('circuits/bin/recursive_aggregation/dkg_aggregator/target/dkg_aggregator.json', '{}')
  write('circuits/bin/recursive_aggregation/dkg_aggregator/target/dkg_aggregator.vk', 'synthetic fixture, not a key')
  const packageManager = JSON.parse(readFileSync(resolve(__dirname, '../../package.json'), 'utf8')).packageManager
  write('package.json', JSON.stringify({ private: true, packageManager }))
  write('packages/interfold-contracts/package.json', JSON.stringify({ private: true }))
  mkdirSync(join(root, 'packages/interfold-contracts'), { recursive: true })
  symlinkSync(resolve(__dirname, '../../node_modules'), join(root, 'node_modules'))
  copyFileSync(
    resolve(__dirname, '../../packages/interfold-contracts/.prettierrc.yml'),
    join(root, 'packages/interfold-contracts/.prettierrc.yml'),
  )
  write(
    'raw.sol',
    `// SPDX-License-Identifier: Apache-2.0
pragma solidity >=0.8.27;
library Errors {
    error ConsistencyCheckFailed();
}
library FrLib {
    bytes4 internal constant FRLIB_MODEXP_FAILED_SELECTOR = 0xf8d61709;
}
abstract contract BaseZKHonkVerifier {
    constructor(uint256 _N, uint256 _logN, uint256 _vkHash, uint256 _numPublicInputs) {
    }
    function verify() public view {}
}
contract HonkVerifier is BaseZKHonkVerifier(N, LOG_N, VK_HASH, NUMBER_OF_PUBLIC_INPUTS) {
}
`,
  )
  return root
}

test('verifier write/check retain selector correction, VK validation, naming and formatted output', async (t) => {
  const root = fixture(t)
  const options = { compile: false, noCleanTargets: true, circuits: ['dkg_aggregator'] }
  await new VerifierGenerator(root, options).generate()
  const path = join(root, 'packages/interfold-contracts/contracts/verifiers/bfv/honk/DkgAggregatorVerifier.sol')
  const output = readFileSync(path, 'utf8')
  assert.match(output, /SPDX-License-Identifier: LGPL-3.0-only/)
  assert.match(output, /contract DkgAggregatorVerifier/)
  assert.match(output, /FRLIB_MODEXP_FAILED_SELECTOR = 0x1f7ec5f0/)
  assert.doesNotMatch(output, /0xf8d61709/)
  assert.match(output, /validateVerificationKey\(HonkVerificationKey.loadVerificationKey\(\)\)/)
  for (const parameter of ['$N', '$LOG_N', '$NUM_PUBLIC_INPUTS']) assert.ok(output.includes(parameter))
  await new VerifierGenerator(root, { ...options, check: true }).generate()
  assert.equal(readFileSync(path, 'utf8'), output)
})

test('all-pair preflight checks inactive committed verifiers without changing selectors or archived keys', async (t) => {
  const root = fixture(t)
  const bin = join(root, 'circuits/bin/recursive_aggregation/decryption_aggregator')
  mkdirSync(join(bin, 'target'), { recursive: true })
  writeFileSync(join(bin, 'Nargo.toml'), '[package]\nname = "decryption_aggregator"\n')
  writeFileSync(join(bin, 'target/decryption_aggregator.json'), '{}')
  writeFileSync(join(bin, 'target/decryption_aggregator.vk'), 'synthetic fixture, not a key')
  await new VerifierGenerator(root, {
    compile: false,
    noCleanTargets: true,
    circuits: ['dkg_aggregator', 'decryption_aggregator'],
  }).generate()
  const honk = join(root, 'packages/interfold-contracts/contracts/verifiers/bfv/honk')
  const hash = (preset: string, committee: string) => `fixture:${preset}/${committee}`
  for (const [preset, committee] of RELEASE_REQUIRED_PAIRS) {
    const pair = join(root, 'dist/circuits', preset, committee)
    for (const artifact of requiredArtifacts) {
      const file = join(pair, artifact)
      mkdirSync(dirname(file), { recursive: true })
      writeFileSync(file, 'synthetic fixture, not an artifact')
    }
    writeFileSync(join(pair, '.build-stamp.json'), JSON.stringify({ preset, committee, sourceHash: hash(preset, committee) }))
    if (preset === 'insecure-64' && committee === 'minimum') continue
    const dest = join(honk, preset, committee)
    mkdirSync(dest, { recursive: true })
    for (const contract of ['DkgAggregatorVerifier', 'DecryptionAggregatorVerifier']) {
      copyFileSync(join(honk, `${contract}.sol`), join(dest, `${contract}.sol`))
    }
  }
  const activeFile = join(root, 'circuits/bin/.active-preset.json')
  const active = readFileSync(activeFile, 'utf8')
  const inactive = join(honk, 'secure-8192/minimum/DkgAggregatorVerifier.sol')
  const original = readFileSync(inactive, 'utf8')
  await checkVerifierPairs(root, RELEASE_REQUIRED_PAIRS, hash)
  assert.equal(readFileSync(activeFile, 'utf8'), active)

  writeFileSync(inactive, `${original}\n// stale inactive verifier\n`)
  await assert.rejects(checkVerifierPairs(root, RELEASE_REQUIRED_PAIRS, hash), /committed Solidity verifier\(s\) differ/)
  assert.equal(readFileSync(inactive, 'utf8'), `${original}\n// stale inactive verifier\n`)
  assert.equal(readFileSync(activeFile, 'utf8'), active)

  await assert.rejects(
    checkVerifierPairs(root, RELEASE_REQUIRED_PAIRS, () => 'wrong source'),
    /Stale circuit artifacts/,
  )
  const [preset, committee] = RELEASE_REQUIRED_PAIRS.at(-1)!
  const missing = join(root, 'dist/circuits', preset, committee, 'evm/recursive_aggregation/dkg_aggregator/dkg_aggregator.vk')
  rmSync(missing)
  await assert.rejects(checkVerifierPairs(root, RELEASE_REQUIRED_PAIRS, hash), /Incomplete circuit artifacts/)
  assert.equal(existsSync(missing), false, 'checks must not regenerate a missing archived key')
})
