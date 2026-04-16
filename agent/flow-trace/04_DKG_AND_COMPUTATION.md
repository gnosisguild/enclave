# Part 4: DKG, Computation & Decryption

## Overview

After committee finalization, the selected ciphernodes perform Distributed Key Generation (DKG)
using threshold BFV (TrBFV) cryptography. This produces a collective public key without any single
party knowing the full secret key. Later, the committee produces decryption shares; all committee
members buffer them, and the active aggregator combines them. The runtime first normalizes the
finalized committee into ascending ticket-score order, and the active aggregator is then the lowest
non-expelled `party_id` in that normalized order.

---

## Phase 1: DKG — Distributed Key Generation

### Step 1: CiphernodeSelected → BFV Key Generation

**Actor:** `ThresholdKeyshare` (created by `ThresholdKeyshareExtension`)

```
CiphernodeSelected event arrives at ThresholdKeyshare
│
├─ handle_ciphernode_selected():
│   │
│   ├─ 1. Generate fresh BFV keypair:
│   │     (secret_key, public_key) = BFV::keygen(share_encryption_preset)
│   │     → This is the node's SHARE ENCRYPTION key
│   │     → Used to encrypt Shamir shares sent to this node
│   │
│   ├─ 2. Encrypt BFV secret key at rest:
│   │     encrypted_sk = Cipher.encrypt(secret_key)
│   │     → Stored locally, password-protected
│   │
│   ├─ 3. State transition: Init → CollectingEncryptionKeys
│   │
│   ├─ 4. Publish EncryptionKeyPending {
│   │     e3_id, party_id, bfv_public_key
│   │   }
│   │   → ZK proof actor picks this up
│   │
│   ├─ 5. Create child actors:
│   │     ├─ EncryptionKeyCollector (waits for all N parties' keys)
│   │     └─ ThresholdShareCollector (waits for all N parties' shares)
│   │     → These collectors start immediately so early peer keys/shares can
│   │       be buffered while this node is still finishing earlier DKG phases
│   │
│   └─ Each collector has a timeout (60s for keys, 120s for shares)
```

### Step 2: C0 Proof Generation → EncryptionKeyCreated

**Actor:** `ProofRequestActor` (`crates/zk-prover/src/actors/proof_request.rs`)

```
ProofRequestActor receives EncryptionKeyPending
│
├─ 1. Creates ZK proof request:
│     ComputeRequest::zk(ZkRequest::PkBfv {
│       bfv_public_key, party_id, bfv_params
│     })
│     → Circuit: PkBfv (C0 — proves BFV keypair was generated correctly)
│
├─ 2. ZkActor (IO layer) receives ComputeRequest:
│     ├─ Writes witness data to temp directory
│     ├─ Spawns: bb prove -b circuit.json -w witness.gz -k vk -o proof/
│     │   → Barretenberg (bb) binary generates ZK proof
│     └─ Returns: Proof { data, public_signals }
│
├─ 3. ProofRequestActor receives ComputeResponse:
│     ├─ Signs proof via sign_proof():
│     │   digest = keccak256(abi.encode(
│     │     PROOF_PAYLOAD_TYPEHASH,
│     │     chainId, e3Id, proofType(C0),
│     │     keccak256(proof.data),
│     │     keccak256(proof.public_signals)
│     │   ))
│     │   signature = ecSign(digest, operator_private_key)
│     │   → 65-byte ECDSA signature (r||s||v)
│     │
│     └─ Publishes EncryptionKeyCreated {
│          e3_id, party_id, bfv_public_key,
│          signed_proof: SignedProofPayload { proof, signature }
│        }
│        → Broadcast to all nodes via libp2p gossip
│
└─ RECEIVING NODES verify C0 proof:
     ProofVerificationActor receives EncryptionKeyReceived (from P2P)
     │
     ├─ Recovers ECDSA signer address from signed proof
     ├─ Dispatches ZK verification to ZkActor:
     │   ZkActor runs: bb verify -k vk -p proof.data
     │
     ├─ If verification PASSES:
     │   ├─ Publishes EncryptionKeyCreated (locally trusted)
     │   └─ Publishes ProofVerificationPassed (cached by AccusationManager)
     │
     └─ If verification FAILS:
         └─ Publishes SignedProofFailed { accused, proof_type: C0 }
            → Triggers accusation pipeline (see Part 5)
```

### Step 3: Collect All Encryption Keys

```
EncryptionKeyCollector waits for EncryptionKeyCreated from ALL N parties
│
├─ On each arrival: store (party_id → bfv_public_key)
│
├─ On TIMEOUT (60s):
│   └─ Publish EncryptionKeyCollectionFailed
│   └─ ThresholdKeyshare actor stops
│
└─ When ALL N collected:
    └─ Send AllEncryptionKeysCollected to parent ThresholdKeyshare
```

### Step 4: Generate TrBFV Key Shares + Shamir Secret Shares

