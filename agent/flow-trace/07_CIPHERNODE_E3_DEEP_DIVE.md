# Ciphernode E3 Deep Dive — Clinical Code-Level Flow

Every step, every decision, every failure path. Code references are `file:line` relative to
`crates/`.

---

## Master Flow — Bird's Eye View

```
 ON-CHAIN                          CIPHERNODE SOFTWARE
 ─────────                         ───────────────────
 Enclave.request()
   │
   ▼
 CommitteeFinalized ──────────────► CiphernodeSelected (per selected node)
                                    │
                          ┌─────────┴─────────┐
                          ▼                   ▼
                    ThresholdKeyshare    PublicKeyAggregator
                    (per node)          (aggregator node)
                          │                   │
                    ┌─────┴─────┐             │
                    │  PHASE 1  │             │
                    │  BFV Key  │             │
                    │  Exchange │             │
                    └─────┬─────┘             │
                          │                   │
                    ┌─────┴─────┐             │
                    │  PHASE 2  │             │
                    │  TrBFV    │             │
                    │  Shares   │             │
                    │  C1-C3    │             │
                    └─────┬─────┘             │
                          │                   │
                    ┌─────┴─────┐             │
                    │  PHASE 3  │             │
                    │  C2/C3    │             │
                    │  Verify   │             │
                    └─────┬─────┘             │
                          │                   │
                    ┌─────┴─────┐       ┌─────┴─────┐
                    │  PHASE 4  │       │  PHASE 5  │
                    │  C4 Dec   │       │  C1 Verify│
                    │  Key Calc │       │  C5 Agg   │
                    └─────┬─────┘       └─────┬─────┘
                          │                   │
                          │         PublicKeyAggregated
                          │                   │
                          ▼                   ▼
 CiphertextOutput ───────► PHASE 6: Threshold Decryption (C6)
 Published                          │
                              ┌─────┴─────┐
                              │  PHASE 7  │
                              │  C6 Verify│  ThresholdPlaintextAggregator
                              │  C7 Agg   │
                              └─────┬─────┘
                                    │
                              PlaintextAggregated
                                    │
                                    ▼
                          publishPlaintextOutput() ──► ON-CHAIN
```

---

## Actors: Who Does What

```
┌──────────────────────────────────────────────────────────────────┐
│                     PER-NODE ACTORS                              │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ThresholdKeyshare          Main DKG state machine               │
│  keyshare/src/threshold_keyshare.rs                              │
│                                                                  │
│  ProofRequestActor          Generates + signs C0-C7 proofs       │
│  zk-prover/src/actors/proof_request.rs                           │
│                                                                  │
│  ProofVerificationActor     Verifies C0 proofs (ECDSA+ZK)        │
│  zk-prover/src/actors/proof_verification.rs                      │
│                                                                  │
│  ShareVerificationActor     Verifies C1-C4/C6 (ECDSA+ZK+gates)   │
│  zk-prover/src/actors/share_verification.rs                      │
│                                                                  │
│  CommitmentConsistencyChecker  Post-ZK cross-circuit checks      │
│  zk-prover/src/actors/commitment_consistency_checker.rs          │
│                                                                  │
│  NodeProofAggregator        Per-node recursive proof folding     │
│  zk-prover/src/actors/node_proof_aggregator.rs                   │
│                                                                  │
│  AccusationManager          Off-chain accusation quorum          │
│  zk-prover/src/actors/accusation_manager.rs                      │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                   AGGREGATOR-ONLY ACTORS                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  PublicKeyAggregator        Collects keyshares, C1, C5, PK agg   │
│  aggregator/src/publickey_aggregator.rs                          │
│                                                                  │
│  ThresholdPlaintextAggregator  Collects C6, threshold decrypt    │
│  aggregator/src/threshold_plaintext_aggregator.rs                │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                      EVM ACTORS                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  EnclaveSolWriter           Publishes PK + plaintext on-chain    │
│  evm/src/enclave_sol_writer.rs                                   │
│                                                                  │
│  SlashingManagerSolWriter   Submits slash proposals on-chain     │
│  evm/src/slashing_manager_sol_writer.rs                          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## Proof Map & Connections

```
                         ┌─────────────────────────────────────────────┐
                         │          PROOF DEPENDENCY GRAPH             │
                         └─────────────────────────────────────────────┘

  ┌─────┐                                           ┌─────┐
  │ C0  │──pk_commitment──────────────────────────►  │ C3a │  (CrossParty, CCC)
  │PkBfv│──pk_commitment──────────────────────────►  │ C3b │  (CrossParty, CCC)
  └─────┘                                           └─────┘
                                                        │
  ┌─────┐                               ┌─────┐        │
  │ C1  │──pk_commitment──────────────►  │ C5  │        │
  │PkGen│                                │PkAgg│        │  All happen during DKG
  └──┬──┘                               └─────┘        │
     │                                                  │
     │  sk_commitment ─────────────────────┐            │
     │  e_sm_commitment ──────────────┐    │            │
     │                                │    │            │
  ┌──┴──┐                            │    │         ┌──┴──┐
  │ C2a │  SkShareComputation        │    │         │ C2b │  ESmShareComputation
  └─────┘                            │    │         └─────┘
                                     │    │
  ┌─────┐                            │    │         ┌─────┐
  │ C4b │──e_sm_commitment───────────┘    │         │ C4a │──sk_commitment────┘
  │ ESM │                     (gate)      │         │ SK  │              (gate)
  └─────┘                                │         └─────┘
     │                                   │            │
     │         ┌─────┐                   │            │
     └────────►│ C6  │◄─────────────────┘─────────────┘
               │ThDec│  expected_sk_commitment, expected_e_sm_commitment
               └──┬──┘
                  │
               ┌──┴──┐
               │ C7  │  DecryptedSharesAggregation
               │ Agg │
               └─────┘
```

### Public Signals Layout

```
C0 PkBfv:
  ┌─────────────────────────┬──────────────────┐
  │ public inputs (varies)  │ pk_commitment    │ ◄── OUTPUT (TAIL)
  └─────────────────────────┴──────────────────┘

C1 PkGeneration:
  ┌──────────────────┬──────────────────┬────────────────────┐
  │ sk_commitment    │ pk_commitment    │ e_sm_commitment    │ ◄── ALL OUTPUTS (no inputs)
  └──────────────────┴──────────────────┴────────────────────┘

C3 ShareEncryption:
  ┌───────────────────────────┬──────────────────────────────┐
  │ expected_pk_commitment    │ expected_message_commitment   │ ◄── ALL INPUTS (no outputs)
  └───────────────────────────┴──────────────────────────────┘

C6 ThresholdShareDecryption:
  ┌───────────────────────────┬──────────────────────────────┬──────────────────┐
  │ expected_sk_commitment    │ expected_e_sm_commitment     │ d_commitment     │
  └───────────────────────────┴──────────────────────────────┴──────────────────┘
    INPUT (HEAD)                INPUT (HEAD)                   OUTPUT (TAIL)

