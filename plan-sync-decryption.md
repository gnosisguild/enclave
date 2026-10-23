# Plan: Synchronized TRBFV decryption on a fresh main-aligned branch

Status: review reopened for confirmed M1 authentication regression and M2 exclusion-convergence
liveness gap. The published review branch is not ready for merge. Secure/minimum C6/C7 leaves are
verified, but the full secure matrix remains unqualified. Date: 2026-10-23. Reference
implementation: `feat/share-prf-bit` @ `27041b947` (kept as read-only reference; built on the dead
`lbfv/chunking` lineage). Upstream dependency:
[fhe.rs v0.5.1](https://github.com/gnosisguild/fhe.rs/releases) — contains
[#286](https://github.com/gnosisguild/fhe.rs/pull/286) (synchronized TRBFV decryption) and
[#289](https://github.com/gnosisguild/fhe.rs/pull/289) (consolidated threshold decryption APIs,
breaking).

## Goal

Ship the synchronized decryptor (PRF-masked partial decryption, designated set of `threshold + 1`
parties, local Lagrange weighting, fresh local smudging noise) on a branch based on current
`origin/main`, **without** any lbfv/chunking lineage content.

## Current checkpoint

The latest instruction is to commit/push the cleanup for cryptographer review and stop further
memory/decomposition experiments. A reported memory-pressure crash/reboot cleared the approved
temporary log directory; earlier temporary qualification and benchmark paths in this plan are
historical references, not currently available raw evidence. Summary observations already recorded
here survive, as do the source and durable worktree artifacts. The 13-job run is unqualified.

### Reopened: M1 bundle authentication and M2 exclusion convergence

The user confirmed that the port removed the schema-8 full-bundle signatures and threshold-share
admission check. Restored previously omitted forged-bundle test cases fail on `96db00dbf`: both an
unsigned threshold bundle and an unauthenticated C4 bundle reserve a dealer slot. The earlier
"review milestone complete" assessment therefore did not establish these requirements and is
withdrawn.

The working fix restores dealer signatures over the full current C2/C3 and C4 envelopes, using
versioned signing domains, and verifies the finalized dealer before storing or forwarding peer
inputs. Validly signed incomplete dealer contributions remain attributable only to that dealer;
transport origin is not signed because receipt changes it. Focused before/after tests reproduce the
old failure and pass with the restored checks. Expanded tamper/admission, eight native-crate,
layout/wire rejection, and fresh real DKG/C6/C7 disk-restart checks now pass. The follow-up changes
remain review-branch work; the asymmetric M2 gate below is not complete.

On 2026-10-23 the user requested committing and pushing this follow-up as a review checkpoint, with
M2 end-to-end convergence, obsolete-fold retirement and the full release matrix explicitly open.
Normal commit/push hooks still apply; this does not authorize merge, deployment or release.

The user approved development schema **10**, gossip **7**, and sync **6**, keeping protocol
9/generation 2. Schema-9 branch stores and wire envelopes must be rejected, not reinterpreted. No
merge, deployment, migration execution or release is approved.

M2 remains open: the aggregator derives its set from a local rejected-party view, while ciphernodes
accept only their own confirmed-exclusion-derived set and reject peer wakeups as authority. The
earlier Small run injected the confirmed exclusion into every node; it did **not** establish
convergence when those views differ. An authenticated convergence path and asymmetric end-to-end
coverage must be established. Merely accepting a network `FreshDecryptionRound` is not a safe fix.
Do not treat the older happy-path/Small results below as closing M1 or M2.

Two further M2 regressions are reproduced and addressed in the working tree: a local C6 failure
previously changed `[6..15]` to `[7..16]` without a confirmed exclusion, and C6 accusations with a
forwarded proof were rejected, leaving peers without a local C6 verdict unable to reverify/vote. The
aggregator now retains usable shares in the same set until confirmation; confirmed exclusions alone
clear the old set and choose replacements. C6 failure accusations carry the authenticated proof for
independent verification, and round-specific cached results are reused only for the same proof
preimage. None of this grants a peer wakeup set-selection authority. Native regressions do not
substitute for the still-missing asymmetric convergence/recovery end-to-end scenario.

Compatibility fixture regeneration uses the approved schema/wire boundary: runtime and DHT layout
locks were regenerated for the restored signature fields, then passed normal comparison runs. No old
store was reset and no fixture refresh is being claimed as migration or semantic proof.

#### Current follow-up evidence and scoped next gate

- `m1-m2-native-regressions-final-2.log`: aggregator 94, data 53, events 138, keyshare 162, net 303,
  slashing 47, sync 53, prover 166 passed (two real-proof prover tests remain ignored in that native
  command). Runtime and DHT layout comparison runs passed separately.
- `m1-restart-minimum.json`: the schema-10 signed-bundle real DKG/C6/C7 and persisted-C6 disk
  restart passed in **222.45 seconds**, sampled peak **6,309.61 MiB**. Public-only proof and timing
  exports use new `m1-restart-minimum-*` paths; historical outputs are preserved.
- `m1-m2-slashing-integration.log`: the prepared `pnpm rust:test:slashing` suite passed all **12**
  tests with Anvil. These verify signing/attestation contracts, not asymmetric decryption recovery.
- `m2-asymmetric-window-boundary-2.log`: a new Small **H=14** workflow test produces authenticated
  C6 accusation/votes through the production transitions and confirms every online voter without a
  delayed committee member. Supplying the same messages to that member before expiry converges;
  at/after expiry the accusation is rejected and no local quorum starts. Each non-accuser requests
  independent C6 verification, but outcomes are controlled fixtures: this is not cryptographic,
  policy-writer, transport, restart, or end-to-end replacement coverage. The initial invocation used
  a short name with `--exact` and selected zero tests; only the corrected one-test run is evidence.
- `m2-slashing-delivery-native.log`: all **48** slashing native tests passed after adding that
  delivery case. Documentation-link validation passed with 1,542 relative links and zero errors.
- `m2-scope-c6-native.log`: all **48** slashing native tests passed again after restricting
  exact-preimage cache selection to C6, leaving main's C3 behavior unchanged per the scoped repair.

The current wire/history allowlist carries accusations/votes, not local quorum/exclusion events.
Replaying an accusation after its live admission window is not a completed-quorum recovery protocol.
No authenticated certificate dissemination/recovery path has been designed or added. The user
subsequently limited implementation to findings caused by the synchronized decryptor; pre-existing
main findings are team advisories, not a general slashing redesign requirement. Live expiry and
peer-wakeup restrictions remain unchanged.

The existing full-node benchmark disables its chain, so it has neither a configured accusation
vote-validity window nor the normal slashing-policy writer. The required asymmetric end-to-end
scenario needs governance/vote-window fixtures at those production boundaries, a delayed
continuing/newly designated member, actual local policy-derived exclusions, and compatible
replacement shares plus final plaintext. Direct all-node exclusion injection is not a substitute. M2
therefore remains open even though the local premature-set-change and forwarding fixes pass.

#### Main comparison and team advisories (2026-10-23)

Compared against local `origin/main` **`3561619771e8e9e32ad06dd60a7c8a042f4b15a6`** and branch base
`354af6bef3b1cdd8fa23c53093feede923125d55`. The compared accusation workflow, network event
allowlist, chain-parameter loader, and slashing-writer paths have no intervening changes between
those refs. This comparison did not fetch or change main.

**In scope for this port:**

- M1 is a regression: main has complete threshold/C4 bundle signatures and threshold dealer
  authentication, which the port removed. Restore those protections for the changed payloads.
- M2's exact-set dependency is new: main's shares are set-independent, so its aggregator can discard
  locally bad shares and use backups without every node recalculating. Synchronized shares require
  continuing/new decryptors to derive the same authorized replacement set. Keep the local
  failure/confirmed-exclusion separation and verify the actual asymmetric live-window path.
- Main did not forward C6 proofs. This port now relies on C6-confirmed exclusions for set changes,
  so supplying peers the authenticated C6 for independent voting is necessary integration work.
- Replaced synchronized C6 rounds introduce multiple preimages per dealer/type. Reuse a cached C6
  result only for the exact proof. Keep this repair C6-specific, rather than also changing main's
  recipient-specific C3 cache behavior.

**Advise the team; do not expand this repair:**

1. Main already rejects a missed accusation at/after expiry and has no completed-quorum
   dissemination/recovery certificate. Relevant owners are accusation admission/deadlines and the
   network's forwardable-event allowlist. A general historical-certificate/expiry design is separate
   work. Synchronized decryption increases sensitivity to incompatible local exclusion views, so
   retain this inherited limitation in qualification; do not claim arbitrary-delay convergence.
2. Main's received-proof cache is keyed by dealer/type, and its C3 accusation path can reuse a
   cached verdict without comparing the supplied proof hash. Static comparison identifies this
   pre-existing recipient/preimage limitation; it is not a new synchronized-decryptor finding or a
   separately qualified security fix here.

#### Additional published-commit review reconciliation (2026-10-23)

The supplied 251-file review targets `96db00dbf`, not the follow-up fixes described here. Its
positive cryptographic assessment is useful review evidence, not the required human protocol
approval or a claim that the full matrix has run. Reconcile its numbered findings as follows:

1. **Dealer admission:** addressed in the working tree by full-envelope signatures and finalized
   dealer checks before slot reservation, with restored red/green forgery coverage. Current source
   versions are schema **10**, gossip **7**, sync **6**, not the reviewed commit's 9/6/5. These
   fixes are not part of the reviewed commit `96db00dbf`.
2. **Stale non-canonical verifiers:** confirmed. Added source-bound archived-pair checking through
   `pnpm check:verifiers --all` and an automated release gate before circuit archiving/publication,
   with a hard release/deployment stop in `RELEASING.md`. The gate fails if any required pair lacks
   a current complete artifact set or its committed Honk wrapper differs from its own EVM VK. It
   never switches active selectors, compiles missing keys, or regenerates committed verifiers. This
   closes the missing full-matrix release check, **not** the unfinished verifier matrix. Direct
   deployment scripts are not a qualification bypass; no deployment was run or authorized.
3. **C6 failure/set convergence:** local premature pinning is fixed and C6 evidence can be
   independently reverified/voted. The synchronized exact-set asymmetric live-window end-to-end
   scenario remains open. General expired-accusation recovery predates this port and stays a team
   advisory under the user's scope, without implying arbitrary-delay convergence.
4. **Obsolete C2ab/C3ab/C4ab folds:** confirmed unused by the new node fold but still discovered,
   linted, installed as required artifacts, and copied by the test harness. Their unchanged source
   files are historical legacy from main, but leaving them active is migration cleanup, not evidence
   that they support the new C3 layout. Retirement/deprecation remains an explicit pending decision;
   no source, historical artifact, durable enum discriminant, or old proof was deleted. The stale
   runtime fold comments were corrected. Do not describe this finding as closed.

Verifier-gate evidence in `sync-dec-qualification/`:

- `m3-all-verifiers-before-fix.log`: the previous CLI silently ignored `--all` and returned success
  after checking only the two active default verifiers.
- `m3-all-verifiers-tooling.log`: all **14** tooling tests passed, including synthetic six-pair
  checking, inactive-verifier drift, stale stamps and missing archived keys. Synthetic keys/proofs
  are not cryptographic qualification.
- `m3-canonical-dist-verifiers.log`: real bb regeneration/comparison of both insecure/minimum
  committed wrappers from their archived EVM keys passed, without changing the active selection.
- `m3-all-verifiers-after-fix.log`: the full-matrix command **fails closed as intended** on missing
  `insecure-64/micro/.build-stamp.json`. This is an unqualified matrix, not a passing release check.
- `m3-release-gate-tests.log`: all **30** release-tooling tests passed, including rejection of a
  workflow weakened to check only the active pair. The release workflow itself was not executed.

Remaining port cleanup from that review is retained, not silently applied as statement changes:
document `native_residue`'s checked-opening/range precondition; restore/justify the C4 PRF-path
modulus-width invariant; assess explicit in-flight decryptor-set storage and obsolete backups;
correct C4 length/PRF-noise/field-key-width names and residual ESM comments; explain the contract's
`N=2T+1` grid-width invariant; add the persistence migration pointer; and avoid unrelated
line-ending churn. Circuit-source changes require fresh source-bound artifact verification, not
restamping old proof evidence. The new local `e3-safe` patch comment records the actual upstream
revision and release blast radius; publication/version cleanup remains deferred. Moving this active
plan out of the root belongs to the later pre-main cleanup, not deletion of current review history.

#### Disabled legacy circuits and secure/micro measurement (2026-10-23)

After the `852a733dd` review checkpoint was pushed, the user requested **disable, not remove** for
the retired shared-smudging variants, proper naming, and a new secure/micro benchmark against the
historical measurements. The user later explicitly requested committing and pushing the current
cleanup for cryptographer review; deployment and release remain unauthorized.

**Evidence retention notice after the reported memory-pressure crash:** the host reboot cleared the
approved temporary directory, including qualification logs, new raw benchmark logs, proof exports,
comparison files, source-snapshot files and private I/O adapter. The numerical observations below
were recorded before that loss and remain in this plan; their original temporary evidence paths are
now historical references, not available replay inputs. The checked-in historical baseline, current
source changes and compiled secure/micro artifacts in the durable benchmark worktree survived. Do
not reconstruct missing proof bytes or describe the 13-job experiment as a verified successful run.

The follow-up retains the C2b `dkg/e_sm_share_computation` source and all three paired
`c2ab_fold`/`c3ab_fold`/`c4ab_fold` sources, each with a `DISABLED` marker. Canonical discovery
skips marked packages and rejects explicitly selecting one before tools/writes; Solidity verifier
discovery does not select them. The DKG workspace, required artifact inventory, active benchmark
selection, integration fallback copying and CI fixture sentinel no longer require them. Lint reports
the marked fold packages as disabled rather than checking them. Historical source, reports,
archives/proofs, generic legacy helpers and durable enum discriminants are preserved. This resolves
the pending disposition from review finding 4 by disabling its build/install requirements, not by
deleting legacy history.

Active C6 now names its residue width `BIT_E_FRESH` / `e_fresh_bit`, exposes a fresh-noise-bound
alias for the existing preset sampler bound, and names its recursive c0 column
`C6_FOLD_C0_COL_OFFSET`. Active node-fold accumulators use the shared synchronized layout constants
rather than unused legacy paired-fold lengths. The sampler bounds, arithmetic, witness/public-input
order, field encodings and committee thresholds are unchanged. Generated preset sources were
refreshed through the canonical `sync-preset` operation; no keys or historical build stamps were
relabeled.

Verification before the benchmark:

- Both disabled-package regression tests fail before the discovery fix (Rust still selected the
  marked package; verifier generation did not reject it).
- `disable-legacy-native.log`: the affected build/config/witness/benchmark test targets passed, with
  their two pre-existing ignored tests still explicitly ignored.
- `disable-legacy-tooling.log`: all **15** circuit-tooling tests passed.
- `disable-legacy-noir-tests.log`: **223** library tests, **7** decryption-aggregator tests, and
  DKG/node-fold tests across all three committees passed; the config execution also passed.
- `disable-legacy-noir-lint.log` and `disable-legacy-generated-check.log`: compiler/formatter and
  canonical generated-config comparison passed. `--dry-run` lists **20** active packages, none of
  the four disabled packages.
- `disable-legacy-integration-check.log`: the production multi-node integration target compiled;
  this is not a completed real-proof benchmark or exclusion-convergence scenario.

The benchmark used the durable isolated worktree
`/Users/giacomo/.local/share/opencode/worktree/e2557a/sync-dec-secure-micro`. Its base is
`852a733dd`, with its then-uncommitted source snapshot originally saved as `source.patch` and
`source-snapshot.json` under the approved temporary directory's `sync-dec-secure-micro-20261023/`.
Its own canonical secure-8192/micro build, matching regenerated Solidity verifiers, optimized Rust
leaf measurements, full recursive protocol run and local EVM replay are staged serially. The primary
checkout remains on insecure/minimum; its earlier artifacts and every historical measurement are
preserved.

The valid historical comparator is `circuits/benchmarks/results_secure_micro`: N=9/T=4/H=5, commit
`c2dfd37d`, Noir beta.16 / bb 3, 13 jobs on an M4 Pro with 48 GiB RAM. The similarly named
`results_secure_agg_micro` actually records N=3 and is not a micro comparator. The new run uses
repository-pinned tools, one proof job and 13 Rayon workers, with a 36-GiB owned-process-tree RSS
cap and 20-GiB free-disk reserve. Compilation and protocol phases have explicit time caps. Changed
tools, concurrency, roster behavior, proof statements and historical `isolated_nargo` versus current
production-adapter metric kinds must stay visible; wall-time ratios are not isolated causal
speedups. Measurements are pending until the bounded runner completes; failed/missing stages must
not become successful zero-duration results.

The first attempt **failed during artifact preparation**, not during a proof or at the RSS guard: 19
of 20 active packages built, while C6 EVM-key generation failed writing `~/.bb-crs/bn254_g1.dat`
with `EINVAL`. `build-secure-micro.log` and `run-summary.json` preserve the **2,503.04-second**
attempt and sampled peak **31,797.23 MiB**. Stock pinned bb independently counted C6 at **23,019,005
gates** / **6,176,987 ACIR opcodes** (`c6-gates-stock.*`), implying a padded 2^25-point domain and a
2-GiB uncompressed G1 cache. Direct libc tests reproduce Darwin's rejection of a single I/O request
at that size; bb 5.2.0's file helper does not cap the request before its existing short-I/O loop.
This is a host/tool I/O limitation exposed by the larger C6, not a failed cryptographic
verification. The historical C6 table lists 2,977,263 gates under different tools and statements;
this is not a measured proving-time ratio.

An isolated, benchmark-only Darwin adapter caps oversized `read`/`write` requests at `INT_MAX` and
leaves bb's short-I/O loops, cryptographic operations and checks unchanged. Boundary-byte, EOF,
small-I/O and stock-failure tests pass. The actual pinned bb executable was not modified; its
SHA-256 and the adapter's identity are retained in `io-adapter/validation.json`. The retry uses a
private CRS directory, not resets/deletes of the shared cache. The unchanged bb validated all
**256** compressed G1 chunks against its embedded SHA-256 hashes before populating that private
cache. With the adapter, diagnostic C6 EVM-key generation passed in **40.11 seconds**, sampled peak
**27,309.97 MiB** (`c6-vk-io-diagnostic.*`). This remains key generation, not a verified C6 proof.
Any resulting benchmark must disclose the adapter and private-cache environment.

The user directed that memory growth must be addressed by **smaller circuits**, not by requesting
more RAM. The 36-GiB guard and current machine therefore remain fixed. A separate `retry-1/` attempt
performs a complete canonical build with existing compiler targets retained, then an actual
secure/micro C6 prove/verify preflight before the full benchmark. If that real proof exceeds the
budget, stop and pursue a proof-preserving decomposition rather than increasing the cap. No split
statement or new proof-interface/serialization change has been implemented or approved by the
current measurements; cross-proof SK/PRF commitments, ciphertext/domain/set bindings, fresh-noise
consistency and recursive VK anchoring would need explicit review. Original failed-attempt evidence
is preserved, and the new proof-memory and protocol timing results remain pending.

The I/O-compatible `retry-1` completed the full **20-package** secure/micro build in **180.78
seconds** with existing compiler targets retained, sampled peak **28,030.44 MiB**. Its fresh
complete stamp is **`ad5b16fa759eda29`**; that warm-target build duration is not a cold-build
speedup. The following TS verifier command failed before generation because the long attempt-local
`TMPDIR` exceeded Darwin's Unix-socket path limit. `retry-2` uses the shorter approved temporary
directory and source-matched `--skip-if-built` reuse, not an artifact restamp or weakened inventory
check. It retains the explicit real-C6 proof-memory gate before the full protocol/gas measurement.
Both earlier attempts were retained separately before the reboot; see the evidence retention notice.

#### Secure/micro completed measurements (2026-10-23)

The real secure-8192/micro run completed all **10 active leaf prove/verify cases** and the full
recursive multi-node plaintext scenario with N=9/T=4/H=5, one simultaneous job and 13 Rayon workers.
The integration test passed in **5,158.01 seconds**; its exported `Entire Test` wall phase is
**5,157.916990 seconds**. The full benchmark's sampled process-tree peak was **31,722.44 MiB**
(30.98 GiB), below the unchanged 36-GiB cap. C6's separate real preflight also proved and verified
under the cap (sampled 30,101.88 MiB). No extra RAM or proof-statement decomposition was required
for this requested micro case. This does not establish that larger secure committees fit.

The initial combined benchmark driver still correctly records failure: after the successful
cryptographic protocol run, gas replay supplied a legacy **one-node** committee placeholder and the
strict DKG wrapper rejected its H\*N PRF-grid length with `InvalidPublicInputsLength`. The gas
fixture now supplies the selected configuration's full N sorted local signer addresses, failing if
there are too few. Neither Solidity checks nor the proofs were weakened or regenerated. A separate
`pnpm bench:circuits replay` of the same saved exports then passed both real Solidity wrappers.
`replay-input-digests.json` confirms the original proof and integration exports are unchanged. The
failed original driver record is preserved; successful replay is separately recorded in
`retry-2/replay-1/protocol_results.json` with `source: saved_folded_proof_evm_replay`.

Observed comparison against the retained `results_secure_micro` baseline:

| Measurement                          | Historical |        New |
| ------------------------------------ | ---------: | ---------: |
| DKG: threshold shares to public key  | 5,251.56 s | 3,630.05 s |
| Ciphertext publication to plaintext  |   334.20 s | 1,503.85 s |
| Entire protocol test wall phase      | 5,605.04 s | 5,157.92 s |
| C6 isolated/adapter proving          |    10.14 s |    58.88 s |
| C6 gates                             |  2,977,263 | 23,019,005 |
| C7 isolated/adapter proving          |     0.81 s |     0.22 s |
| DKG wrapper call gas estimate        |  3,136,910 |  3,242,023 |
| Decryption wrapper call gas estimate |  3,658,366 |  3,859,864 |

These are **observed results, not controlled speedups**. Historical concurrency was 13 jobs versus 1
now; Noir beta.16/bb 3 became beta.26/bb 5.2.0; the exported noise lambda is **50 versus 45** in the
current preset (not tuned by this task). Proof statements, transport, topology and metric kinds also
changed. C6 is substantially more expensive, while the retired shared-noise tracks disappear and C7
no longer interpolates. Missing old peak-RSS data is not zero, and wrapper `estimateGas` already
includes transaction costs: keep payload-only calldata estimates separate, not added again as a
claimed total. The older gas-report aggregation convention is a team methodology advisory, not a
protocol change made here.

The public-data consolidated `comparison.md` and `comparison.json` were saved under
`sync-dec-secure-micro-20261023/`, alongside the original failures, frozen source provenance,
successful raw records, protocol proofs and replay; those temporary files were later lost as noted
above. The surviving compiled-artifact stamp is **`ad5b16fa759eda29`**. The measured source was the
isolated dirty snapshot based on `852a733dd`. No production deployment, release, full-matrix claim,
or M2 asymmetric exclusion/recovery qualification is implied by the recorded happy-path
observations.

#### Requested 13-job follow-up experiment (2026-10-23)

The user subsequently requested the benchmark at **13 simultaneous jobs**. A separate
`sync-dec-secure-micro-20261023/jobs-13/` run used the same secure/micro source stamp
`ad5b16fa759eda29`, pinned tools, private CRS/I/O adapter, machine and 13 Rayon workers. Only the
protocol/gas scenario is repeated
(`pnpm bench:circuits gas --mode secure --committee micro --multithread-jobs 13`); standalone leaf
cases are serial measurements and are not reproved just to change a protocol-concurrency setting.
The completed one-job run and historical baseline remain unchanged.

The monitor was configured with a **36-GiB process-tree RSS cap**, a 20-GiB disk reserve and
0.2-second RSS sampling. The user reported excessive memory pressure and a crash, and the harness
cancelled the background command because its server restarted. No final 13-job result or telemetry
survived the temporary-directory cleanup, so neither its peak nor the exact first failing stage can
be established from the available records. No benchmark/bb processes remained on the post-reboot
check. A sampled watchdog is not hard memory isolation and does not reserve memory for the OS and
other applications; this was not a safe configuration merely because one proof had fitted the cap.
Do not rerun it as part of this checkpoint or publish a 13-job speedup/qualification claim.

The latest user instruction is to **commit and push what exists and leave the memory/decomposition
design to the cryptographer**. No circuit split, new proof statement, weakened check, increased RAM
limit or scheduler redesign is included here. Review remains open for memory-bounded proof work
units and their cryptographic bindings, M2 asymmetric convergence, the full release matrix and the
required human protocol approval. This checkpoint records the uncertainty rather than presenting the
crashed experiment as completed verification.

### Prior qualification (historical; does not close the reopened M1/M2 findings)

The user authorized finishing the remaining work, then committing and pushing the feature branch;
merging to main, deployment and publication are not authorized. The user separately approved
protocol **9** / generation **2** and the drained fresh-deployment/history boundary for source
configuration. Schema 9 / gossip 6 / sync 5 remain reserved. The migration/rollback description is
in `docs/architecture/upgrades.md`; execution and the required pre-merge review are still separate.
The user subsequently selected **"Push tested review branch"**: finish branch-level recursive,
contract and recovery checks, then commit/push with larger secure pairs explicitly pending. The full
six-pair production qualification is not a condition of that review-branch push, and must not be
represented as completed.

The latest compiler fix adds a fixed-array SAFE squeeze at a checked permutation boundary, used by
PRF expansion only. It preserves tags, full stream lengths, permutation order, empty/tail behavior,
and continuation state; the general streaming API remains unchanged. **223 Noir tests** and six
nonconstant secure-width old/new mask comparisons passed. This is distinct from the rejected
one-shot/vector and per-element fixed-output experiments below.

The canonical builder has now completed **secure-8192/minimum C6 and C7 plus their VK variants** in
640.15 seconds, with sampled peak 24,383 MiB, under a 900-second / 28-GiB guard. Source-bound stamp:
`71d5265019dfcc1c`. The earlier secure compilation blocker is therefore cleared for this pair.
Actual correlated secure leaf qualification then **passed**: both C6 shares and C7 proved and
verified, their public commitment links matched, and tampered C6/C7 public inputs were rejected. The
run took 282.88 seconds with sampled peak 21,310 MiB. Public-only proof JSON is retained under
`sync-dec-qualification/secure-minimum-public-proofs/`; no witness was exported. This is not secure
DKG/recursion/Solidity or larger-committee qualification. The ignored target supports explicitly
selected committees and optional public-only exports. Its first attempt exposed a test-helper
symlink destination incorrectly reapplying `E3_CUSTOM_BB`; the fixture path is now isolated from the
already-resolved real binary, and the rerun passed with the explicit custom binary.

Native builder tests passed (18 library + 2 CLI, one ignored); canonical generated-config comparison
passed. Updated capability runs passed aggregator (93), config (45) and keyshare (162). A network
expiry scenario failed both parallel and serial runs because its peer-dial selection could choose a
non-loopback host interface. Its test-only dial now prefers loopback while retaining advertised
non-loopback Kademlia addresses; the focused regression passes without changing admission policy or
timeouts. The complete serial network rerun passed all **303 tests**. Slashing (46), trbfv (7) and
zk-prover (166, two ignored) also passed. The replacement scenario compiles successfully.

An optional Small-committee real-proof scenario now holds compute admission while delivering a
confirmed exclusion, observes replacement requests on every newly designated node, then releases the
retired share calculation and checks that it starts no C6 proof. It also asserts that all published
shares use the replacement set. Its complete insecure/Small artifacts built in an isolated worktree
in 55.41 seconds (sampled peak 4,944 MiB). The real-proof scenario **passed** in 4,997.91 seconds,
with sampled peak 8,388 MiB, below its 6,000-second / 12-GiB guard. It completed 20 share
calculations (old and replacement sets) but only the 10 replacement C6 proofs, then real recursive
aggregation and the expected plaintext. Minimum/Micro have no spare member in their selected H
roster. This models delivery of an authenticated exclusion fact, not accusation consensus or
on-chain slashing. The explicit late-response injection is a retired share calculation; late C6
proof-worker responses retain their separate native/unit coverage rather than being described as the
same real-proof case.

The complete insecure/minimum artifact pair was rebuilt canonically afterward (47.23 seconds,
sampled peak 4,925 MiB), restoring the intended default selection and both VK-tree anchors. Fresh
recursive/contract and distributed checks use these regenerated artifacts, not the earlier fixture.
The default pair's source-bound stamp is `65bc58b7910d3084`; the secure leaf stamp above belongs to
its different preset.

Completed current-revision verification:

- The correlated full C0-C7 recursive suite and both adversarial fold tests passed. The final EVM
  proofs refreshed the committed public-only fixture through its canonical sync script.
- All **859 contract tests** passed, including actual real-proof Solidity acceptance. An initial
  invocation incorrectly supplied a wrapped export as a raw fixture and skipped that one case; the
  corrected raw-fixture run has no pending tests. Storage validation, size checks and verifier
  comparison passed; registry EIP-170 headroom remains 205 bytes.
- The full real-proof multi-node flow passed with recursive aggregation enabled and a persisted-C6
  disk restart (184.85 seconds, sampled peak 6,206 MiB). It exported public proofs and timing data,
  not node secrets. This is separate from the successful Small replacement case.
- The complete root Noir suite, Noir checks and Rust formatting passed. Its configuration verifier
  first exposed the stale pre-Lagrange C6 quotient formula and a narrowing u128 comparison. The
  verifier now independently decomposes the cubic bound in safe u128 intermediates and compares the
  final 129-bit result in Field; generated parameters and cryptographic statements were not
  weakened. The same command failed before this checker fix and passed afterward.
- Events (136), EVM (258), request (67), multithread (32), proof/slashing integration (12) and the
  runtime layout comparison passed in addition to the capability counts above.
- SDK tests initially found a stale generated WASM bundle without the current preset name. Both WASM
  targets were rebuilt after preserving the old bundle. All **37 SDK tests**, **7 prepared real
  encryption-proof tests**, and the actual browser encryption smoke passed. The browser run used
  installed Chrome through the supported `CHROME_BIN` option; no check was skipped.
- Circuit tooling, license, committee, address, repository and documentation-link checks passed.
  Root ESLint traversed the preserved ignored CRISP experiment; the same unchanged check passed in
  the clean verification worktree. The experiment and lint exclusions were not modified.

Qualification logs and the debug-info-disabled Cargo target are isolated under the approved
temporary directory's `sync-dec-qualification/`. Canonical circuit outputs are being recreated as
needed, with a 20-GiB free-disk reserve; unrelated experiments remain untouched. The primary
checkout stayed on insecure/minimum while Small ran in the separate worktree. That worktree has now
been restored through the canonical builder to the complete default pair for final checks/push. At
this pre-commit checkpoint no commit or push has run; final repository hooks still gate the approved
review-branch push. The full release matrix and larger secure qualification remain explicitly
outside this milestone. Main merge, deployment and publication remain unauthorized.

### Post-change read-only review for the review branch

Reviewed the actual latest diff and owner code separately from editing, under proof statements,
threshold decryption, DKG, persistence/replay, network delivery and upgrade policy. This is an
implementation review, not an independent cryptographic audit or the required human pre-merge
approval.

- Checked fixed-array SAFE rate/tail/empty transitions and its boundary/IO assertions against the
  original streaming API; checked context index partition, exact-peer selection, full-N recipient
  anchors, scalar-quotient no-wrap conditions and already-bounded wide packing.
- The configuration checker's cube decomposition keeps its u128 intermediates bounded for `q<2^62`
  and u32 degree, while its final Field result stays below the scalar modulus. No generated bound,
  scalar-equality requirement or compiler safety check was relaxed.
- Rechecked current-round digest validation before C6 intent creation, authenticated share/domain
  admission before slot use, and exclusion-derived replacement/retirement. The new harness holds
  compute admission rather than actor mailboxes and releases the slot on early failure.
- Old-data replay remains unsupported across schema 8/9; old peers fail protocol/wire admission; old
  deployed verifiers/anchors do not establish acceptance of the new statements. The approved source
  decision is protocol 9/generation 2 with a fresh drained history boundary, not a rolling upgrade
  or counter downgrade. Same-address history conversion and rollout execution remain out of scope.
- Test-only symlink and interface-selection fixes preserve production configuration precedence, peer
  admission and timeout semantics. No L-BFV/chunking feature was introduced and the local SAFE
  dependency patch is retained. Remaining evidence limits are recorded above, not treated as
  successful release qualification.

### Secure-size component isolation and fixed context storage

Follow-up probes used the pinned compiler with all constraint checks enabled, normally capped at 60
seconds / 8 GiB. Degree-8192 results isolated several distinct costs: the existing product kernel
compiled in 4.18 seconds, checked ciphertext commitments in 7.26 seconds, and the C6-style packed
transcript in 50.83 seconds. A 24,576-value reduction batch and the original context digest reached
the time cap; a full PRF stream reached the memory cap. These are component diagnostics, not secure
C6 qualification. A synthetic degree-1024 C6 with secure widths/bounds compiled in 25.91 seconds; it
is not a supported BFV preset.

Adopted one further change in `circuits/lib/src/math/prf.nr`: construct the complete PRF context in
a fixed-size array rather than repeatedly growing a vector. Preserve exactly
`|S|, S, level, component count`, each component's `L, N`, CRT/ascending coefficient order, native
conversion, domain, IO pattern and digest width. The general streaming SAFE implementation remains
unchanged. The latest repository context component compiled at degree 8192 in **17.88 seconds**,
below the same 60-second guard, to 999,436 ACIR opcodes. The prior growing-vector version did not
complete within that guard. No claim is made about its uncapped time or full-C6 speedup.

Verification after adoption: **220 Noir tests** passed, including a frozen legacy three-CRT digest
with signed boundary coefficients. Six nonconstant secure-width old/new mask comparisons and three
nonconstant three-CRT context comparisons passed. The actual insecure/minimum C6 still has 23,727
ACIR opcodes / 88,437 Honk gates (latest compile: 1.58 seconds); a fresh real recursive-target proof
verified with the unchanged legacy nonzero-mask witness. Wrong quotient, noise lift, key and
owner/set witnesses, plus tampered proof public inputs, were rejected. The existing 120-case integer
reference evidence still covers the unchanged reduction implementation. Read-only review recorded
the quotient/no-wrap argument, exact peer selection, subtraction preconditions, wide packing and
context index partition in the scratch audit's `performance-review.md`.

Rejected scratch alternatives remain outside repository source: batching all division hints into one
unconstrained call did not materially improve compilation, and a fixed-output squeeze method did not
resolve PRF-stream scaling. Their value tests passed; that alone did not justify adoption. These
snapshots are separate from the earlier rejected one-shot SAFE experiment.

One post-change full secure C6 diagnostic reused the earlier **180-second / 16-GiB** guard. It still
reached the time cap (sampled peak 11,219 MiB), with no compiler diagnostic or proof attempt. Pass
timings show that the first constant-folding pass completed, but the first unrolling pass did not
finish. Remaining investigation should therefore cover whole-C6 unrolling/array handling as well as
the earlier dominator hotspot, rather than only extending timeouts or rewriting the product kernel.
**Secure compilation/proving, fresh correlated recursion/contract runs, the release matrix and
distributed failure/recovery qualification remain outstanding.**

Logs, bounded runners, source snapshots, differential programs and public proofs are preserved in
the approved temporary directory's `c6-cost-audit/`. `report-before-context.json` preserves the
prior report; `report.json` records the latest source fingerprints and evidence. Disposable scratch
component targets were cleaned without touching the other experiments, saved results or proofs. No
repository preset switch, Cargo target, circuit archive, commit, push or publication was made.

### C6 cost audit and bounded optimization

After the requested artifact cleanup, inspected C6 without recreating Cargo or project circuit
outputs. The polynomial product already uses the linear-time negacyclic kernel. The main cost was
the PRF: a full-committee loop behind witness-dependent branches expanded unnecessary streams, and
every output used a canonical 254-bit decomposition and repeated modular operations.

Changes under test:

- Select exactly `S-1=T` peers before expanding their two PRF streams. Preserve full-N key indexing,
  canonical owner/set checks, key commitments, SAFE domain/IO pattern, full `L*N` output length and
  coefficient order.
- Verify full-field quotient/remainder witnesses with `quotient<=floor((p-1)/q)` and a tighter
  remainder limit at that quotient. Bounds derive from a checked byte reduction of p-1, so scalar
  equality cannot conceal the integer alias n+p. Small moduli retain the checked byte/limb path. No
  unverified remainder or mask is trusted.
- Flatten modular subtraction's verification equation and require its result below the modulus. This
  resolved a Brillig-coverage `bug:` diagnostic exposed during a full C6 compile; no underconstraint
  or Brillig checks were disabled.
- Batch modulus-only bounds per CRT row and keep the scalar-bound implication and centered-residue
  selection branchless. A sampled secure compile spent almost all active compiler samples in
  constant-folding dominator lookups, not BFV convolution. Long coefficient-wise control-flow chains
  therefore need attention as well as final gate counts.
- Fix the secure quotient packing path: `BIT_R=129` needs one shifted coefficient per carrier. Plain
  packing now permits that already-bounded case, matching Rust's existing codec; checked packing
  still rejects the unhelpful single-carrier layout. This does not substitute raw coefficients into
  the Fiat-Shamir transcript.

Single-run isolated measurements with pinned nargo/bb (not the release matrix):

| Case                                                        | Before                 | After               |
| ----------------------------------------------------------- | ---------------------- | ------------------- |
| One secure-width field reduction                            | 1,407 ACIR opcodes     | 14 ACIR opcodes     |
| One PRF stream, degree 64 / 3 limbs                         | 270,204 ACIR opcodes   | 2,748 ACIR opcodes  |
| Full PRF mask, degree 64 / 3 limbs / 3 parties / set size 2 | 1,448,466 ACIR opcodes | 16,669 ACIR opcodes |
| Same mask compile time                                      | 102.57 seconds         | 1.26 seconds        |
| Same mask peak resident memory                              | 10.12 GB               | 0.34 GB             |

The mask microbenchmark repeats a secure-width modulus to isolate arithmetic cost; it is not a valid
BFV preset qualification. The actual insecure/minimum C6 wrapper compiled separately to 23,727 ACIR
opcodes / 88,437 Honk circuit gates (latest compile: 1.68 seconds). A real recursive-target C6 proof
verified for non-prefix set `[1,3]`, using a nonzero mask generated by the pre-optimization code and
deterministic zero SK/ciphertext/noise material. A tampered public-domain input was rejected. This
is a leaf regression proof, not a new DKG-to-contract or distributed run.

Verification: 219 Noir library tests passed, including malformed sets, frozen non-prefix masks,
Python integer remainder vectors and forged scalar-wrap/quotient/remainder rejection. A constrained
program also matched 120 nonconstant witness cases against Python and rejected an incorrect
remainder. Six constrained old/new mask comparisons passed for all owner positions in `[1,3,5]`,
including negative centered ciphertext coefficients. The latest full C6 accepted the unchanged
public share commitment and old nonzero mask; wrong quotient, noise lift, key and owner/set were
rejected, and a freshly generated proof verified. Scratch programs/logs/public test proofs are in
the approved temporary directory's `c6-cost-audit/`; no real node secrets were used.

Isolated secure-size C6 compiles reached diagnostic time caps (180 and 360 seconds, peaks below 11
GiB), without a compiler diagnostic; those caps are not proof rejection. Sampling identified
`InstructionResultCache::get` / `DominatorTree::dominates_helper` in Noir's constant-folding pass as
the active hotspot. A block-wise one-shot SAFE experiment passed 220 Noir tests and preserved six
old/new mask cases, but still hit the 180-second cap (peak below 13 GiB). It was therefore **not
applied to repository source**. The general streaming SAFE implementation is unchanged. Further
secure compiler scaling investigation and actual secure proofs remain necessary. No repository
preset was switched. **Earlier recursive/Honk fixtures and verification results below predate these
optimizations**: canonical artifacts, real secure C6/C7, recursive/contract acceptance and the
supported release matrix must be regenerated/revalidated before production readiness.

### Production completion pass (2026-10-22)

- Full correlated C0-C7 test restored and passing: selected dealers `[0,2]`, decryptor coordinates
  `[1,3]`, DKG/decryption EVM-target proof verification and tampered-proof rejection. Final counts
  are 16 and 76 fields. Public-only exports refresh the committed BFV fixture through
  `sync_bfv_vk_binding_fixture.sh`; no witness secrets are exported.
- `BfvVkBindingIntegration.spec.ts`: all 3 tests passed, including real final Honk proof acceptance
  through both wrappers. Registry anchors come from the correlated DKG proof, not the decryption
  proof under test. These results are insecure-64/minimum only.
- C3/node/nodes/DKG/C6/final-decryption folds and Rust builders now propagate PRF commitments.
  Complete insecure/minimum artifacts and both Honk verifiers were regenerated canonically. Both
  fold-accumulator tests passed, including adversarial leaf/genesis/predecessor VK substitution.
- EIP-712 v2 uses `prfKeyCommitsHash`; v1 signatures and altered rows are rejected. New PRF storage
  is appended and the registry gap shrinks by one slot; old ESM mappings stay reserved. The registry
  uses its existing linked library to verify/store DKG anchors and stays below the release size
  budget (24,371 runtime bytes, 205 EIP-170 bytes headroom). Production storage baselines are
  unchanged.
- PK/decryption routers now dispatch the new grid-carrying layouts. The focused wrapper/PRF/v2
  contract run passed 58 tests. The complete `pnpm evm:test` suite passed **859 tests**, including
  the real-proof fixture, router, attestation, publication and lifecycle cases.
- Strict admission binds signed C6 owner/set before slot reservation. Hydration also validates round
  identity and share commitments. Local wakeups do not authorize arbitrary sets; authenticated
  exclusions independently drive each keyshare, duplicate sets coalesce, and replacement retires
  obsolete recovery work and proof correlations. Actor fixtures were updated rather than relaxing
  checks. Keyshare passed 162 tests; aggregator passed 93 tests after the hydration checks; the
  proof-request actor target also passed with the replaced-set regression.
- The final zk-prover unit run passed 166 tests with 2 intentionally ignored tests after removing
  retired ESM-only links/tests. The v2 ethers digest/signature vector passed in that run.
- `pnpm evm:build`, `validate:upgrade`, and `size:check` passed after the gap/size corrections.
  `check:bindings` reports the intentionally added getter versus HEAD; regenerated artifacts are in
  the uncommitted diff. It cannot pass its HEAD comparison until those artifacts are committed;
  committing is not authorized by this task.
- Native suites passed for events (136), EVM (258), multithread (32), request (67), trbfv (7),
  slashing (46) and net (303). Runtime, DHT and node-proof-plan layout locks were regenerated only
  for the reserved schema/wire change; normal comparison runs passed afterward. Config (22),
  parameter (54), witness targets, samples (13) and trbfv integration also passed.
- `test_trbfv_actor` passed with real recursive aggregation enabled and the normal 1,000-second
  plaintext observation budget (441.46 seconds total). The earlier 180-second override timed out
  while a second C6 job was still running. A preceding run exposed a genuine old C2 parser in the
  runtime consistency checker; it is fixed to skip the outgoing-key prefix rather than mistake keys
  for messages. The same scenario failed before that fix and passed afterward.
- Retired ESM consistency rules and their obsolete rule-specific tests were removed; enum wire
  discriminants remain reserved. New C2/C3 tests reject keys used as message commitments. C6/C7
  comparisons now require the same designated set and c0, so absence of a comparable target cannot
  accuse an earlier round; both link and slashing-owner regression tests passed.
- Both adversarial fold tests passed (186.78 seconds); `rust:test:slashing` passed all 12 selected
  tests, including Anvil collateral-call mocks (not real collateral settlement). Rust's v2 fold
  digest/signature also matches an independently generated ethers v6 vector.
- `build:circuits check --committee all`, builder tests (18 plus 2 library tests, 1 ignored),
  `nargo test` (205), verifier comparison, whole-workspace test compilation and repository guards
  passed. The new authored `configs/recursive.nr` is now a required source-hash input, with a
  mutation regression; the complete insecure/minimum pair was rebuilt rather than restamped.
- Secure-8192/minimum C6/C7 qualification was **blocked before proving**. C7 built; C6's
  `nargo compile` ended with `SIGKILL`, and the runner reached its 1,800-second build cap. No secure
  proof test ran, so this is neither proof rejection nor qualification. The complete
  insecure/minimum artifact pair and selectors were then rebuilt successfully; no compiler/prover
  jobs remained. Logs: `prod-secure-qualification.log`, `prod-secure-leaf-build.log`, and
  `prod-post-secure-restore.log` in the approved OpenCode temporary directory. The ignored
  `secure_sync_decryption_tests` target is ready for a prepared secure runner. Secure final
  aggregation and the full six-pair release matrix remain unqualified.
- After restoration, Solidity compilation, real wrapper/v2 tests, storage compatibility, verifier
  comparison and Solhint passed again. Rust formatting and license checks passed. The only Solhint
  warnings are the repository's existing 10 warnings; no limits or baselines were changed.

Still required: resolve secure C6 compile/resource limits, verify secure artifacts/proofs and the
complete release matrix, and obtain the required human review. Same-address historical DKG replay is
**not supported**: the proposed rollout requires a drained fresh registry/deployment-history
boundary, not a bare store reset on old addresses. Final protocol counters and rollout remain for
human approval. No commit, push, deployment or release ran.

### Read-only protocol review record

Reviewed the actual diff and post-change keyshare/aggregator admission, canonical-key projection,
recursive trust tree, final decryption statement, registry/wrapper/storage and source-identity code.
References: DKG, threshold decryption, proof statements, artifact identity, persistence/replay,
network delivery and upgrade policy. This is a scoped implementation review, not a cryptographic
audit or the required human approval.

Findings fixed and regression-checked: stale C2 consistency offsets could accuse honest dealers;
old-round C6/C7 comparisons could treat missing comparable work as fault; missing recursive-layout
source identity allowed stale artifact provenance; appended registry storage needed one gap slot;
registry bytecode initially exceeded EIP-170. Preserved constraints include exact T+1
reconstruction, selected-H/full-N indexing, complete recursive VK anchoring, canonical fields and
message ranges, fresh-noise reuse, signed owner/set admission and exclusion-derived replacement.

Compatibility remains deliberately breaking: schema-8 stores cannot be reused, old wire majors are
rejected before decoding, and deployed pre-migration verifiers do not accept the new statements.
Same-address old-chain replay and secure matrix coverage are outstanding limits. `protocol_version`
and `node_generation` remain unchanged for explicit human review; **not approved for release**.

### Earlier leaf checkpoint (superseded where the production pass above differs)

- Branch: `feat/sync-decryption`, base/HEAD `354af6bef`; all changes remain uncommitted.
- C6 now prepares and proves `lambda*c1*sk + fresh_noise + PRF_mask`. It retains main's checked
  packing and reduced negacyclic identity. Bounds use arbitrary-precision integers, including the
  centered Lagrange factor. The witness uses the released FHE PRF implementation, not a copied host
  implementation.
- C7 adds the committed c0 prefix once and sums already-weighted partials without interpolation.
  Correlated sample data retain the exact C6 inputs from the same released-FHE reconstruction.
- Both C6 and C7 generated and verified real Barretenberg proofs with the current insecure/minimum
  artifacts. Dedicated compiled-Noir tests also reject altered keys, shares, noise, c0 and message.
  This is leaf evidence, not a recursive or contract end-to-end result.
- C2 commits packed messages plus outgoing key commitments; C3 constrains the transported key bits;
  C4 opens the packed messages, removes the bits, and commits the normalized aggregate SK plus
  incoming key commitments. Compiled witness acceptance/rejection tests cover these leaves.
- C1 no longer takes or outputs shared ESM material. Its public output is
  `(sk_commitment, pk_commitment)`; the generated noise bound remains available for C6.
- Whole-workspace test compilation passed with fixture auto-build disabled. Native tests and format
  checks are being repeated as interfaces change; do not treat that as proof-matrix success.
- The filesystem filled during testing. Only rebuildable Cargo outputs for zk-prover, keyshare, evm
  and net were cleaned; source work and saved benchmarks were retained. Subsequent checks use
  `CARGO_INCREMENTAL=0` to limit cache growth.

### Required before completion

1. Propagate the changed public-input layouts through C3/C6/node folds, DKG/decryption aggregation,
   Rust proof parsing, VK genealogy, canonical key projection, and Solidity wrappers/registry
   anchors. PRF key commitments must be anchored across DKG and decryption, not merely supplied by
   the prover. Old ESM anchors do not establish that binding.
2. Bind every admitted decryption bundle to one canonical designated set, including initial
   collection, recovery, and set replacement. Review authenticated network delivery and replay of
   `FreshDecryptionRound`; its local event alone does not reach another node.
3. Restore and adapt the correlated node-fold proof tests removed by an earlier port batch
   (`crates/zk/prover/tests/common/node_fold_witness.rs` and
   `crates/zk/prover/tests/node_fold_correlated_e2e_tests.rs`). A compile after deleting coverage is
   not a replacement for those tests. Update the remaining old-public-layout fixtures faithfully.
4. Regenerate final recursive artifacts/verifiers through the canonical flow; run the real
   multi-node and proof-to-contract scenarios and review the actual diff.

### Compatibility decision for review

The current development branch uses disk schema **9**, gossip wire **6**, and sync wire **5** to
reject old payload encodings rather than decode them as new events. The proposed rollout is a
drained, coordinated reset/redeployment, not a rolling upgrade. DKG key material and proof artifacts
must be regenerated. Existing deployed verifiers remain pre-migration-only; they are not declared
compatible with the new leaves. `protocol_version` and `node_generation` have **not** been changed;
their final values and the production rollout require human approval. Rollback requires the old
binary, old verifier/artifact combination and a pre-upgrade store backup, not reuse of schema-9
state. No deployment, publication, release or rollback operation has been run.

Review references:
`docs/architecture/{dkg,threshold-decryption,proof-statements, circuit-artifacts,persistence-and-replay,upgrades}.md`
and `CONTRIBUTING.md#protocol-compatibility`. Review outcome: **incomplete / not ready for merge**
until the requirements above are met.

### Verification record for this checkpoint

Cargo commands below used `CARGO_INCREMENTAL=0 E3_ZK_PROVER_SKIP_FIXTURE_BUILD=1`; the latter
disables fixture installation, not proof verification. Pinned tools available locally were nargo
1.0.0-beta.26 and Barretenberg 5.2.0.

- `cargo check --locked --workspace --tests`: passed.
- `cargo test --locked -p e3-zk-config -p e3-zk-witness`: passed (22 configuration tests and 41
  witness tests across targets). Two additional signed-noise encoding/parser tests passed.
- `cargo test --locked -p e3-fhe-params`: 58 tests passed in the covering parameter run.
- `pnpm build:circuits check --committee all`: passed.
- `nargo test` in `circuits/lib`: 205 tests passed; output contained no `bug:` diagnostic.
- `cargo test --locked -p e3-net --lib domain::wire::tests::`: 9 tests passed, including rejection
  of pre-migration wire versions before payload decoding. Earlier use of a different filter ran zero
  tests and is not counted as evidence.
- `cargo test --locked -p e3-aggregator --lib`: 93 tests passed after adapting C1/C7 fixtures;
  recovery now rejects a retained C7 proof with a mismatched c0 commitment.
- `cargo test --locked -p e3-zk-prover --test witness_compatibility -- --ignored`: all 4 selected
  compiled-circuit tests passed, including the nonconsecutive C6 set `{1,3}` and malformed inputs.
- `cargo test --locked -p e3-zk-prover --test local_e2e_tests test_share_ -- --nocapture`: C2 and C3
  proofs generated and verified, 2 tests passed without skips.
- The same real-proof target's `test_pk_generation_commitment_consistency`,
  `test_threshold_share_decryption_commitment_consistency`,
  `test_decrypted_shares_aggregation_commitment_consistency`, and
  `test_c4_c6_sk_commitment_aligned_transcript_e2e` passed. The last explicitly verifies both proofs
  before comparing the C4-to-C6 SK anchor.
- `witness_compatibility` compiled-circuit tests cover changed C6 inputs, C7 reconstruction and
  C2/C3/C4 packed-key constraints. These are witness checks, not additional SNARK verification.

Real-proof coverage above is **insecure-64/minimum only**, not a secure preset, committee matrix,
recursive tree, multi-node deployment or deployed contract result. The full zk-prover unit suite has
not been revalidated. The earlier C0 recovery failure from a stale degree-128 `pk.json` was resolved
by a canonical `--group dkg --circuit pk` rebuild; the exact failing recovery test then passed.
Regenerated leaf VKs do not supply the missing final recursive VK-tree anchors.
Formatting/shared-source changes also require the final source-bound artifact rebuild before any
release.

### Leaf ABI checkpoint for the recursive port

These are current wrapper public-field orders, not a completed recursive interface. Set size is
`S = T+1`, message width is `K`, and `L` below is the threshold CRT count. The full-N grid and
selected-H roster axes must not be interchanged.

| Leaf | Public fields, in order                                                                                                                         |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| C1   | SK commitment, threshold-PK commitment                                                                                                          |
| C2   | expected SK commitment, outgoing key commitments `[N]`, packed-message commitments `[N][L]`                                                     |
| C3   | expected recipient PK, expected packed message, recipient index, row index, key commitment, ciphertext commitment                               |
| C4   | expected packed-message commitments `[H][L]`, recipient index, incoming key commitments `[H]`, aggregate SK commitment                          |
| C6   | aggregate SK, ciphertext, c0 prefix, domain high/low, party index, designated IDs `[S]`, outgoing keys `[N]`, incoming keys `[N]`, d commitment |
| C7   | d commitments `[S]`, party IDs `[S]`, message `[K]`, c0-prefix commitment                                                                       |

The C7 c0 commitment is appended, preserving the existing D/party/message prefix and matching the
reference decryption-aggregator convention. Runtime C7 admission compares that tail against every
selected C6 c0 input. C6 key fields above are commitments; raw keys remain private witnesses. The
recursive port must also retain main's recipient-key consistency and checked-opening guards.

## Non-goals

- No lbfv modules (`aggregate_lbfv`, `gen_lbfv_key_shares`, `lbfv_operation`, `bfv-client` lbfv
  envelope), no chunk circuits (`esm`/`sk`/`c2ab`/`c3ab`/`c4ab` folds), no preset renames
  (`secure_8192`/`secure_16384` stay as main's current scheme).
- No `crates/support` (risc0/Boundless compute provider) — separate feature, tracked separately.
- No `agent/` harness docs, no `examples/CRISP` changes (both lineage-only).

## Why a fresh branch instead of moving `feat/share-prf-bit`

- The branch is 205 commits behind main; its base (`lbfv/chunking`, PRs #1909–#2041) was never
  merged into main and its `origin/lbfv/*` branches were deleted upstream.
- Merging main into the branch yields ~365 conflicts (crate restructure, CRISP deletion, preset
  rename collisions) and the end state would still contain lbfv — contradicting the goal.
- The sync-dec slice is separable: measured drift of main vs the lineage base against the feature
  delta for the core files:

  | File                                           | main drift   | feature delta | port difficulty            |
  | ---------------------------------------------- | ------------ | ------------- | -------------------------- |
  | `trbfv/calculate_threshold_decryption.rs`      | none         | +102/−12      | applies as-is              |
  | `trbfv/calculate_decryption_share.rs`          | small        | +157/−73      | near-direct                |
  | `aggregator/.../verify_decryption_shares.rs`   | tiny         | +113/−20      | near-direct                |
  | `keyshare/.../create_decryption_share.rs`      | moderate     | +240/−18      | adapt                      |
  | `circuits/.../share_decryption.nr`             | heavy        | +233/−81      | semantic rework (hardest)  |
  | `circuits/.../decrypted_shares_aggregation.nr` | near-rewrite | +30/−94       | re-apply on main's version |
  | `circuits/.../dkg/share_encryption.nr`         | huge         | +52/−15       | targeted re-apply          |

- Additive circuit pieces port directly: `circuits/lib/src/math/prf.nr` (+197, new),
  `math/modulo/U64.nr` (+53, new), `math/commitments.nr` additions (+54).

## Dependency changes (fhe.rs v0.4.2 → v0.5.1)

Current v0.5.1 resolves its upstream `e3-safe` pin to `ab6b9c943b6d23eda8d0ba5f2efd9039635acfa4`.
The workspace patch was **introduced on this branch**, not inherited from main, and applies to
release builds too. Its manifest comment now names the exact upstream revision and requires
compatibility re-review when either implementation changes; do not infer future byte equivalence
from the version or pin alone. Publication/version cleanup remains deferred. The decision/sequencing
notes below retain the earlier v0.5.0 transition context.

- `fhe`, `fhe-traits`, `fhe-math`, and `fhe-util` now use tag `v0.5.1`; the local e3-safe patch is
  active.
- `threshold-decryption` is **on by default** (optional `ark-ff 0.5`, `e3-safe` git rev `fb5f141`).
- Add to the workspace manifest:
  `[patch."https://github.com/theinterfold/interfold"] e3-safe = { path = "./crates/safe" }` so the
  graph uses the in-repo `crates/safe` (0.20.0-dev.2).

**Decision (2026-10-21):** patch bridge now, publish later.

- fhe.rs re-pins `e3-safe` from rev `fb5f141` (0.17.0) to the Interfold commit this port branches
  from (currently `ab6b9c943`, 0.20.0-dev.2) via a quick upstream PR. Pin to the **port branch's
  base commit**, not "latest main" — Interfold is mid folder-restructuring, and pinning the exact
  base makes the patch unification exact by construction.
- Interfold adds the `[patch]` above. It applies because both sides then resolve version
  `0.20.0-dev.2`; with a rev pin this is stable (no floating-source drift).
- Publishing `e3-safe` to crates.io and switching fhe.rs to a version requirement is **deferred
  until the Interfold folder restructuring completes**; revisit at the 0.20.0 release. Note caret
  requirements cannot match pre-release versions, so the version dep needs a stable tag anyway.
- Sequencing: the port can start on fhe.rs v0.5.0 **before** the upstream re-pin lands — the
  unpatched graph carries two e3-safe copies (git 0.17.0 + in-repo 0.20.0-dev.2), which compiles and
  is functionally identical on native targets (changes since `fb5f141` are OpenVM feature-gating and
  test cleanup; the native `SafeSponge`/permutation is untouched). Switch to the re-pinned tag
  (v0.5.1 or the merge commit) + `[patch]` as soon as it is available.
- During restructuring, re-check the patch source URL, crate path, upstream version requirements and
  lockfile. A workspace version change alone does not establish a patch failure: the released fhe.rs
  manifest specifies a git revision for e3-safe without an explicit version requirement.
- Blast radius on main is small: `ShareManager`, `SmudgingBoundCalculator(Config)` re-exports
  survive at the same `fhe::trbfv` paths. The retired legacy threshold-decryption _methods_ affect
  exactly the two trbfv files this port rewrites — dependency migration and feature port coincide.
- New API surface consumed: `SynchronizedDecryptor`, `DecryptionShare`, `PartyPrfKeys`,
  `PartyPrfKeyTransport`, `PrfKey`, `PRF_KEY_LEN`, `SmudgingNoise(Generator)`, `Lambda`,
  `ShamirSecretSharing`. The reference branch was already aligned to this API (commit f342b4ef0
  "align PRF integration with consolidated fhe API"), and fhe.rs v0.5.0's `trbfv_sync_dec` fixed
  vectors were generated against it.

## Workstreams

1. **Dependency bump + compile**: bump tags, add `[patch]`, fix retired-method call sites (expected
   only `crates/trbfv/src/{calculate_decryption_share,calculate_threshold_decryption}.rs`). Verify:
   `cargo check -p e3-trbfv -p e3-fhe-params`, workspace `cargo check`.
2. **Circuit math (additive)**: port `prf.nr`, `math/modulo/U64.nr`, `commitments.nr` additions.
   Verify: circuit compile for affected bins; PRF gadget must reproduce fhe.rs v0.5.0 fixed vectors
   (domain separators, encoding, coefficient order) — cross-check against
   `crates/fhe/tests/trbfv_sync_dec.rs` upstream.
3. **Host Rust**: trbfv sync-dec path (near-direct port), keyshare `create_decryption_share.rs`
   (+240 reference, adapt to main's threshold_keyshare), aggregator `verify_decryption_shares.rs`,
   events/net wiring. Verify: `cargo test -p` per crate.
4. **Circuit rework**: `share_decryption.nr` (heaviest), `decrypted_shares_aggregation.nr`,
   `pk_generation.nr`, dkg `share_encryption/share_decryption/share_computation` on main's bases.
   Witness shapes rebuilt for main's `secure`/`insecure` + committee presets (micro/minimum/small).
5. **zk prover**: adapt threshold/proof-request wiring to `crates/zk/{config,prover,witness}`.
6. **Contracts**: `BfvDecryptionVerifier.sol` changes from the reference; regenerate verifiers and
   artifacts via the canonical generators (`scripts/circuits`); update addresses/manifests per
   `packages/interfold-contracts/README.md`. **Protocol gate — see below.**
7. **Tests/e2e**: port reference tests (keyshare, trbfv integration, circuits e2e, contracts specs);
   real-crypto end-to-end scenario as the final gate (mocks are insufficient for proof changes).

## Protocol / review gates (per CONTRIBUTING "Protocol compatibility")

- Decryption share proofs and the decryption verifier change cryptographic statements and witness
  shapes: needs explicit compatibility/rollout decision (coordinated verifier + program
  redeployment; release-counter implications) recorded for human review. Not decided silently.
- PRF key transport during DKG touches durable event/snapshot schemas (`PartyPrfKeyTransport`):
  confirm event identity/replay implications and whether a schema-version bump is required.
- `invariant-review` pass required before reporting contracts/circuits changes done.

## Risks

- Circuit ↔ host PRF equivalence (mitigated: upstream fixed vectors + cross-check tests).
- Main's preset/committee structure differs from the reference lineage — witness regeneration and
  verifier rebuilds must use canonical generators; preset switches touch shared files (coordinate).
- fhe.rs v0.5.x patch releases could adjust the consolidated API (low — just released from our own
  stack).

## Estimate

~60–90 files (vs 276 in the lineage diff). Host side: days. Circuits: the hard core.
Contracts/verifier regen + real-crypto e2e: the gating tail. Order of a focused week of engineering
with verification.

## Disposition of existing branch

`feat/share-prf-bit` stays as the reference. After the port lands and is validated, archive/delete
it and the `lbfv` local branches.

## Open questions

1. **RESOLVED — PRF key transport vs. main's DKG presets.** The reference packs each PRF key bit as
   `residue + bit·q` in a share-limb coefficient, which requires DKG share plaintext modulus
   `t = 2q` (q = largest threshold modulus). Parameters differ between the lines:

   | preset                         | threshold max q         | DKG t                   | relation  |
   | ------------------------------ | ----------------------- | ----------------------- | --------- |
   | reference secure 8192          | 288,230,376,164,294,657 | 576,460,752,328,589,314 | t = 2q ✅ |
   | main secure 8192               | 576,460,752,317,792,257 | 576,460,752,317,792,257 | t = q ❌  |
   | pre-migration main insecure 64 | 134,217,089             | 134,217,089             | t = q ❌  |

   Options: **A** narrow preset switch (keep main's threshold params, set DKG t = 2q per preset,
   regenerate DKG-side configs/verifiers via canonical generators, port `prf_key.rs` as-is); **B**
   redesign packing on current params (bit- or byte-per-coefficient; new proof surface in the
   share-encryption circuit, unvalidated). The user approved option A using the parameter search,
   without waiting for final production parameters. The resulting t=2q parameter pairs and the
   packing module are installed and tested; see the progress log.

2. Does the DKG-time PRF key transport require new durable events (schema bump) or reuse existing
   threshold-share events?
3. Committee/preset scope for the first cut: `insecure` only for dev-loop speed, `secure` gated
   behind circuit build times?
4. Deployment sequencing for the new `BfvDecryptionVerifier` (fresh deployment vs upgrade path).

## Progress log

- 2026-10-21: fhe.rs bumped to v0.5.1 with working `[patch]` for `e3-safe` (branch
  `feat/sync-decryption` off `origin/main` @ 354af6bef). v0.5.1 has no legacy decryption path, so
  the `e3-trbfv` migration to `SynchronizedDecryptor` is unavoidable and in progress (3 call sites).
  Reference copies of `fhe-params/{smudging,prf_key}.rs` staged, not yet wired.
- 2026-10-21: **open question 1 resolved — option A-narrow via the search tool.** Threshold sets
  unchanged; DKG presets re-derived with the new `BfvSearchConfig::second_k_multiple = 2` convention
  (DKG share plaintext modulus = 2 × largest threshold prime). Secure DKG: t =
  1,152,921,504,635,584,514, primes `0x24c2230d75cdc001`,`0x260dfc1463740001`. Insecure DKG: t =
  268,434,178, primes `0x1ffffb01`,`0x1ffffd81`. Both first sets reproduced current threshold
  constants exactly; all 53 `e3-fhe-params` tests pass (reproduce-tests assert the new sets).
  `PRF_KEY_BITS` added to both DKG constant modules (32 insecure / 256 secure). Next: regenerate
  `circuits/lib/src/configs/{secure,insecure}/dkg.nr` (bakes QIS/t/Q_MOD_T) via the canonical
  circuit build flow — preset switch, coordinate before pushing. Note: DKG CRPs change
  (deterministic from seed + params) — dev key material reset, testnet reset boundary applies.
- 2026-10-21 (later): dkg.nr regenerated via `sync-preset` (both presets); `check --committee all`
  passes. NOTE: `SHARE_ENCRYPTION_SCALED_QUOTIENT` flipped true→false in both presets (Δ now below
  the smallest limb; share-encryption circuit takes the direct k1 path). fhe-params `prf_key` +
  `smudging` modules wired in (58 tests pass).
- 2026-10-21 (port round): **Rust sync-dec layer ported.** trbfv: `calculate_decryption_share`
  (SynchronizedDecryptor, fresh noise, PRF keys, witness data e_fresh/e_signed/context_digest),
  `calculate_threshold_decryption` (decryptor-set + digest checks, mask-cancelling
  `open_partial_shares`), `gen_pk_share_and_sk_sss` (e_sm pre-generation removed, mult_depth added),
  `helpers.calculate_error_size` new signature, `ShamirShare::replace_row`,
  `BfvEncryptedShare::encrypt`. events: `FreshDecryptionRound` appended; `DecryptionshareCreated`,
  `DecryptionShareProofSigned`, `ShareDecryptionProofPending` gain `decryptors` (additive durable
  schema; old persisted events without the field will not deserialize — dev/testnet reset applies).
  aggregator: `expected_decryptors` on Collecting, `fresh_decryptor_ids`, fresh-round restarts on
  expulsion/C6-failure (publish FreshDecryptionRound), final request carries decryptors+digest.
  zk-prover: decryptors threaded through PendingShareDecryptionProof → events. evm: error-size call
  adapted. Circuits: `math/prf.nr` + U64 extension + `DS_PRF_KEY`/
  `compute_packed_prf_key_commitment` ported; `nargo compile` clean. keyshare graft in progress
  (subagent). REMAINING: keyshare completion, C6/C7/dkg circuit rework, zk witness/config inputs,
  contracts/verifier regen, real-crypto e2e.