```
ThresholdKeyshare receives AllEncryptionKeysCollected
│
├─ State: CollectingEncryptionKeys → GeneratingThresholdShare
├─ Stores all parties' BFV public keys
│
├─ COMPUTE REQUEST 1: GenPkShareAndSkSss
│   │
│   │  ┌─── TrBFV Computation ──────────────────────────────────┐
│   │  │                                                         │
│   │  │  Inputs: BFV params, party_id, threshold_m, threshold_n│
│   │  │                                                         │
│   │  │  Steps:                                                 │
│   │  │  1. Generate TrBFV secret key (sk) & public key share  │
│   │  │     → sk is this node's portion of the collective key   │
│   │  │     → pk_share is the public contribution               │
│   │  │                                                         │
│   │  │  2. Create Shamir Secret Shares of sk (sk_sss):        │
│   │  │     ShareManager::create_shares(sk, M, N)               │
│   │  │     → Splits sk into N shares, any M+1 can reconstruct │
│   │  │     → One share per committee member                    │
│   │  │                                                         │
│   │  │  3. Generate smudging noise (e_sm_raw):                │
│   │  │     → Statistical security parameter                    │
│   │  │     → Prevents information leakage during decryption    │
│   │  │                                                         │
│   │  │  4. Extract raw polynomials for ZK proof:              │
│   │  │     pk0_share_raw, sk_raw, eek_raw                     │
│   │  │                                                         │
│   │  │  5. Encrypt all secrets with node's Cipher              │
│   │  │                                                         │
│   │  │  Output: pk_share, sk_sss[N], e_sm_raw, raw_polys     │
│   │  └─────────────────────────────────────────────────────────┘
│
└─ COMPUTE REQUEST 2: GenEsiSss (immediately after)
    │
    │  ┌─── TrBFV Computation ──────────────────────────────────┐
    │  │                                                         │
    │  │  Generate Shamir shares of Error Smudging Info (ESI):  │
    │  │  → Multiple sets, one per ciphertext                    │
    │  │  → Each set: N shares, M+1 threshold to reconstruct    │
    │  │                                                         │
    │  │  Output: esi_sss[num_ciphertexts][N]                   │
    │  └─────────────────────────────────────────────────────────┘
```

### Step 5: Encrypt & Broadcast Shares (with C1, C2, C3 Proofs)

```
Both GenPkShareAndSkSss and GenEsiSss complete
│
├─ handle_shares_generated():
│   │
│   ├─ 1. For EACH other party j in committee:
│   │     Encrypt sk_sss[j] under party j's BFV public key
│   │     Encrypt esi_sss[*][j] under party j's BFV public key
│   │     → BfvEncryptedShares::encrypt_all()
│   │     → Only party j can decrypt their share
│   │
│   ├─ 2. Build ThresholdShare struct:
│   │     {
│   │       party_id,
│   │       pk_share,          // public key share (public)
│   │       encrypted_sk_sss,  // encrypted for each target party
│   │       encrypted_esi_sss  // encrypted for each target party
│   │     }
│   │
│   ├─ 3. Build proof requests for FIVE circuit types:
│   │     ├─ C1: PkGenerationProofRequest
│   │     │   → Proves TrBFV pk_share was generated correctly from sk
│   │     ├─ C2a: ShareComputationProofRequest (SK)
│   │     │   → Proves Shamir shares of sk were computed correctly
│   │     ├─ C2b: ShareComputationProofRequest (ESM)
│   │     │   → Proves Shamir shares of smudging noise were computed correctly
│   │     ├─ C3a: ShareEncryptionProofRequests (SK, one per recipient × row)
│   │     │   → Proves each sk_sss share was encrypted correctly under recipient's BFV key
│   │     └─ C3b: ShareEncryptionProofRequests (ESM, one per ESI × recipient × row)
│   │         → Proves each esi_sss share was encrypted correctly
│   │
│   ├─ 4. Publish ThresholdSharePending {
│   │       full_share, proof_request(C1),
│   │       sk_share_computation_request(C2a),
│   │       e_sm_share_computation_request(C2b),
│   │       sk_share_encryption_requests(C3a[]),
│   │       e_sm_share_encryption_requests(C3b[]),
│   │       recipient_party_ids
│   │     }
│   │     → ProofRequestActor picks this up
│   │
│   └─ State: GeneratingThresholdShare → AggregatingDecryptionKey
```

### Step 5a: C1 + C2 + C3 Proof Generation

**Actor:** `ProofRequestActor`

```
ProofRequestActor receives ThresholdSharePending
│
├─ 1. Creates PendingThresholdProofs tracker:
│     expected = 1 (C1) + 1 (C2a) + 1 (C2b) + SK_ENC_COUNT (C3a) + ESM_ENC_COUNT (C3b)
│     → All proofs must complete before publishing ThresholdShareCreated
│
├─ 2. Dispatches ALL proof requests in parallel:
│     ├─ C1:  ComputeRequest::zk(ZkRequest::PkGeneration {...})
│     ├─ C2a: ComputeRequest::zk(ZkRequest::ShareComputation { kind: SK })
│     ├─ C2b: ComputeRequest::zk(ZkRequest::ShareComputation { kind: ESM })
│     ├─ C3a[i]: ComputeRequest::zk(ZkRequest::ShareEncryption { recipient, row })
│     │   → One per recipient party × modulus row
│     └─ C3b[i]: ComputeRequest::zk(ZkRequest::ShareEncryption { esi_idx, recipient, row })
│         → One per ESI × recipient party × modulus row
│
├─ 3. ZkActor generates proofs via bb binary (in parallel via multithread):
│     → Each proof takes 1-10 seconds depending on circuit complexity
│
├─ 4. As each ComputeResponse arrives:
│     ├─ Store proof in PendingThresholdProofs map
│     ├─ Check is_complete():
│     │   all of: pk_generation_proof(C1), sk_share_computation_proof(C2a),
│     │           e_sm_share_computation_proof(C2b),
│     │           ALL sk_share_encryption_proofs(C3a),
│     │           ALL e_sm_share_encryption_proofs(C3b)
│     └─ When ALL proofs complete (is_complete() → true):
│
├─ 5. Sign all proofs via sign_and_group_proofs():
│     → Each proof gets its own SignedProofPayload with ECDSA signature
│     → C3a/C3b proofs indexed by (recipient_party_id, row_index)
│
├─ 6. Publish events:
│     ├─ PkGenerationProofSigned { e3_id, party_id, signed_proof(C1) }
│     ├─ DkgProofSigned { signed_proof } × (C2a, C2b, each C3a, each C3b)
│     └─ ThresholdShareCreated {
│          e3_id, party_id,
│          threshold_share,               // pk_share + encrypted shares
│          signed_pk_generation_proof,     // C1
│          signed_sk_computation_proof,    // C2a
│          signed_esm_computation_proof,   // C2b
│          signed_sk_encryption_proofs,    // C3a[] indexed by (recipient, row)
│          signed_esm_encryption_proofs    // C3b[] indexed by (esi, recipient, row)
│        }
│        → Broadcast to all nodes via libp2p gossip
│
└─ IMPORTANT: ThresholdShareCreated is NOT published until ALL proofs complete
   → Ensures no incomplete data is gossiped
```