C5 PkAggregation:
  ┌─────────────────────────────────────────────────┬──────────────────┐
  │ pk_commitments[0] ... pk_commitments[H-1]       │ commitment       │
  └─────────────────────────────────────────────────┴──────────────────┘
    H INPUT FIELDS (HEAD)                             OUTPUT (TAIL)
```

---

## PHASE 1: BFV Key Exchange

### Step 1.1: Node Selected

```
CiphernodeSelected event
         │
         ▼
┌─────────────────────────────────────────────┐
│ handle_ciphernode_selected()                │
│ keyshare/src/threshold_keyshare.rs:738      │
├─────────────────────────────────────────────┤
│                                             │
│  Is state == Init?                          │
│     │                                       │
│     ├── NO ──► ignore event, return         │
│     │                                       │
│     ▼ YES                                   │
│  Generate BFV keypair:                      │
│    sk = SecretKey::random()     (line 751)  │
│    pk = PublicKey::new(&sk)     (line 752)  │
│                                             │
│  Encrypt sk locally:                        │
│    sk_bfv = SensitiveBytes     (line 754)   │
│                                             │
│  Publish EncryptionKeyPending               │
│    { pk_bfv bytes }            (line 771)   │
│                                             │
│  STATE: Init ──► CollectingEncryptionKeys   │
│                                             │
│  FAILURE MODES:                             │
│   • BFV key generation panics              │
│     → node crashes, doesn't participate     │
│   • Already past Init                       │
│     → event silently ignored                │
└─────────────────────────────────────────────┘
```

### Step 1.2: C0 Proof Generation

```
EncryptionKeyPending event
         │
         ▼
┌─────────────────────────────────────────────────┐
│ ProofRequestActor                               │
│ zk-prover/src/actors/proof_request.rs           │
├─────────────────────────────────────────────────┤
│                                                 │
│  Build C0 proof request                         │
│  Publish ComputeRequest::zk(PkBfv proof)        │
│         │                                       │
│         ▼                                       │
│  ZK multithread generates C0 proof              │
│         │                                       │
│         ├── SUCCESS                              │
│         │     Sign proof: SignedProofPayload     │
│         │       ProofType::C0PkBfv              │
│         │     Publish EncryptionKeyCreated       │
│         │       { pk, signed_c0_proof }          │
│         │     Also: DKGInnerProofReady           │
│         │       (for per-node recursive fold)    │
│         │                                       │
│         └── FAILURE                              │
│               Proof generation fails             │
│               → EncryptionKeyCreated published   │
│                 WITHOUT C0 proof (proof=None)     │
│               → other nodes can still collect    │
│                 the key but C0 verify will fail   │
└─────────────────────────────────────────────────┘
```

### Step 1.3: Collecting Encryption Keys

```
EncryptionKeyCreated events arriving from all N nodes
         │
         ▼
┌──────────────────────────────────────────────────────────┐
│ handle_encryption_key_created()                          │
│ keyshare/src/threshold_keyshare.rs:606                   │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  Is sender expelled?                                     │
│     ├── YES ──► ignore, return                           │
│     ▼ NO                                                 │
│                                                          │
│  Add to EncryptionKeyCollector                           │
│     │                                                    │
│     ├── Not all N keys yet ──► wait for more             │
│     │                                                    │
│     ▼ All N keys collected                               │
│  Emit AllEncryptionKeysCollected                         │
│                                                          │
│  CONCURRENT: C0 VERIFICATION (per key, separate actor)   │
│  ┌────────────────────────────────────────────────┐      │
│  │ ProofVerificationActor                         │      │
│  │ zk-prover/src/actors/proof_verification.rs:74  │      │
│  │                                                │      │
│  │ For each EncryptionKeyCreated with C0 proof:   │      │
│  │   │                                            │      │
│  │   ├── ECDSA validation:                        │      │
│  │   │     Recover address from signature         │      │
│  │   │     │                                      │      │
│  │   │     ├── FAIL: bad signature                │      │
│  │   │     │   → SignedProofFailed                │      │
│  │   │     │   → ProofVerificationFailed          │      │
│  │   │     │   → AccusationManager triggered      │      │
│  │   │     │                                      │      │
│  │   │     ▼ PASS                                 │      │
│  │   │                                            │      │
│  │   ├── ZK verification (multithread):           │      │
│  │   │     │                                      │      │
│  │   │     ├── FAIL: circuit violation            │      │
│  │   │     │   → SignedProofFailed                │      │
│  │   │     │   → ProofVerificationFailed          │      │
│  │   │     │   → AccusationManager triggered      │      │
│  │   │     │                                      │      │
│  │   │     ▼ PASS                                 │      │
│  │   │                                            │      │
│  │   ▼                                            │      │
│  │ ProofVerificationPassed emitted                │      │
│  │   → CCC caches (address, C0PkBfv, signals)    │      │
│  │   → AccusationManager caches as passed         │      │
│  │   → Used later for C0→C3 link check            │      │
│  └────────────────────────────────────────────────┘      │
│                                                          │
│  FAILURE MODES:                                          │
│   • Member expelled during collection                    │
│     → removed from collector, threshold_n decremented    │
│     → if remaining < threshold needed → E3Failed         │
│   • Timeout (collector has deadline)                     │
│     → EncryptionKeyCollectionFailed                      │
│     → E3Failed published                                 │
│   • No C0 proof attached                                 │
│     → key still collected (C0 verify skipped)            │
│     → party may be flagged later by CCC                  │
└──────────────────────────────────────────────────────────┘
```

### Step 1.4: Transition to Share Generation

```
AllEncryptionKeysCollected
         │
         ▼
┌──────────────────────────────────────────────────────┐
│ handle_all_encryption_keys_collected()               │
│ keyshare/src/threshold_keyshare.rs:784               │
├──────────────────────────────────────────────────────┤
│                                                      │
│  STATE: CollectingEncryptionKeys                     │
│         ──► GeneratingThresholdShare                 │
│                                                      │
│  Publish ComputeRequest::trbfv(GenPkShareAndSkSss)   │
│    Contains:                                         │
│    • All collected BFV public keys                   │
│    • TrBFV config (threshold params)                 │
│    • CRP seed for deterministic randomness           │
│                                                      │
│  FAILURE MODES:                                      │
│   • TrBFV computation fails                          │
│     → ComputeRequestError                            │
│     → node stuck in GeneratingThresholdShare         │
│     → eventually times out on-chain                  │
└──────────────────────────────────────────────────────┘
```

---

## PHASE 2: Share Computation & Proof Generation

### Step 2.1: TrBFV Shares Generated

```
ComputeResponse(GenPkShareAndSkSss)
         │
         ▼
┌──────────────────────────────────────────────────────┐
│ handle_gen_pk_share_and_sk_sss_response()            │
│ keyshare/src/threshold_keyshare.rs:877               │
├──────────────────────────────────────────────────────┤
│                                                      │
│  Extracts:                                           │
│    pk_share   — this node's public key share         │
│    sk_sss     — Shamir shares of secret key          │
│                 (one share per recipient node)        │
│    esi_sss    — Shamir shares of smudging noise      │
│                 (per ESI index, per recipient)        │
│                                                      │
│  Stores locally (encrypted at rest)                  │
│                                                      │
│  Triggers share encryption + proof generation        │
│  (see Step 2.2)                                      │
│                                                      │
│  FAILURE MODES:                                      │
│   • Response contains error                          │
│     → node cannot proceed                            │
│     → on-chain timeout will trigger E3Failed         │
└──────────────────────────────────────────────────────┘
```

### Step 2.2: Encrypt Shares & Build Proof Requests

```
handle_shares_generated()
keyshare/src/threshold_keyshare.rs:1012
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│  1. Decrypt sk_sss and esi_sss from local storage  (line 1055-1059)      │
│                                                                          │
│  2. For each recipient node j (j ≠ self):                                │
│     Encrypt sk_share[j] using recipient's BFV public key                 │
│     Encrypt esi_share[j][k] for each ESI index k                        │
│     Capture encryption witnesses (u_rns, e0_rns, e1_rns)                │
│                                                   (line 1078-1092)       │
│                                                                          │
│  3. Build proof requests:                                                │
│                                                                          │
│     ┌─────────────────────────────────────────────────────────────┐      │
│     │ PROOF REQUESTS CREATED                                      │      │
│     ├─────────────────────────────────────────────────────────────┤      │
│     │                                                             │      │
│     │ C1: PkGenerationProofRequest                   × 1          │      │
│     │     Proves: TrBFV key generation correct                    │      │
│     │     Outputs: (sk_commitment, pk_commitment, e_sm_commitment)│      │
│     │                                                             │      │
│     │ C2a: ShareComputationProofRequest(SecretKey)    × 1          │      │
│     │     Proves: Shamir SSS of secret key correct                │      │
│     │     Outputs: (key_hash, commitment)                         │      │
│     │                                                             │      │
│     │ C2b: ShareComputationProofRequest(SmudgingNoise) × 1        │      │
│     │     Proves: Shamir SSS of smudging noise correct            │      │
│     │     Outputs: (key_hash, commitment)                         │      │
│     │                                                             │      │
│     │ C3a: ShareEncryptionProofRequest               × L_sk*(N-1) │      │
│     │     Proves: BFV encryption of SK share correct              │      │
│     │     Inputs: (expected_pk_commitment, expected_msg_commitment)│      │
│     │     L_sk = num_moduli_sk, N-1 = other nodes                 │      │
│     │                                                             │      │
│     │ C3b: ShareEncryptionProofRequest               × L_esm*E*(N-1)    │
│     │     Proves: BFV encryption of ESM share correct             │      │
│     │     L_esm = num_moduli_esi, E = num_esi                    │      │
│     │     Inputs: (expected_pk_commitment, expected_msg_commitment)│      │
│     │                                                             │      │
│     │ TOTAL per node = 3 + L_sk*(N-1) + L_esm*E*(N-1)            │      │
│     └─────────────────────────────────────────────────────────────┘      │
│                                                                          │
│  4. All requests sent to ProofRequestActor                              │
│     → ComputeRequest::zk per proof                                       │
│     → ZK multithread generates each proof                                │
│     → On completion: sign with ECDSA                                     │
│     → Bundle into ThresholdShareCreated event per recipient              │
│                                                                          │
│  FAILURE MODES:                                                          │
│   • Decryption of local shares fails                                     │
│     → node crashes or returns error                                      │
│   • BFV encryption fails for a recipient                                 │
│     → share not sent to that recipient                                   │
│   • ZK proof generation fails for any proof                             │
│     → proof missing from ThresholdShareCreated                           │
│     → recipient marks sender as incomplete (pre_dishonest)               │
└──────────────────────────────────────────────────────────────────────────┘
```

### Step 2.3: ThresholdShareCreated Broadcast

```
┌──────────────────────────────────────────────────────────────┐
│ ThresholdShareCreated event (one per recipient)              │
│ events/src/enclave_event/threshold_share_created.rs          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Payload:                                                    │
│  ┌────────────────────────────────────────────────────┐      │
│  │ e3_id: E3id                                        │      │
│  │ share: Arc<ThresholdShare>  (encrypted share data) │      │
│  │ target_party_id: u64       (recipient)             │      │
│  │ signed_c2a_proof: Option<SignedProofPayload>       │      │
│  │ signed_c2b_proof: Option<SignedProofPayload>       │      │
│  │ signed_c3a_proofs: Vec<SignedProofPayload>         │      │
│  │ signed_c3b_proofs: Vec<SignedProofPayload>         │      │
│  └────────────────────────────────────────────────────┘      │
│                                                              │
│  Sent via P2P gossip to the recipient node                   │
│                                                              │
│  CONCURRENT: Each proof is also wrapped + emitted as         │
│  DKGInnerProofReady for NodeProofAggregator folding          │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## PHASE 3: Share Collection & C2/C3 Verification

### Step 3.1: Collecting All Shares

```
ThresholdShareCreated events from all (N-1) other nodes
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ ThresholdShareCollector                                      │
│ Collects until all N-1 shares received or timeout            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  For each arriving share:                                    │
│    Is sender expelled? ──YES──► ignore                       │
│    Already received from sender? ──YES──► ignore             │
│    Add to collection                                         │
│                                                              │
│  When all (N-1) shares collected:                            │
│    → AllThresholdSharesCollected                             │
│                                                              │
│  FAILURE MODES:                                              │
│   • Timeout before all shares arrive                         │
│     → ThresholdShareCollectionFailed                         │
│     → E3Failed(InsufficientCommitteeMembers)                 │
│     → ThresholdKeyshare actor stopped                        │
│                                                              │
│   • Member expelled during collection                        │
│     → removed from expected set                              │
│     → if remaining collected = new expected → complete       │
│     → if too few remain → E3Failed                           │
└──────────────────────────────────────────────────────────────┘
```

### Step 3.2: Pre-Dishonest Classification