**C2 proofs:** For each C2a/C2b request, the prover builds a **recursive** proof for
`sk_share_computation` / `e_sm_share_computation`. That `Proof` is what `PendingThresholdProofs`
stores and what gets ECDSA-signed for gossip (`ProofType::C2aSkShareComputation` /
`C2bESmShareComputation`). The old generic `recursive_aggregation/wrapper/*` circuits and two-proof
`recursive_aggregation/fold` were removed; aggregation is done by ad-hoc Noir bins under
`circuits/bin/recursive_aggregation/` (e.g. `c2ab_fold`, `c3ab_fold`, `c6_fold`, `node_fold`,
`nodes_fold`, `dkg_aggregator`, `decryption_aggregator` — `nodes_fold` chains `H` `node_fold` proofs
for `dkg_aggregator`; `decryption_aggregator` folds C6 via non-ZK `c6_fold` then checks C7 with ZK).
The per-circuit `wrapper/` Noir step was removed; multithread still sets `wrapped_proof` in
responses to `proof.clone()` of the inner recursive proof so aggregators keep the same response
shape.

**Ciphernode / aggregator integration:** `ZkRequest::FoldProofs` was removed. The multithread actor
implements `ZkRequest::NodeDkgFold` (full per-node pipeline to a `NodeFold` proof),
`ZkRequest::DkgAggregation` (`NodesFold` + C5 + `DkgAggregator`), and
`ZkRequest::DecryptionAggregation` (per-ciphertext `C6Fold` + C7 + `DecryptionAggregator`).
`NodeProofAggregator` buffers all `DKGInnerProofReady` proofs then issues one `NodeDkgFold` request;
`PublicKeyAggregator` and `ThresholdPlaintextAggregator` dispatch the aggregator requests instead of
pairwise folding.

### Step 6: Collect All Threshold Shares (with C2/C3 Verification)

```
ThresholdShareCollector waits for ThresholdShareCreated from ALL N parties
│
├─ Each ThresholdShareCreated arrives via libp2p P2P network
│
├─ ThresholdKeyshare.handle_threshold_share_created():
│   ├─ Filters: only process shares where target_party_id == MY party_id
│   │   → Each share blob contains material for ALL parties
│   │   → This node only extracts what's encrypted for it
│   └─ Forwards filtered share to ThresholdShareCollector
│
├─ On TIMEOUT (120s):
│   └─ Publish ThresholdShareCollectionFailed
│   └─ ThresholdKeyshare actor stops
│
└─ When ALL N shares collected:
    ├─ Send AllThresholdSharesCollected to ThresholdKeyshare
    │
    └─ DISPATCH C2/C3 VERIFICATION:
        ThresholdKeyshare.dispatch_c2_c3_verification()
        │
        └─ Publishes ShareVerificationDispatched {
             kind: ShareProofs,
             party_proofs: [all C2a, C2b, C3a, C3b proofs per party],
             pre_dishonest: [parties with missing/incomplete proofs]
           }
           → ShareVerificationActor picks this up
```

### Step 6a: C2/C3 Share Proof Verification

**Actor:** `ShareVerificationActor` (`crates/zk-prover/src/actors/share_verification.rs`)