```
AllThresholdSharesCollected
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ handle_all_threshold_shares_collected()                      │
│ keyshare/src/threshold_keyshare.rs:1225                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  For each party's share, check proof completeness:           │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │              PROOF COMPLETENESS CHECK               │     │
│  │                                                     │     │
│  │  Has C2a proof?  ──NO──► no_proof_parties           │     │
│  │  Has C2b proof?  ──NO──► no_proof_parties           │     │
│  │                                                     │     │
│  │  C3a count == expected?                             │     │
│  │    expected = own share's num_moduli_sk             │     │
│  │    ──NO──► incomplete_proof_parties                  │     │
│  │                                                     │     │
│  │  C3b count == expected?                             │     │
│  │    expected = own share's num_esi * num_moduli_esi  │     │
│  │    ──NO──► incomplete_proof_parties                  │     │
│  └─────────────────────────────────────────────────────┘     │
│                                                              │
│  pre_dishonest = no_proof_parties ∪ incomplete_proof_parties │
│                                                              │
│  Are ALL parties pre_dishonest? (no proofs to verify at all) │
│     │                                                        │
│     ├── YES: honest_count = N - |pre_dishonest|              │
│     │        honest_count > threshold_m?                     │
│     │          ├── NO ──► E3Failed(InsufficientCommittee)    │
│     │          ▼ YES                                         │
│     │        Skip verification, go directly to               │
│     │        proceed_with_decryption_key_calculation()        │
│     │                                                        │
│     ▼ NO (some parties have proofs)                          │
│  Dispatch C2/C3 verification (Step 3.3)                      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Step 3.3: C2/C3 Verification (ShareVerificationActor)

```
ShareVerificationDispatched { kind: ShareProofs, share_proofs, pre_dishonest }
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ ShareVerificationActor::verify_proofs()                                  │
│ zk-prover/src/actors/share_verification.rs:269                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  FOR EACH party in share_proofs:                                         │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐     │
│  │  Is party in pre_dishonest?                                     │     │
│  │     ├── YES ──► SKIP entirely (no ECDSA, no ZK)  (line 290)    │     │
│  │     │           No events emitted.                              │     │
│  │     │           Pre_dishonest parties have no proof payload     │     │
│  │     │           to build fault evidence from.                   │     │
│  │     │                                                           │     │
│  │     ▼ NO                                                        │     │
│  │                                                                 │     │
│  │  ECDSA VALIDATION (inline, line 400-468):                       │     │
│  │  ┌───────────────────────────────────────────────────────┐      │     │
│  │  │ For each SignedProofPayload in party's proofs:        │      │     │
│  │  │                                                       │      │     │
│  │  │  1. e3_id matches? ──NO──► ECDSA FAIL                │      │     │
│  │  │  2. Signature recovery succeeds?                      │      │     │
│  │  │     ──NO──► ECDSA FAIL                                │      │     │
│  │  │  3. All proofs from same address (signer consistency)?│      │     │
│  │  │     ──NO──► ECDSA FAIL                                │      │     │
│  │  │  4. Circuit name matches ProofType?                   │      │     │
│  │  │     ──NO──► ECDSA FAIL                                │      │     │
│  │  │                                                       │      │     │
│  │  │  ECDSA FAIL:                                          │      │     │
│  │  │    → party added to ecdsa_dishonest                   │      │     │
│  │  │    → emit_signed_proof_failed() called                │      │     │
│  │  │      → SignedProofFailed event                        │      │     │
│  │  │      → ProofVerificationFailed event                  │      │     │
│  │  │      → AccusationManager picks up PVF                │      │     │
│  │  │                                                       │      │     │
│  │  │  ECDSA PASS:                                          │      │     │
│  │  │    → party added to ecdsa_passed_parties              │      │     │
│  │  └───────────────────────────────────────────────────────┘      │     │
│  └─────────────────────────────────────────────────────────────────┘     │
│                                                                          │
│  All parties ECDSA-failed?                                               │
│     ├── YES ──► all_dishonest = pre_dishonest ∪ ecdsa_dishonest          │
│     │           publish_complete() immediately                           │
│     │           (no ZK needed)                                           │
│     │                                                                    │
│     ▼ NO (at least one party passed ECDSA)                               │
│                                                                          │
│  Recover addresses for ECDSA-passed parties  (line 307-314)              │
│  Compute proof hashes + cache public_signals  (line 329-360)             │
│                                                                          │
│  DISPATCH ZK VERIFICATION (multithread):                                 │
│  ComputeRequest::zk(VerifyShareProofs { ecdsa_passed_parties })          │
│    → only ECDSA-passed parties are ZK-verified                           │
│    → pre_dishonest and ecdsa_dishonest are NOT sent to ZK                │
│                                                                          │
│  Store in self.pending:                                                   │
│    PendingVerification { e3_id, kind, ec, ecdsa_dishonest,               │
│      pre_dishonest, dispatched_party_ids, party_addresses,               │
│      party_proof_hashes, party_public_signals, party_signed_proofs }     │
│                                                                          │
│  FAILURE MODES:                                                          │
│   • ComputeRequest publish fails                                         │
│     → all dispatched parties treated as dishonest                        │
│     → publish_complete() with everyone dishonest                         │
│   • ZK multithread crashes                                               │
│     → ComputeRequestError received                                       │
│     → all dispatched parties treated as dishonest                        │
└──────────────────────────────────────────────────────────────────────────┘
```

### Step 3.4: ZK Results Processing

```
ComputeResponse arrives with ZK results
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ handle_compute_response()                                                │
│ zk-prover/src/actors/share_verification.rs:471                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Build all_dishonest = pre_dishonest ∪ ecdsa_dishonest                   │
│                                                                          │
│  CROSS-CHECK: every dispatched party must appear in results              │
│    Missing party? → add to all_dishonest (defense-in-depth)  (line 507) │
│                                                                          │
│  FOR EACH ZK result:                                                     │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │  Party not in dispatched_party_ids?                              │    │
│  │     → IGNORE (spurious result, defense-in-depth)  (line 519)    │    │
│  │                                                                  │    │
│  │  result.all_verified == false?  (ZK FAILED)                      │    │
│  │     │                                                            │    │
│  │     ▼                                                            │    │
│  │     Add to all_dishonest                         (line 530)      │    │
│  │     emit_signed_proof_failed()                   (line 538-544)  │    │
│  │       → SignedProofFailed event                                  │    │
│  │       → ProofVerificationFailed event                            │    │
│  │       → AccusationManager picks up PVF                          │    │
│  │                                                                  │    │
│  │  result.all_verified == true?  (ZK PASSED)                       │    │
│  │     │                                                            │    │
│  │     ├── Party in all_dishonest already?                          │    │
│  │     │     ├── YES → SUPPRESS ProofVerificationPassed  (line 546)│    │
│  │     │     │         Log warning (should be unreachable)          │    │
│  │     │     │                                                      │    │
│  │     │     ▼ NO                                                   │    │
│  │     │   Emit ProofVerificationPassed per proof type  (line 548) │    │
│  │     │     → CCC caches for commitment link checks                │    │
│  │     │     → AccusationManager caches as passed                  │    │
│  │     │                                                            │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  CACHE C4 SIGNALS (if kind == DecryptionProofs):                         │
│    For each ZK-passed honest party:                                      │
│      c4_signals_cache[e3_id][party_id] = public_signals  (line 597)     │
│    (used later by C4→C6 gate)                                            │
│                                                                          │
│  EVICT C4 CACHE (if kind == ThresholdDecryptionProofs):                  │
│    c4_signals_cache.remove(e3_id)                        (line 595)     │
│    (cache no longer needed after C6 verification)                        │
│                                                                          │
│  publish_complete(e3_id, kind, all_dishonest)                            │
│    → ShareVerificationComplete event                                     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Step 3.5: Post-Verification Threshold Check

```
ShareVerificationComplete { kind: ShareProofs, dishonest_parties }
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ handle_share_verification_complete(ShareProofs)              │
│ keyshare/src/threshold_keyshare.rs:1395                      │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  honest_parties = all_parties - dishonest_parties            │
│                                                              │
│  honest_count > threshold_m?                                 │
│     │                                                        │
│     ├── NO ──► E3Failed(InsufficientCommitteeMembers)        │
│     │          Actor stops.                                  │
│     │                                                        │
│     ▼ YES                                                    │
│  proceed_with_decryption_key_calculation()                    │
│  (PHASE 4)                                                   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## PHASE 4: Decryption Key Computation & C4 Proofs

### Step 4.1: Decrypt Received Shares & Validate Dimensions

```
proceed_with_decryption_key_calculation()
keyshare/src/threshold_keyshare.rs:1501
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│  1. Filter to honest parties only                (line 1523-1531)        │
│                                                                          │
│  2. FOR EACH honest party's share:               (line 1533-1561)        │
│     ┌──────────────────────────────────────────────────────────┐         │
│     │ Validate dimensions against own share (trusted source):  │         │
│     │   ESI count matches?    ──NO──► exclude party             │         │
│     │   SK moduli count?      ──NO──► exclude party             │         │
│     │   ESM moduli count?     ──NO──► exclude party             │         │
│     │                                                          │         │
│     │ On exclusion:                                            │         │
│     │   → log warning with party_id + mismatch details         │         │
│     │   → party removed from honest set                        │         │
│     └──────────────────────────────────────────────────────────┘         │
│                                                                          │
│  3. RE-CHECK THRESHOLD after exclusions          (line 1638-1650)        │
│     remaining > threshold_m?                                             │
│       ├── NO ──► E3Failed(InsufficientCommitteeMembers)                  │
│       ▼ YES                                                              │
│                                                                          │
│  4. Collect ciphertexts for C4 proofs            (line 1670-1699)        │
│     C4a: SK share ciphertexts from honest parties                        │
│     C4b: ESM share ciphertexts per smudging noise index                  │
│                                                                          │
│  5. Decrypt all shares using own BFV secret key  (line 1702-1740)        │
│                                                                          │
│  6. Compute decryption key aggregation                                   │
│     ComputeRequest::trbfv(CalculateDecryptionKey)  (line 1742)           │
│                                                                          │
│  7. Build C4 proof requests                      (line 1759-1784)        │
│     sk_request: DkgShareDecryptionProofRequest    (for C4a)              │
│     esm_requests: Vec<DkgShareDecryptionProofRequest>  (for C4b)         │
│                                                                          │
│  FAILURE MODES:                                                          │
│   • All parties excluded by dimension check                              │
│     → E3Failed                                                           │
│   • BFV decryption fails                                                 │
│     → error, node cannot continue                                        │
│   • Compute request fails                                                │
│     → ComputeRequestError, node stuck                                    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Step 4.2: C4 Proof Generation, Signing, Broadcast

```
DecryptionShareProofsPending
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ ProofRequestActor::handle_decryption_share_proofs_pending()  │
│ zk-prover/src/actors/proof_request.rs:550                    │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Generate C4a (SK decryption proof):                         │
│    ComputeRequest::zk(DkgShareDecryption(sk_request))        │
│    ProofType: C4aSkShareDecryption                           │
│    Output: commitment (in public_signals tail)               │
│                                                              │
│  Generate C4b (ESM decryption proofs):                       │
│    FOR EACH ESI index:                                       │
│      ComputeRequest::zk(DkgShareDecryption(esm_request[i])) │
│      ProofType: C4bESmShareDecryption                        │
│                                                              │
│  On all complete:                                            │
│    Sign each proof with ECDSA                                │
│    Publish DecryptionKeyShared event:                         │
│      { party_id, node,                                       │
│        signed_sk_decryption_proof,                           │
│        signed_e_sm_decryption_proofs: Vec }                  │
│                                                              │
│  Also: DKGInnerProofReady for recursive fold                 │
│                                                              │
│  FAILURE MODES:                                              │
│   • Any ZK proof generation fails                            │
│     → that proof missing from DecryptionKeyShared            │
│     → recipient marks sender's proof count wrong             │
│     → sender becomes pre_dishonest                           │
│   • Signing fails                                            │
│     → same result                                            │
└──────────────────────────────────────────────────────────────┘
```

### Step 4.3: C4 Collection & Verification

```
DecryptionKeyShared events from all honest parties
         │
         ▼
┌──────────────────────────────────────────────────────────────┐
│ DecryptionKeySharedCollector                                 │
│                                                              │
│  Collect until all honest parties' C4 proofs received        │
│     │                                                        │
│     ├── Timeout ──► DecryptionKeySharedCollectionFailed       │
│     │               → E3Failed, actor stopped                │
│     │                                                        │
│     ▼ All collected                                          │
│  AllDecryptionKeySharesCollected                             │
│     │                                                        │
│     ▼                                                        │
│  dispatch_c4_verification()                                  │
│  keyshare/src/threshold_keyshare.rs:1929                     │
│     │                                                        │
│     ▼                                                        │
│  PRE-VALIDATION (line 1944-1979):                            │
│    For each party:                                           │
│      ESM proof count == expected?                            │
│        ──NO──► pre_dishonest                                 │
│                                                              │
│    Still enough honest? ──NO──► E3Failed                     │
│                                                              │
│  ShareVerificationDispatched { kind: DecryptionProofs }      │
│    → Same ECDSA + ZK flow as C2/C3 (Step 3.3-3.4)           │
│    → C4 public_signals cached on ZK pass (for C4→C6 gate)   │
│                                                              │
│  ShareVerificationComplete { kind: DecryptionProofs }        │
│    → Update honest set, threshold check                      │
│    → Publish KeyshareCreated (Exchange #4)                   │
│      { pk_share, signed_pk_generation_proof (C1) }           │
└──────────────────────────────────────────────────────────────┘
```

---

## PHASE 5: Public Key Aggregation (Aggregator)

### Step 5.1-5.7: Complete Aggregation Flow