```
ShareVerificationActor receives ShareVerificationDispatched(kind=ShareProofs)
│
├─ PHASE 1: Lightweight ECDSA Validation (inline):
│   │
│   ├─ For EACH party's proofs:
│   │   ├─ Verify e3_id matches
│   │   ├─ Recover ECDSA signer address from each signed proof
│   │   ├─ Verify signer consistency: all proofs from same address
│   │   ├─ Validate circuit names match expected ProofType::circuit_names()
│   │   │
│   │   ├─ If ANY ECDSA check fails:
│   │   │   └─ Emit SignedProofFailed { accused, proof_type }
│   │   │      → Triggers accusation pipeline (see Part 5)
│   │   │
│   │   └─ If ECDSA passes: cache recovered address, proceed
│   │
│   └─ Store PendingConsistencyCheck {
│        ecdsa_dishonest, pre_dishonest, dispatched_party_ids,
│        recovered_addresses, party_proofs (for ZK dispatch)
│      }
│
├─ PHASE 2: Commitment Consistency Check (dispatched to per-E3 checker):
│   │
│   ├─ Publishes CommitmentConsistencyCheckRequested {
│   │     correlation_id, kind, party_proofs: [(party_id, address, proofs)]
│   │   }
│   │
│   ├─ CommitmentConsistencyChecker (per-E3 actor) receives this:
│   │   ├─ Caches each party's (address, proof_type) → {public_signals, data_hash}
│   │   ├─ Evaluates all registered CommitmentLinks:
│   │   │     C0→C3   (SourceMustExistInTargets): C3's expected_pk_commitment ∈ any C0 pk_commitment
│   │   │     C1→C2a  (SameParty):                C1's sk_commitment == C2a's expected_secret_commitment
│   │   │     C1→C2b  (SameParty):                C1's e_sm_commitment == C2b's expected_secret_commitment
│   │   │     C1→C5   (CrossParty):               C1's pk_commitment ∈ C5 expected pk inputs
│   │   │     C2→C3   (SameParty):                C3's expected_message_commitment ∈ C2's share commitments
│   │   │     C2→C4   (SourceMustExistInTargets): C2's L share commitments for recipient R exactly
│   │   │                                          match C4_R's expected_commitments row for sender X
│   │   │     C4a→C6  (SameParty):                C4a's commitment == C6's expected_sk_commitment
│   │   │     C4b→C6  (SameParty):                C4b's commitment == C6's expected_e_sm_commitment
│   │   │     C6→C7   (CrossParty):               C6's d_commitment matches C7's expected_d_commitment
│   │   │     (on-chain / E3 state)              C3 `ct_commitment` output and C6 `ct_commitment` input bind to the same ciphertext as user_data_encryption (not a CommitmentLink row)
│   │   │
│   │   ├─ On mismatch: publishes CommitmentConsistencyViolation
│   │   │   → AccusationManager initiates accusation quorum (see Part 5)
│   │   └─ Responds with CommitmentConsistencyCheckComplete { inconsistent_parties }
│   │
│   └─ On CommitmentConsistencyCheckComplete:
│       ├─ Merge inconsistent_parties into dishonest set
│       └─ Proceed to Phase 3 with remaining honest parties
│
├─ PHASE 3: Heavy ZK Verification (dispatched to multithread):
│   │
│   ├─ Publishes ComputeRequest::zk(VerifyShareProofsRequest {
│   │     party_proofs, // consistency-passing parties' ZK proof data
│   │   })
│   │
│   ├─ Multithread ZK verify: `bb verify` on inner recursive circuits (same path as
│   │   `ZkProver::verify_proof`)
│   │   → Returns per-party pass/fail results
│   │
│   └─ On ComputeResponse:
│       ├─ Cross-check: all dispatched parties accounted for
│       ├─ For each party:
│       │   ├─ all_verified → Emit ProofVerificationPassed
│       │   └─ NOT all_verified → Emit SignedProofFailed
│       │
│       └─ Publish ShareVerificationComplete {
│            kind: ShareProofs,
│            dishonest_parties: {pre_dishonest ∪ ecdsa_fails ∪ consistency_fails ∪ zk_fails}
│          }
│
└─ ThresholdKeyshare receives ShareVerificationComplete:
    ├─ If dishonest_parties is empty: proceed to Step 7
    └─ If dishonest_parties is non-empty:
        → Accusation pipeline handles slashing (see Part 5)
        → DKG may still proceed if enough honest parties remain
```

### Step 7: Calculate Decryption Key (with C4 Proofs & Verification)

```
ThresholdKeyshare receives AllThresholdSharesCollected
│
├─ 1. Decrypt each received share using THIS node's BFV secret key:
│     For each party j's share:
│       sk_sss_j = BFV::decrypt(encrypted_sk_sss_j, my_bfv_sk)
│       esi_sss_j = BFV::decrypt(encrypted_esi_sss_j, my_bfv_sk)
│
├─ 2. COMPUTE REQUEST: CalculateDecryptionKey
│     │
│     │  ┌─── TrBFV Computation ──────────────────────────────┐
│     │  │                                                     │
│     │  │  Inputs: all sk_sss shares, all esi_sss shares     │
│     │  │                                                     │
│     │  │  1. Reconstruct summed secret key polynomial:       │
│     │  │     sk_poly_sum = Shamir::reconstruct(              │
│     │  │       [sk_sss_1, sk_sss_2, ..., sk_sss_N]          │
│     │  │     )                                               │
│     │  │     → This is NOT the full secret key               │
│     │  │     → It's this node's PORTION of the summed key    │
│     │  │                                                     │
│     │  │  2. Reconstruct summed ESI polynomials:             │
│     │  │     es_poly_sum = Shamir::reconstruct(              │
│     │  │       [esi_sss_1, esi_sss_2, ..., esi_sss_N]       │
│     │  │     )                                               │
│     │  │                                                     │
│     │  │  Output: (sk_poly_sum, es_poly_sum)                 │
│     │  │  → Stored encrypted locally for later decryption    │
│     │  └─────────────────────────────────────────────────────┘
│
├─ 3. PUBLISH C4 PROOF REQUESTS:
│     DecryptionShareProofsPending {
│       sk_request:   DkgShareDecryptionProofRequest (C4a),
│       esm_requests: Vec<DkgShareDecryptionProofRequest> (C4b, one per ESI),
│       sk_poly_sum, es_poly_sum  // decrypted aggregates for proof inputs
│     }
│     → ProofRequestActor picks this up
│
├─ 4. C4 PROOF GENERATION (ProofRequestActor):
│     │
│     │  ├─ Creates PendingDecryptionProofs:
│     │  │   expected = 1 (C4a for SK) + num_esi (C4b for each ESM)
│     │  │
│     │  ├─ Dispatches proof requests:
│     │  │   C4a: ComputeRequest::zk(ZkRequest::DkgShareDecryption { kind: SK })
│     │  │   C4b[i]: ComputeRequest::zk(ZkRequest::DkgShareDecryption { kind: ESM, esi_idx })
│     │  │   → Proves share decryption was performed correctly
│     │  │   → Proves the reconstructed key portion is valid
│     │  │
│     │  ├─ ZkActor generates all C4 proofs
│     │  │
│     │  └─ When is_complete() (all C4a + C4b proofs):
│     │      ├─ Signs all proofs
│     │      └─ Publishes DecryptionKeyShared {
│     │           e3_id, party_id,
│     │           sk_poly_sum, es_poly_sum,        // protocol data
│     │           signed_sk_decryption_proof,       // C4a
│     │           signed_esm_decryption_proofs[]    // C4b per ESI
│     │         }
│     │         → Broadcast to all committee nodes via P2P gossip
│     │         → This is Protocol Exchange #3 (decryption key sharing)
│
├─ 5. COLLECT C4 SHARES FROM ALL PARTIES:
│     ThresholdKeyshare waits for DecryptionKeyShared from ALL N parties
│     │
│     └─ When all collected → AllDecryptionKeySharesCollected
│
├─ 6. C4 VERIFICATION:
│     ThresholdKeyshare.dispatch_c4_verification()
│     │
│     └─ Publishes ShareVerificationDispatched {
│          kind: DecryptionProofs,
│          party_proofs: [C4a + C4b proofs per party]
│        }
│        → ShareVerificationActor performs same 2-phase verification:
│          Phase 1: ECDSA signature recovery
│          Phase 2: Commitment consistency check (C2→C4, C4a→C6, C4b→C6)
│          Phase 3: ZK proof verification via bb binary
│        → On failure: SignedProofFailed → accusation pipeline
│        → On pass: ProofVerificationPassed (cached)
│
├─ 7. State: AggregatingDecryptionKey → ReadyForDecryption
│
└─ 8. Publish KeyshareCreated {
       e3_id, party_id,
       pk_share,        // public key share
       signed_proof     // ZK proof of correct generation
     }
    → Broadcast to committee members via P2P
```

---

## Phase 2: Public Key Aggregation (Committee-Buffered, Active Aggregator Submits)

```
  All committee members receive KeyshareCreated events
│
├─ KeyshareCreatedFilterBuffer gates events:
  │   └─ Only accepts KeyshareCreated from verified committee members
  │   └─ Buffers until BOTH CommitteeFinalized and AggregatorChanged(is_aggregator=true)
  │   └─ On expulsion-driven handoff, the next active aggregator flushes its existing buffer
│
  ├─ Only the active aggregator's buffer flushes into PublicKeyAggregator
  │
  ├─ When threshold_n keyshares collected:
│   │
│   ├─ 1. Aggregate public key shares:
│   │     aggregate_pk = Fhe::get_aggregate_public_key(
│   │       [pk_share_1, pk_share_2, ..., pk_share_N]
│   │     )
│   │     → Uses PublicKeyShare::aggregate()
│   │     → Produces the COLLECTIVE public key
│   │     → Anyone can encrypt with this key
│   │     → Only M+1 committee members can decrypt together
│   │
│   ├─ 2. Compute commitment:
│   │     pk_hash = compute_pk_commitment(aggregate_pk)
│   │
│   ├─ 3. REQUEST C5 PROOF:
│   │     Publish PkAggregationProofPending {
│   │       proof_request: PkAggregationProofRequest,
│   │       public_key: aggregate_pk,
│   │       public_key_hash: pk_hash
│   │     }
│   │
│   ├─ 4. C5 PROOF GENERATION (ProofRequestActor):
│   │     ├─ Dispatches ComputeRequest::zk(ZkRequest::PkAggregation {...})
│   │     │   → Circuit: PkAggregation (C5)
│   │     │   → Proves aggregate PK was correctly computed from all pk_shares
│   │     ├─ ZkActor generates proof via bb binary
│   │     ├─ Signs proof
│   │     └─ Publishes PkAggregationProofSigned {
│   │          e3_id, party_id, signed_proof(C5)
│   │        }
│   │
│   └─ 5. Publish PublicKeyAggregated {
│         e3_id, aggregate_pk, pk_hash, node_list
│       }
│
└─ CiphernodeRegistrySolWriter receives PublicKeyAggregated:
  ├─ Requires EffectsEnabled
  ├─ Requires active_aggregators[e3_id] == true
  ├─ Reads chain state to confirm committee public key is still unset
  └─ Calls contract.publishCommittee(e3_id, nodes, publicKey, pkHash)
        │
        │  ┌─── ON-CHAIN (CiphernodeRegistryOwnable) ──────────┐
        │  │                                                     │
        │  │  publishCommittee(e3Id, nodes, pk, pkHash) {        │
        │  │    1. require(initialized && finalized)             │
        │  │    2. require(publicKeyHashes[e3Id] == 0)           │
        │  │       → Can only publish once                       │
        │  │    3. require(nodes.length == committee.length)     │
        │  │    4. publicKeyHashes[e3Id] = pkHash                │
        │  │    5. enclave.onCommitteePublished(e3Id, pkHash)    │
        │  │       │                                             │
        │  │       │  ┌─ Enclave.sol ────────────────────────┐  │
        │  │       │  │  onCommitteePublished(e3Id, pkHash) {│  │
        │  │       │  │    require(stage==CommitteeFinalized) │  │
        │  │       │  │    e3.committeePublicKey = pkHash     │  │
        │  │       │  │    → stored as bytes32 (a hash)      │  │
        │  │       │  │    stage = KeyPublished               │  │
        │  │       │  │    Emit E3StageChanged(KeyPublished)  │  │
        │  │       │  │  }                                   │  │
        │  │       │  └──────────────────────────────────────┘  │
        │  │    6. Emit CommitteePublished(e3Id, nodes, pk, C5 proof) │
        │  │       → Note: emits full pk bytes, NOT just pkHash  │
        │  │  }                                                  │
        │  └─────────────────────────────────────────────────────┘
```