```
KeyshareCreated events from all N nodes
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ PublicKeyAggregator                                                      │
│ aggregator/src/publickey_aggregator.rs                                    │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  STATE: Collecting                                                       │
│                                                                          │
│  Collect KeyshareCreated { pk_share, C1 proof } from each node           │
│  Track insertion order → party_id mapping                                │
│                                                                          │
│  All N keyshares received?                                               │
│     ├── NO ──► wait (or member expelled → remove + re-check)            │
│     ▼ YES                                                                │
│                                                                          │
│  STATE: Collecting ──► VerifyingC1                                       │
│                                                                          │
│  Dispatch C1 verification:                                               │
│    ShareVerificationDispatched { kind: PkGenerationProofs }              │
│    → Same ECDSA + ZK flow                                                │
│         │                                                                │
│         ▼                                                                │
│  ShareVerificationComplete { kind: PkGenerationProofs }                  │
│         │                                                                │
│         ▼                                                                │
│  handle_c1_verification_complete()                    (line 232)         │
│                                                                          │
│  POST-C1 COMMITMENT CROSS-CHECK (line 271-309):                         │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │ For each ZK-passed party:                                        │    │
│  │   1. Compute pk_commitment from raw keyshare bytes               │    │
│  │      (hash the polynomial coefficients)                          │    │
│  │   2. Extract pk_commitment from C1 proof's public_signals        │    │
│  │      (field index 1 in PK_GENERATION_OUTPUTS)                    │    │
│  │   3. Compare:                                                    │    │
│  │      MATCH    → party stays honest                               │    │
│  │      MISMATCH → add to dishonest_parties                        │    │
│  │                 emit SignedProofFailed                            │    │
│  │                 (keyshare doesn't match what C1 proves)          │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Re-filter to honest entries after commitment check  (line 341)          │
│                                                                          │
│  honest_keyshares.len() > threshold_m?               (line 361)          │
│     ├── NO ──► E3Failed                                                  │
│     ▼ YES                                                                │
│                                                                          │
│  AGGREGATE PUBLIC KEY:                                                   │
│    fhe.get_aggregate_public_key(honest_keyshares)    (line 384)          │
│                                                                          │
│  STATE: VerifyingC1 ──► GeneratingC5Proof                                │
│                                                                          │
│  Emit PkAggregationProofPending (triggers C5 proof gen)                  │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │ C5 PROOF + CROSS-NODE FOLD (concurrent)                          │    │
│  │                                                                  │    │
│  │ C5 Proof:                                                        │    │
│  │   ProofRequestActor generates PkAggregation proof                │    │
│  │   Public signals: [pk_commitments[0..H], pk_agg_commitment]      │    │
│  │   Sign → PkAggregationProofSigned                                │    │
│  │                                                                  │    │
│  │ Cross-Node Fold:                                                 │    │
│  │   Collect DKGRecursiveAggregationComplete from each honest party │    │
│  │   Fold all per-node proofs into single cross-node proof          │    │
│  │   (or skip if proof aggregation disabled)                        │    │
│  │                                                                  │    │
│  │ BOTH MUST COMPLETE before publishing                             │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  try_publish_complete()                              (line 639)          │
│    Preconditions: C5 signed AND fold done                                │
│                                                                          │
│  Publish PublicKeyAggregated:                                            │
│    { pubkey, nodes, pk_aggregation_proof, dkg_aggregated_proof }         │
│                                                                          │
│  STATE: GeneratingC5Proof ──► Complete                                   │
│                                                                          │
│  ON-CHAIN: EnclaveSolWriter publishes the aggregated PK                  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## PHASE 6: Threshold Decryption (C6)

```
CiphertextOutputPublished (on-chain event, computation is done)
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ handle_ciphertext_output_published()                                     │
│ keyshare/src/threshold_keyshare.rs:2056                                  │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  STATE: ReadyForDecryption ──► Decrypting                                │
│                                                                          │
│  Store ciphertext bytes                                                  │
│  ComputeRequest::trbfv(CalculateDecryptionShare)                         │
│    Uses: sk_poly_sum, es_poly_sum, ciphertexts                           │
│         │                                                                │
│         ▼                                                                │
│  ComputeResponse → d_share_poly (decryption share polynomial)            │
│         │                                                                │
│         ▼                                                                │
│  STATE: Decrypting ──► GeneratingDecryptionProof                         │
│                                                                          │
│  Publish ShareDecryptionProofPending                                     │
│    → ProofRequestActor generates C6 proof                                │
│    → ProofType: C6ThresholdShareDecryption                               │
│    → Public inputs: expected_sk_commitment, expected_e_sm_commitment     │
│    → Public output: d_commitment                                         │
│    → Sign with ECDSA                                                     │
│         │                                                                │
│         ▼                                                                │
│  Publish DecryptionshareCreated:                                         │
│    { party_id, decryption_share, signed_decryption_proofs,               │
│      wrapped_proofs (for cross-node fold) }                              │
│                                                                          │
│  FAILURE MODES:                                                          │
│   • Not in ReadyForDecryption state                                      │
│     → event ignored (wrong lifecycle phase)                              │
│   • Decryption share computation fails                                   │
│     → ComputeRequestError, node stuck                                    │
│   • C6 proof generation fails                                           │
│     → DecryptionshareCreated not published                               │
│     → aggregator won't receive this node's share                        │
│     → may still succeed if enough other shares                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## PHASE 7: Plaintext Reconstruction (Aggregator)