---

## Phase 3: Encrypted Computation

### Input Submission (External)

```
Data providers submit encrypted inputs:
│
└─ e3Program.publishInput(e3Id, encryptedData)
   → Must be within inputWindow [start, end]
   → Encrypted under the committee's aggregate public key
   → Only M+1 committee members can collectively decrypt
```

### Ciphertext Output Publication

```
Compute provider runs computation on encrypted data:
│
└─ Enclave.publishCiphertextOutput(e3Id, ciphertextOutput, proof)
    │
    │  ┌─── ON-CHAIN (Enclave.sol) ─────────────────────────────┐
    │  │                                                         │
    │  │  publishCiphertextOutput(e3Id, output, proof) {         │
    │  │    1. require(stage == KeyPublished)                    │
    │  │    2. require(block.timestamp <= computeDeadline)       │
    │  │    3. require(block.timestamp >= inputWindow[1])        │
    │  │       → Input window must have closed                   │
    │  │    4. require(e3.ciphertextOutput == 0)                │
    │  │       → Can only publish once                           │
    │  │    5. e3.ciphertextOutput = keccak256(output)           │
    │  │    6. e3Program.verify(e3Id, hash, proof)               │
    │  │       → Program verifies computation correctness        │
    │  │       → Must return true                                │
    │  │    7. stage = CiphertextReady                           │
    │  │    8. decryptionDeadline = now + decryptionWindow       │
    │  │    9. Emit CiphertextOutputPublished(e3Id, output)      │
    │  │   10. Emit E3StageChanged(CiphertextReady)              │
    │  │  }                                                      │
    │  └─────────────────────────────────────────────────────────┘
```

---

## Phase 4: Decryption Share Generation (Each Committee Member, with C6 Proof)

```
EnclaveSolReader decodes CiphertextOutputPublished event
│
└─ ThresholdKeyshare receives CiphertextOutputPublished:
    │
    ├─ State: ReadyForDecryption → Decrypting
    │
    ├─ COMPUTE REQUEST: CalculateDecryptionShare
    │   │
    │   │  ┌─── TrBFV Computation ──────────────────────────────┐
    │   │  │                                                     │
    │   │  │  Inputs:                                            │
    │   │  │    - ciphertext (encrypted computation output)      │
    │   │  │    - sk_poly_sum (this node's secret key portion)   │
    │   │  │    - es_poly_sum (this node's smudging noise)       │
    │   │  │                                                     │
    │   │  │  Compute:                                           │
    │   │  │    decryption_share = ShareManager::compute_share(  │
    │   │  │      ciphertext, sk_poly_sum, es_poly_sum           │
    │   │  │    )                                                │
    │   │  │    → One decryption share polynomial per ciphertext │
    │   │  │    → Smudging noise prevents info leakage           │
    │   │  │    → Share alone reveals NOTHING about plaintext    │
    │   │  │                                                     │
    │   │  │  Output: Vec<decryption_share_polynomial>           │
    │   │  └─────────────────────────────────────────────────────┘
    │
    ├─ REQUEST C6 PROOF:
    │   Publish ShareDecryptionProofPending {
    │     proof_request: ThresholdShareDecryptionProofRequest,
    │     decryption_shares
    │   }
    │
    ├─ C6 PROOF GENERATION (ProofRequestActor):
    │   ├─ Dispatches ComputeRequest::zk(ZkRequest::ThresholdShareDecryption {...})
    │   │   → Circuit: ThresholdShareDecryption (C6)
    │   │   → Proves decryption share was correctly computed from
    │   │     sk_poly_sum, es_poly_sum, and ciphertext
    │   │   → Fiat-Shamir transcript absorbs full `d` (all coefficients per CRT limb)
    │   ├─ ZkActor generates proof via bb binary
    │   ├─ Signs proof
    │   └─ Publishes signed C6 proof
    │
    ├─ Publish DecryptionshareCreated {
    │     e3_id, party_id,
    │     decryption_share: Vec<polynomial>,
    │     signed_proof: SignedProofPayload(C6),
    │     node: address
    │   }
    │   → Broadcast via P2P to committee members for buffering
    │
    └─ State: Decrypting → Completed
```

---

## Phase 5: Plaintext Aggregation (Committee-Buffered, Active Aggregator Submits)