```
DecryptionshareCreated events from all N nodes
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ ThresholdPlaintextAggregator                                             │
│ aggregator/src/threshold_plaintext_aggregator.rs                         │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  STATE: Collecting                                                       │
│  Collect shares + C6 proofs until all N received                         │
│                                                                          │
│  STATE: Collecting ──► VerifyingC6                                       │
│                                                                          │
│  ══════════════════════════════════════════════════════════════           │
│  ║              C4→C6 GATE (PRE-VERIFICATION)                ║           │
│  ║  share_verification.rs:164-225                            ║           │
│  ║                                                           ║           │
│  ║  Before dispatching ZK, for EACH party:                   ║           │
│  ║                                                           ║           │
│  ║  Has cached C4 signals for this party?                    ║           │
│  ║     │                                                     ║           │
│  ║     ├── NO ──► skip gate, ZK will still verify            ║           │
│  ║     │          (no cached data = can't compare)           ║           │
│  ║     │                                                     ║           │
│  ║     ▼ YES                                                 ║           │
│  ║                                                           ║           │
│  ║  Is party already pre_dishonest?                          ║           │
│  ║     ├── YES ──► skip (already flagged)                    ║           │
│  ║     ▼ NO                                                  ║           │
│  ║                                                           ║           │
│  ║  Check C4a→C6: sk_commitment                             ║           │
│  ║    Extract sk_commitment from C4a public_signals          ║           │
│  ║    Extract expected_sk_commitment from C6 public_signals  ║           │
│  ║    Match? ──NO──► mismatch = true                         ║           │
│  ║                                                           ║           │
│  ║  Check C4b→C6: e_sm_commitment                           ║           │
│  ║    Extract e_sm_commitment from C4b public_signals        ║           │
│  ║    Extract expected_e_sm_commitment from C6 pub signals   ║           │
│  ║    Match? ──NO──► mismatch = true                         ║           │
│  ║                                                           ║           │
│  ║  mismatch == true?                                        ║           │
│  ║     │                                                     ║           │
│  ║     ├── YES:                                              ║           │
│  ║     │   pre_dishonest.insert(party_id)                    ║           │
│  ║     │   emit_signed_proof_failed()                        ║           │
│  ║     │     → SignedProofFailed                             ║           │
│  ║     │     → ProofVerificationFailed                       ║           │
│  ║     │     → AccusationManager triggered                  ║           │
│  ║     │                                                     ║           │
│  ║     ▼ NO: party passes gate                               ║           │
│  ║                                                           ║           │
│  ══════════════════════════════════════════════════════════════           │
│                                                                          │
│  ShareVerificationDispatched { kind: ThresholdDecryptionProofs }          │
│    → pre_dishonest parties skip ECDSA + ZK entirely                      │
│    → Same ECDSA + ZK flow for remaining parties                          │
│    → c4_signals_cache EVICTED after verification  (line 595)             │
│         │                                                                │
│         ▼                                                                │
│  ShareVerificationComplete { kind: ThresholdDecryptionProofs }           │
│         │                                                                │
│         ▼                                                                │
│  handle_c6_verification_complete()                   (line 292)          │
│                                                                          │
│  Filter to honest shares only                        (line 323-328)      │
│  honest_shares.len() > threshold_m?                  (line 330-335)      │
│     ├── NO ──► error (cannot compute plaintext)                          │
│     ▼ YES                                                                │
│                                                                          │
│  STATE: VerifyingC6 ──► Computing                                        │
│                                                                          │
│  ComputeRequest::trbfv(CalculateThresholdDecryption)                     │
│    Uses only honest shares                                               │
│         │                                                                │
│         ▼                                                                │
│  ComputeResponse → plaintext                                             │
│                                                                          │
│  STATE: Computing ──► GeneratingC7Proof                                  │
│                                                                          │
│  C7 proof generation (ProofRequestActor)                                 │
│    Proves: correct plaintext reconstruction                              │
│         │                                                                │
│         ▼                                                                │
│  try_publish_complete()                              (line 536)          │
│    Preconditions: C7 proofs ready AND C6 fold done                       │
│         │                                                                │
│         ▼                                                                │
│  PlaintextAggregated event:                                              │
│    { decrypted_output, aggregation_proofs (C7),                          │
│      c6_aggregated_proof (cross-node fold) }                             │
│         │                                                                │
│         ▼                                                                │
│  EnclaveSolWriter → publishPlaintextOutput() on-chain                    │
│                                                                          │
│  STATE: GeneratingC7Proof ──► Complete                                   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## ACCUSATION & SLASHING FLOW

```
ProofVerificationFailed event
(emitted by ShareVerificationActor or ProofVerificationActor)
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ AccusationManager::on_local_proof_failure()                               │
│ zk-prover/src/actors/accusation_manager.rs:314                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. Resolve accused address (from event or committee lookup)             │
│  2. Cache in received_data[(accused, proof_type)] = { hash, failed }     │
│  3. Already accused for (accused, proof_type)? ──YES──► return (dedup)  │
│  4. Create ProofFailureAccusation:                                       │
│       accusation_id = keccak256(chainId, e3_id, accused, proof_type)     │
│       Sign with ECDSA (matches Solidity typehash)                        │
│  5. Broadcast via P2P gossip                                             │
│  6. Cast own vote: agrees=true, signed                                   │
│  7. Start 300s timeout                                                   │
│  8. check_quorum() immediately (in case threshold_m == 1)                │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
           P2P gossip to other committee members
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ AccusationManager::on_accusation_received()                              │
│ zk-prover/src/actors/accusation_manager.rs:435                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Verify accuser is committee member     ──FAIL──► ignore                 │
│  Verify accused is committee member     ──FAIL──► ignore                 │
│  Is it my own accusation?               ──YES──► ignore (already voted)  │
│  Verify ECDSA signature on accusation   ──FAIL──► ignore                 │
│  Already pending for this accusation_id? ──YES──► ignore (dedup)         │
│                                                                          │
│  DETERMINE VOTE:                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐    │
│  │                                                                  │    │
│  │  Have local cache for (accused, proof_type)?                     │    │
│  │     │                                                            │    │
│  │     ├── YES, verification_passed == false                        │    │
│  │     │   → vote agrees=true (our verification also failed)        │    │
│  │     │                                                            │    │
│  │     ├── YES, verification_passed == true                         │    │
│  │     │   → vote agrees=false (our verification passed)            │    │
│  │     │                                                            │    │
│  │     └── NO (cache miss):                                         │    │
│  │          │                                                       │    │
│  │          ├── proof_type is C3a/C3b AND forwarded payload present │    │
│  │          │   → validate forwarded ECDSA                          │    │
│  │          │     ├── INVALID → abstain (return, no vote)           │    │
│  │          │     ▼ VALID                                           │    │
│  │          │   → dispatch async ZK re-verification                 │    │
│  │          │   → DEFER vote (cast after ZK completes)              │    │
│  │          │   → on ZK result: agrees = !zk_passed                 │    │
│  │          │                                                       │    │
│  │          └── other proof type, no forwarded payload              │    │
│  │              → ABSTAIN (return without voting)                   │    │
│  │                                                                  │    │
│  └──────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Sign vote with ECDSA, broadcast via P2P                                 │
│  check_quorum()                                                          │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘

         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ check_quorum()                                                           │
│ zk-prover/src/actors/accusation_manager.rs:740                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  agree_count >= threshold_m?                                             │
│     │                                                                    │
│     ├── YES:                                                             │
│     │   All agreeing voters saw same data_hash?                          │
│     │     ├── YES → AccusedFaulted     (SLASHABLE)                       │
│     │     └── NO  → Equivocation       (SLASHABLE)                       │
│     │   → emit AccusationQuorumReached immediately                       │
│     │                                                                    │
│     └── NO:                                                              │
│          Can quorum still be reached (with remaining voters)?            │
│            ├── YES → wait for more votes                                 │
│            └── NO  → resolve:                                            │
│                  Multiple data_hashes across votes?                      │
│                    ├── YES → Equivocation   (SLASHABLE)                  │
│                    └── NO  →                                             │
│                       Only accuser says bad, others say good?            │
│                         ├── YES → AccuserLied   (NOT slashable)          │
│                         └── NO  → Inconclusive  (NOT slashable)          │
│                                                                          │
│  ON TIMEOUT (300s, line 830-886):                                        │
│    Same resolution logic as "cannot reach quorum"                        │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
         │
         ▼ (only if SLASHABLE)
┌──────────────────────────────────────────────────────────────────────────┐
│ SlashingManagerSolWriter                                                 │
│ evm/src/slashing_manager_sol_writer.rs:91                                │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  AccusationQuorumReached { outcome = AccusedFaulted | Equivocation }     │
│                                                                          │
│  Sort agreeing voters by address (ascending)                             │
│  Am I in the top 3?                                                      │
│     ├── NO ──► don't submit (leave to others)                            │
│     ▼ YES                                                                │
│                                                                          │
│  Staggered submission (prevent wasted gas):                              │
│    Rank 0 (lowest address):  submit immediately                          │
│    Rank 1:                   wait 30s, then submit                       │
│    Rank 2:                   wait 60s, then submit                       │
│                                                                          │
│  On-chain: SlashingManager.proposeSlash(e3_id, operator, evidence)       │
│    Evidence = abi.encode(proofType, voters[], agrees[], hashes[], sigs[])│
│                                                                          │
│  Contract verifies:                                                      │
│    • Array lengths match                                                 │
│    • numVotes >= threshold_m                                             │
│    • Each voter: ascending order, no self-vote, active member            │
│    • Each ECDSA signature valid (matches Solidity typehash)              │
│    • Replay protection (no double-slash)                                 │
│    → ATOMIC EXECUTION: slash ticket + license + expel from committee     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## MEMBER EXPULSION (at any phase)

```
SlashExecuted event (on-chain slash confirmed)
         │
         ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ CommitteeMemberExpelled event emitted                                    │
│ Received by ALL actors simultaneously                                    │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ThresholdKeyshare::handle_committee_member_expelled()                   │
│  keyshare/src/threshold_keyshare.rs:519                                  │
│    1. Add to expelled_parties set                                        │
│    2. Remove from honest_parties                                         │
│    3. Remove from all collectors:                                        │
│       • EncryptionKeyCollector                                           │
│       • ThresholdShareCollector                                          │
│       • DecryptionKeySharedCollector                                     │
│    4. If collector now considers "all collected"                          │
│       → triggers next phase transition                                   │
│    5. If threshold can't be met → E3Failed                               │
│                                                                          │
│  PublicKeyAggregator::handle_member_expelled()                           │
│  aggregator/src/publickey_aggregator.rs:777                              │
│    1. Remove keyshare and C1 proof                                       │
│    2. Decrement threshold_n                                              │
│    3. If in Collecting and now "all received"                             │
│       → transition to VerifyingC1                                        │
│    4. If threshold can't be met → E3Failed                               │
│                                                                          │
│  AccusationManager:                                                      │
│    1. Purge votes from expelled member                                   │
│    2. Remove from committee list                                         │
│    3. Re-check pending quorums                                           │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## ON-CHAIN TIMEOUTS

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    TIMEOUT DEADLINES                                     │
├──────────────────────┬───────────────────────┬───────────────────────────┤
│ Phase                │ Deadline              │ On Expiry                 │
├──────────────────────┼───────────────────────┼───────────────────────────┤
│ Committee Formation  │ committeeDeadline     │ markE3Failed() callable   │
│ (Requested→Finalized)│                       │ → E3Failed                │
│                      │                       │ → Requester refunded      │
├──────────────────────┼───────────────────────┼───────────────────────────┤
│ DKG                  │ dkgDeadline           │ markE3Failed() callable   │
│ (Finalized→KeyPub)   │                       │ → E3Failed                │
│                      │                       │ → Partial refund          │
├──────────────────────┼───────────────────────┼───────────────────────────┤
│ Computation          │ computeDeadline       │ markE3Failed() callable   │
│ (KeyPub→Ciphertext)  │                       │ → E3Failed                │
│                      │                       │ → Partial refund          │
├──────────────────────┼───────────────────────┼───────────────────────────┤
│ Decryption           │ decryptionDeadline    │ markE3Failed() callable   │
│ (Ciphertext→Complete)│                       │ → E3Failed                │
│                      │                       │ → Partial refund          │
└──────────────────────┴───────────────────────┴───────────────────────────┘

Anyone can call markE3Failed() after a deadline expires.
On failure: slashed funds go to E3RefundManager.
On success: fees split between CN rewards and protocol treasury.
```

---

## COMPLETE FAILURE CATALOG

```
┌──────────────────────────────────────────────────────────────────────────┐
│ #  │ WHERE                    │ WHAT                  │ CONSEQUENCE       │
├────┼──────────────────────────┼───────────────────────┼───────────────────┤
│  1 │ Step 1.1                 │ BFV keygen fails      │ Node can't join   │
│  2 │ Step 1.3                 │ Key collection timeout│ E3Failed          │
│  3 │ Step 1.3                 │ C0 ECDSA fails        │ Accusation        │
│  4 │ Step 1.3                 │ C0 ZK fails           │ Accusation        │
│  5 │ Step 1.3                 │ Member expelled       │ Recheck threshold │
│  6 │ Step 1.4                 │ TrBFV compute fails   │ Node stuck        │
│  7 │ Step 2.2                 │ BFV encryption fails  │ Missing share     │
│  8 │ Step 2.2                 │ ZK proof gen fails    │ Incomplete proofs │
│  9 │ Step 3.1                 │ Share collect timeout  │ E3Failed          │
│ 10 │ Step 3.1                 │ Member expelled       │ Recheck threshold │
│ 11 │ Step 3.2                 │ No proofs from party  │ pre_dishonest     │
│ 12 │ Step 3.2                 │ Wrong proof count     │ pre_dishonest     │
│ 13 │ Step 3.2                 │ All pre_dishonest     │ Skip to threshold │
│ 14 │ Step 3.3                 │ ECDSA e3_id mismatch  │ ecdsa_dishonest   │
│ 15 │ Step 3.3                 │ ECDSA sig invalid     │ ecdsa_dishonest   │
│ 16 │ Step 3.3                 │ ECDSA signer mismatch │ ecdsa_dishonest   │
│ 17 │ Step 3.3                 │ Circuit name wrong    │ ecdsa_dishonest   │
│ 18 │ Step 3.3                 │ All ECDSA fail        │ Skip ZK           │
│ 19 │ Step 3.3                 │ ZK dispatch fails     │ All dishonest     │
│ 20 │ Step 3.4                 │ ZK verification fails │ Accusation        │
│ 21 │ Step 3.4                 │ Missing from ZK result│ Treated dishonest │
│ 22 │ Step 3.5                 │ honest ≤ threshold_m  │ E3Failed          │
│ 23 │ Step 4.1                 │ Dimension mismatch    │ Party excluded    │
│ 24 │ Step 4.1                 │ All excluded by dims  │ E3Failed          │
│ 25 │ Step 4.1                 │ BFV decryption fails  │ Node stuck        │
│ 26 │ Step 4.2                 │ C4 proof gen fails    │ Missing proofs    │
│ 27 │ Step 4.3                 │ C4 collect timeout    │ E3Failed          │
│ 28 │ Step 4.3                 │ Wrong ESM proof count │ pre_dishonest     │
│ 29 │ Step 4.3                 │ C4 ECDSA/ZK fails     │ Accusation        │
│ 30 │ Step 4.3                 │ honest ≤ threshold    │ E3Failed          │
│ 31 │ Step 5.1-5.7             │ C1 ZK fails           │ Accusation        │
│ 32 │ Step 5.1-5.7             │ C1 commitment≠keyshare│ Party excluded    │
│ 33 │ Step 5.1-5.7             │ Too few honest for agg│ E3Failed          │
│ 34 │ Step 5.1-5.7             │ C5 proof gen fails    │ PK not published  │
│ 35 │ Step 5.1-5.7             │ Cross-node fold fails │ PK not published  │
│ 36 │ Phase 6                  │ Not in ReadyForDecrypt│ Event ignored     │
│ 37 │ Phase 6                  │ Decrypt share fails   │ Node stuck        │
│ 38 │ Phase 6                  │ C6 proof gen fails    │ Share not sent    │
│ 39 │ Phase 7                  │ C4→C6 gate mismatch   │ Accusation        │
│ 40 │ Phase 7                  │ C6 ECDSA/ZK fails     │ Accusation        │
│ 41 │ Phase 7                  │ Too few honest decrypt│ Cannot compute    │
│ 42 │ Phase 7                  │ C7 proof gen fails    │ Not published     │
│ 43 │ Phase 7                  │ C6 fold fails         │ Not published     │
│ 44 │ Any phase                │ On-chain timeout      │ markE3Failed()    │
│ 45 │ Any phase                │ Member expelled       │ Recheck threshold │
└────┴──────────────────────────┴───────────────────────┴───────────────────┘
```