```
  All committee members receive DecryptionshareCreated events
│
  ├─ DecryptionshareCreatedBuffer gates events:
  │   ├─ Tracks expelled parties
  │   ├─ Buffers until AggregatorChanged(is_aggregator=true)
  │   └─ Flushes verified shares to ThresholdPlaintextAggregator when this node is active
  │
  ├─ ThresholdPlaintextAggregator receives flushed shares
  │   ├─ Verifies sender is in committee
  │   ├─ Adds the share if verified
  │   └─ Ignores non-members or expelled parties
│
  ├─ C6 VERIFICATION (per-share, on active aggregator):
│   ShareVerificationActor receives C6 signed proofs
│   ├─ ECDSA recovery + ZK verification (same 2-phase as C2/C3)
│   ├─ On failure: SignedProofFailed → accusation pipeline
│   └─ On pass: ProofVerificationPassed (cached)
│
├─ When M+1 shares collected (threshold met):
│   │
│   ├─ State → Computing
│   │
│   ├─ COMPUTE REQUEST: CalculateThresholdDecryption
│   │   │
│   │   │  ┌─── TrBFV Computation ──────────────────────────────┐
│   │   │  │                                                     │
│   │   │  │  Inputs:                                            │
│   │   │  │    - ciphertext output                              │
│   │   │  │    - M+1 decryption shares from different parties   │
│   │   │  │    - party IDs                                      │
│   │   │  │                                                     │
│   │   │  │  Compute:                                           │
│   │   │  │  1. Lagrange interpolation on share polynomials     │
│   │   │  │     → Shamir threshold reconstruction               │
│   │   │  │  2. Combine to recover full decryption              │
│   │   │  │  3. BFV decode plaintext to output bytes            │
│   │   │  │                                                     │
│   │   │  │  Output: plaintext_bytes                            │
│   │   │  └─────────────────────────────────────────────────────┘
│   │
│   ├─ REQUEST C7 PROOF:
│   │   Publish AggregationProofPending {
│   │     proof_request: DecryptedSharesAggregationProofRequest,
│   │     plaintext: Vec<plaintext_bytes>,
│   │     shares: Vec<(party_id, Vec<decryption_share>)>
│   │   }
│   │
│   ├─ C7 PROOF GENERATION (ProofRequestActor):
│   │   ├─ Dispatches ComputeRequest::zk(
│   │   │     ZkRequest::DecryptedSharesAggregation {...}
│   │   │   )
│   │   │   → Circuit: DecryptedSharesAggregation (C7)
│   │   │   → Proves plaintext was correctly reconstructed from M+1 shares
│   │   ├─ ZkActor generates proof(s) via bb binary
│   │   ├─ Signs each C7 proof (one per ciphertext index)
│   │   └─ Publishes AggregationProofSigned {
│   │        e3_id, party_id, signed_proof(C7)
│   │      }
│   │
│   └─ Publish PlaintextAggregated { e3_id, decrypted_output }
│
└─ EnclaveSolWriter receives PlaintextAggregated:
  ├─ Requires EffectsEnabled
  ├─ Requires active_aggregators[e3_id] == true
  ├─ Reads chain state to confirm plaintextOutput is still empty
  └─ Calls contract.publishPlaintextOutput(e3Id, output, proof)
        │
        │  ┌─── ON-CHAIN (Enclave.sol) ─────────────────────────┐
        │  │                                                     │
        │  │  publishPlaintextOutput(e3Id, output, proof) {      │
        │  │    1. require(stage == CiphertextReady)             │
        │  │    2. require(now <= decryptionDeadline)            │
        │  │    3. e3.plaintextOutput = output                   │
        │  │    4. decryptionVerifier.verify(                    │
        │  │         e3Id, keccak256(output), proof              │
        │  │       )                                             │
        │  │       → Verifies decryption was done correctly      │
        │  │    5. stage = Complete                              │
        │  │    6. _distributeRewards(e3Id)                      │
        │  │       │                                             │
        │  │       │  ┌─ Reward Distribution ────────────────┐  │
        │  │       │  │  1. Get active committee nodes:      │  │
        │  │       │  │     nodes = ciphernodeRegistry       │  │
        │  │       │  │       .getActiveCommitteeNodes(e3Id) │  │
        │  │       │  │  2. If no active nodes:              │  │
        │  │       │  │     → Refund requester               │  │
        │  │       │  │  3. Divide payment equally:          │  │
        │  │       │  │     perNode = payment / nodes.length │  │
        │  │       │  │     dust → last member               │  │
        │  │       │  │  4. Approve BondingRegistry          │  │
        │  │       │  │  5. bondingRegistry.distributeRewards│  │
        │  │       │  │       (token, nodes, amounts)        │  │
        │  │       │  │     → Transfers fee tokens to each   │  │
        │  │       │  │       registered operator            │  │
        │  │       │  │  6. Emit RewardsDistributed          │  │
        │  │       │  └──────────────────────────────────────┘  │
        │  │    7. Emit PlaintextOutputPublished(e3Id, output, C7 proof) │
        │  │    8. Emit E3StageChanged(Complete)                 │
        │  │  }                                                  │
        │  └─────────────────────────────────────────────────────┘
```

---

## ZK Proof Type Summary (C0–C7)

```
┌──────┬────────────────────────────┬───────────────────┬──────────────────────────────┐
│ Code │ Name                       │ Stage             │ What It Proves               │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C0   │ BFV Public Key             │ DKG: Key Gen      │ BFV keypair generated        │
│      │                            │                   │ correctly                    │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C1   │ TrBFV PK Generation        │ DKG: Share Gen    │ Threshold pk_share derived   │
│      │                            │                   │ correctly from sk; outputs   │
│      │                            │                   │ sk_commitment, pk_commitment,│
│      │                            │                   │ e_sm_commitment              │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C2a  │ SK Share Computation       │ DKG: Share Gen    │ Shamir shares of sk computed │
│      │                            │                   │ correctly                    │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C2b  │ ESM Share Computation      │ DKG: Share Gen    │ Shamir shares of smudging    │
│      │                            │                   │ noise computed correctly     │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C3a  │ SK Share Encryption        │ DKG: Share Gen    │ sk_sss encrypted correctly   │
│      │                            │                   │ under recipient's BFV key   │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C3b  │ ESM Share Encryption       │ DKG: Share Gen    │ esi_sss encrypted correctly  │
│      │                            │                   │ under recipient's BFV key   │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C4a  │ SK Decryption Share (T2)   │ DKG: Key Calc     │ Verifies H decrypted shares  │
│      │                            │                   │ match C2a commitments; sums  │
│      │                            │                   │ and normalises (reduce mod   │
│      │                            │                   │ q, reverse, center) before   │
│      │                            │                   │ hashing; output commitment   │
│      │                            │                   │ consumed by C6               │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C4b  │ ESM Decryption Share (T2)  │ DKG: Key Calc     │ Same as C4a for e_sm branch; │
│      │                            │                   │ output commitment consumed   │
│      │                            │                   │ by C6                        │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C5   │ PK Aggregation             │ Aggregation       │ Aggregate PK correctly       │
│      │                            │                   │ computed from all pk_shares  │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C6   │ Threshold Share Decryption │ Decryption        │ Decryption share correctly   │
│      │ (T5)                       │                   │ derived from sk + ciphertext;│
│      │                            │                   │ public output: commitment to │
│      │                            │                   │ first MAX_MSG_NON_ZERO_COEFFS│
│      │                            │                   │ coeffs of d per CRT limb     │
├──────┼────────────────────────────┼───────────────────┼──────────────────────────────┤
│ C7   │ Decrypted Shares Agg.      │ Final Aggregation │ Plaintext correctly          │
│      │                            │                   │ reconstructed from shares    │
│      │                            │                   │ (modular decode over t);     │
│      │                            │                   │ public inputs: C6 `d`         │
│      │                            │                   │ commitments + party IDs + msg;│
│      │                            │                   │ in-circuit equality vs        │
│      │                            │                   │ commitments from witness      │
│      │                            │                   │ decryption shares             │
└──────┴────────────────────────────┴───────────────────┴──────────────────────────────┘

Slash Reasons by Proof Type:
  C0–C4:  E3_BAD_DKG_PROOF
  C5:     E3_BAD_PK_AGGREGATION_PROOF
  C6:     E3_BAD_DECRYPTION_PROOF
  C7:     E3_BAD_AGGREGATION_PROOF
```

### Proof Infrastructure

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ZK Proof Infrastructure                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ProofRequestActor (Business Layer)                                 │
│  ├─ Subscribes to *Pending events (proof requests)                 │
│  ├─ Dispatches ComputeRequest::zk to ZkActor                      │
│  ├─ Collects responses, signs proofs (ECDSA EIP-191)               │
│  ├─ Manages pending proof state (C1–C3 and C4 proof bundles per E3) │
│  └─ Publishes *Created / *Signed events when all proofs complete   │
│                                                                     │
│  ProofVerificationActor (C0 Verification)                          │
│  ├─ EncryptionKeyReceived → ECDSA recovery + ZK verify            │
│  ├─ On pass → EncryptionKeyCreated (locally trusted)               │
│  └─ On fail → SignedProofFailed → AccusationManager                │
│                                                                     │
│  ShareVerificationActor (C2/C3/C4/C6 Verification)                │
│  ├─ Two-phase: ECDSA inline + ZK dispatched to multithread        │
│  ├─ Defense-in-depth: cross-checks dispatched vs returned parties  │
│  └─ On fail → SignedProofFailed → AccusationManager                │
│                                                                     │
│  ZkActor (IO Layer)                                                │
│  ├─ Manages Barretenberg (bb) binary and circuit files             │
│  ├─ Spawns child processes: bb prove / bb verify                   │
│  └─ Returns Proof { data, public_signals }                         │
│                                                                     │
│  AccusationManager (see Part 5 for full detail)                    │
│  ├─ Receives SignedProofFailed → creates accusations               │
│  ├─ Off-chain voting quorum among committee members                │
│  └─ AccusationQuorumReached → on-chain slash submission            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Complete DKG Data Flow

```
Party 1                    Party 2                    Party 3
───────                    ───────                    ───────
Generate BFV keypair       Generate BFV keypair       Generate BFV keypair
  (sk₁, pk₁)                (sk₂, pk₂)                (sk₃, pk₃)

Broadcast pk₁ ──────────→ Receive pk₁ ──────────→ Receive pk₁
Receive pk₂ ←──────────── Broadcast pk₂ ──────────→ Receive pk₂
Receive pk₃ ←──────────── Receive pk₃ ←──────────── Broadcast pk₃

Generate TrBFV key:        Generate TrBFV key:        Generate TrBFV key:
  (SK₁, PK_share₁)          (SK₂, PK_share₂)          (SK₃, PK_share₃)

Shamir split SK₁:         Shamir split SK₂:         Shamir split SK₃:
  s₁₁, s₁₂, s₁₃            s₂₁, s₂₂, s₂₃            s₃₁, s₃₂, s₃₃

Encrypt & send:            Encrypt & send:            Encrypt & send:
  Enc(s₁₂, pk₂) → P2        Enc(s₂₁, pk₁) → P1        Enc(s₃₁, pk₁) → P1
  Enc(s₁₃, pk₃) → P3        Enc(s₂₃, pk₃) → P3        Enc(s₃₂, pk₂) → P2

Receive & decrypt:         Receive & decrypt:         Receive & decrypt:
  s₂₁ = Dec(_, sk₁)         s₁₂ = Dec(_, sk₂)         s₁₃ = Dec(_, sk₃)
  s₃₁ = Dec(_, sk₁)         s₃₂ = Dec(_, sk₂)         s₂₃ = Dec(_, sk₃)

Reconstruct:               Reconstruct:               Reconstruct:
  dk₁ = sum(s₁₁,s₂₁,s₃₁)   dk₂ = sum(s₁₂,s₂₂,s₃₂)   dk₃ = sum(s₁₃,s₂₃,s₃₃)

═══════════════════════════════════════════════════════════════
Each party now has dk_i (decryption key portion)
No party knows the full secret key
Any M+1 parties can collaboratively decrypt

ACTIVE AGGREGATOR collects PK_share₁ + PK_share₂ + PK_share₃
  → Produces aggregate_PK (public, published on-chain)
  → Anyone can encrypt, only committee can decrypt
```
