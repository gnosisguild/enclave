// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::{anyhow, bail, Context, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_data::Persistable;
use e3_events::{
    prelude::*, trap, BusHandle, CiphernodeSelected, CiphertextOutputPublished, ComputeRequest,
    ComputeResponse, ComputeResponseKind, CorrelationId, DecryptionKeyShared,
    DecryptionShareProofsPending, DecryptionshareCreated, Die, DkgProofSigned,
    DkgShareDecryptionProofRequest, E3Failed, E3RequestComplete, E3Stage, E3id, EType,
    EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCollectionFailed,
    EncryptionKeyCreated, EncryptionKeyPending, EventContext, FailureReason, KeyshareCreated,
    PartyId, PartyProofsToVerify, PartyShareDecryptionProofsToVerify, PkGenerationProofRequest,
    PkGenerationProofSigned, ProofType, Sequenced, ShareComputationProofRequest,
    ShareEncryptionProofRequest, ShareVerificationComplete, ShareVerificationDispatched,
    SignedProofPayload, ThresholdShare, ThresholdShareCollectionFailed, ThresholdShareCreated,
    ThresholdSharePending, TypedEvent, VerificationKind,
};
use e3_fhe_params::create_deterministic_crp_from_default_seed;
use e3_fhe_params::{build_pair_for_preset, BfvParamSet, BfvPreset};
use e3_trbfv::{
    calculate_decryption_key::{CalculateDecryptionKeyRequest, CalculateDecryptionKeyResponse},
    calculate_decryption_share::{
        CalculateDecryptionShareRequest, CalculateDecryptionShareResponse,
    },
    gen_esi_sss::{GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::{GenPkShareAndSkSssRequest, GenPkShareAndSkSssResponse},
    helpers::{deserialize_secret_key, serialize_secret_key},
    shares::{BfvEncryptedShares, EncryptableVec, Encrypted, ShamirShare, SharedSecret},
    TrBFVConfig, TrBFVRequest, TrBFVResponse,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::CiphernodesCommitteeSize;
use fhe::bfv::{PublicKey, SecretKey};
use fhe_traits::{DeserializeParametrized, Serialize};
use rand::rngs::OsRng;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    mem,
    sync::Arc,
};
use tracing::{error, info, trace, warn};

use crate::decryption_key_shared_collector::{
    AllDecryptionKeySharesCollected, DecryptionKeySharedCollectionFailed,
    DecryptionKeySharedCollector,
};
use crate::encryption_key_collector::{AllEncryptionKeysCollected, EncryptionKeyCollector};
use crate::threshold_share_collector::{ReceivedShareProofs, ThresholdShareCollector};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub struct GenPkShareAndSkSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub struct GenEsiSss {
    pub ciphernode_selected: CiphernodeSelected,
    pub e_sm_raw: SensitiveBytes,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AllThresholdSharesCollected {
    shares: Vec<Arc<ThresholdShare>>,
    /// Proofs from each sender, ordered by party_id (parallel to shares).
    share_proofs: Vec<ReceivedShareProofs>,
}

impl AllThresholdSharesCollected {
    pub fn new(
        shares: HashMap<u64, Arc<ThresholdShare>>,
        proofs: HashMap<u64, ReceivedShareProofs>,
    ) -> Self {
        let mut entries: Vec<_> = shares.into_iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        let (party_ids, shares): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let share_proofs = party_ids
            .iter()
            .map(|pid| {
                proofs.get(pid).cloned().unwrap_or(ReceivedShareProofs {
                    signed_c2a_proof: None,
                    signed_c2b_proof: None,
                    signed_c3a_proofs: Vec::new(),
                    signed_c3b_proofs: Vec::new(),
                })
            })
            .collect();
        Self {
            shares,
            share_proofs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CollectingEncryptionKeysData {
    sk_bfv: SensitiveBytes,
    pk_bfv: ArcBytes,
    ciphernode_selected: CiphernodeSelected,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProofRequestData {
    pub pk0_share_raw: ArcBytes,
    pub sk_raw: SensitiveBytes,
    pub eek_raw: SensitiveBytes,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneratingThresholdShareData {
    pk_share: Option<ArcBytes>,
    sk_sss: Option<Encrypted<SharedSecret>>,
    esi_sss: Option<Vec<Encrypted<SharedSecret>>>,
    e_sm_raw: Option<SensitiveBytes>,
    sk_bfv: SensitiveBytes,
    pk_bfv: ArcBytes,
    collected_encryption_keys: Vec<Arc<EncryptionKey>>,
    ciphernode_selected: Option<CiphernodeSelected>,
    proof_request_data: Option<ProofRequestData>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AggregatingDecryptionKey {
    pk_share: ArcBytes,
    sk_bfv: SensitiveBytes,
    signed_pk_generation_proof: Option<SignedProofPayload>,
    signed_sk_share_computation_proof: Option<SignedProofPayload>,
    signed_e_sm_share_computation_proof: Option<SignedProofPayload>,
    signed_sk_share_encryption_proofs: Vec<SignedProofPayload>,
    signed_e_sm_share_encryption_proofs: Vec<SignedProofPayload>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReadyForDecryption {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
    signed_pk_generation_proof: Option<SignedProofPayload>,
    signed_sk_share_computation_proof: Option<SignedProofPayload>,
    signed_e_sm_share_computation_proof: Option<SignedProofPayload>,
    signed_sk_share_encryption_proofs: Vec<SignedProofPayload>,
    signed_e_sm_share_encryption_proofs: Vec<SignedProofPayload>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decrypting {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
    signed_pk_generation_proof: Option<SignedProofPayload>,
    signed_sk_share_computation_proof: Option<SignedProofPayload>,
    signed_e_sm_share_computation_proof: Option<SignedProofPayload>,
    signed_sk_share_encryption_proofs: Vec<SignedProofPayload>,
    signed_e_sm_share_encryption_proofs: Vec<SignedProofPayload>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum KeyshareState {
    // Before anything
    Init,
    // Collecting BFV encryption keys from all parties
    CollectingEncryptionKeys(CollectingEncryptionKeysData),
    // Generating TrBFV share material
    GeneratingThresholdShare(GeneratingThresholdShareData),
    // Collecting remaining TrBFV shares to aggregate decryption key
    AggregatingDecryptionKey(AggregatingDecryptionKey),
    // Awaiting decryption
    ReadyForDecryption(ReadyForDecryption),
    // Decrypting something
    Decrypting(Decrypting),
    // Finished
    Completed,
}

impl KeyshareState {
    pub fn next(self: &KeyshareState, new_state: KeyshareState) -> Result<KeyshareState> {
        use KeyshareState as K;
        // The following can be used to check that we are transitioning to a valid state
        let valid = {
            // If we are in the same branch the new state is valid
            if mem::discriminant(self) == mem::discriminant(&new_state) {
                true
            } else {
                match (self, &new_state) {
                    (K::Init, K::CollectingEncryptionKeys(_)) => true,
                    (K::CollectingEncryptionKeys(_), K::GeneratingThresholdShare(_)) => true,
                    (K::GeneratingThresholdShare(_), K::AggregatingDecryptionKey(_)) => true,
                    (K::AggregatingDecryptionKey(_), K::ReadyForDecryption(_)) => true,
                    (K::ReadyForDecryption(_), K::Decrypting(_)) => true,
                    (K::Decrypting(_), K::Completed) => true,
                    _ => false,
                }
            }
        };

        if valid {
            Ok(new_state)
        } else {
            Err(anyhow!(
                "Bad state transition {:?} -> {:?}",
                self.variant_name(),
                new_state.variant_name()
            ))
        }
    }
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Init => "Init",
            Self::CollectingEncryptionKeys(_) => "CollectingEncryptionKeys",
            Self::GeneratingThresholdShare(_) => "GeneratingThresholdShare",
            Self::AggregatingDecryptionKey(_) => "AggregatingDecryptionKey",
            Self::ReadyForDecryption(_) => "ReadyForDecryption",
            Self::Decrypting(_) => "Decrypting",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThresholdKeyshareState {
    pub e3_id: E3id,
    pub address: String,
    pub party_id: PartyId,
    pub state: KeyshareState,
    pub threshold_m: u64,
    pub threshold_n: u64,
    pub params: ArcBytes,
}

impl ThresholdKeyshareState {
    pub fn new(
        e3_id: E3id,
        party_id: PartyId,
        state: KeyshareState,
        threshold_m: u64,
        threshold_n: u64,
        params: ArcBytes,
        address: String,
    ) -> Self {
        Self {
            e3_id,
            address,
            party_id,
            state,
            threshold_m,
            threshold_n,
            params,
        }
    }

    /// Return a valid Self based on a new state struct.
    pub fn new_state(self, new_state: KeyshareState) -> Result<Self> {
        Ok(ThresholdKeyshareState {
            state: self.state.next(new_state)?,
            ..self
        })
    }

    pub fn get_trbfv_config(&self) -> TrBFVConfig {
        TrBFVConfig::new(self.params.clone(), self.threshold_n, self.threshold_m)
    }

    pub fn get_e3_id(&self) -> &E3id {
        &self.e3_id
    }

    pub fn get_party_id(&self) -> PartyId {
        self.party_id
    }

    pub fn get_threshold_m(&self) -> u64 {
        self.threshold_m
    }

    pub fn get_threshold_n(&self) -> u64 {
        self.threshold_n
    }

    pub fn get_params(&self) -> &ArcBytes {
        &self.params
    }

    pub fn get_address(&self) -> &str {
        &self.address
    }

    pub fn variant_name(&self) -> &str {
        self.state.variant_name()
    }
}

impl TryInto<CollectingEncryptionKeysData> for ThresholdKeyshareState {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<CollectingEncryptionKeysData, Self::Error> {
        match self.state {
            KeyshareState::CollectingEncryptionKeys(s) => Ok(s),
            _ => Err(anyhow!("Invalid state: expected CollectingEncryptionKeys")),
        }
    }
}

impl TryInto<GeneratingThresholdShareData> for ThresholdKeyshareState {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<GeneratingThresholdShareData, Self::Error> {
        match self.state {
            KeyshareState::GeneratingThresholdShare(s) => Ok(s),
            _ => Err(anyhow!("Invalid state")),
        }
    }
}

impl TryInto<AggregatingDecryptionKey> for ThresholdKeyshareState {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<AggregatingDecryptionKey, Self::Error> {
        match self.state {
            KeyshareState::AggregatingDecryptionKey(s) => Ok(s),
            _ => Err(anyhow!("Invalid state")),
        }
    }
}

impl TryInto<ReadyForDecryption> for ThresholdKeyshareState {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<ReadyForDecryption, Self::Error> {
        match self.state {
            KeyshareState::ReadyForDecryption(s) => Ok(s),
            _ => Err(anyhow!("Invalid state")),
        }
    }
}

impl TryInto<Decrypting> for ThresholdKeyshareState {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<Decrypting, Self::Error> {
        match self.state {
            KeyshareState::Decrypting(s) => Ok(s),
            _ => Err(anyhow!("Invalid state")),
        }
    }
}

pub struct ThresholdKeyshareParams {
    pub bus: BusHandle,
    pub cipher: Arc<Cipher>,
    pub state: Persistable<ThresholdKeyshareState>,
    pub share_enc_preset: BfvPreset,
}

pub struct ThresholdKeyshare {
    bus: BusHandle,
    cipher: Arc<Cipher>,
    decryption_key_collector: Option<Addr<ThresholdShareCollector>>,
    encryption_key_collector: Option<Addr<EncryptionKeyCollector>>,
    decryption_key_shared_collector: Option<Addr<DecryptionKeySharedCollector>>,
    state: Persistable<ThresholdKeyshareState>,
    share_enc_preset: BfvPreset,
    /// Transient coordination data bridging async gaps — not persisted.
    /// Shares pending C2/C3 verification, consumed in `proceed_with_decryption_key_calculation`.
    pending_shares: Vec<ThresholdShare>,
    /// C4 proof data built during aggregation, consumed after CalculateDecryptionKey.
    pending_c4_proof_data: Option<(
        DkgShareDecryptionProofRequest,
        Vec<DkgShareDecryptionProofRequest>,
    )>,
    /// Honest party IDs determined by C2/C3 verification, narrowed by C4.
    honest_parties: Option<HashSet<u64>>,
    /// DecryptionKeyShared events arriving before ReadyForDecryption.
    early_decryption_key_shares: HashMap<u64, DecryptionKeyShared>,
}

impl ThresholdKeyshare {
    pub fn new(params: ThresholdKeyshareParams) -> Self {
        Self {
            bus: params.bus,
            cipher: params.cipher,
            decryption_key_collector: None,
            encryption_key_collector: None,
            decryption_key_shared_collector: None,
            state: params.state,
            share_enc_preset: params.share_enc_preset,
            pending_shares: Vec::new(),
            pending_c4_proof_data: None,
            honest_parties: None,
            early_decryption_key_shares: HashMap::new(),
        }
    }
}

impl Actor for ThresholdKeyshare {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl ThresholdKeyshare {
    pub fn ensure_collector(
        &mut self,
        self_addr: Addr<Self>,
    ) -> Result<Addr<ThresholdShareCollector>> {
        let Some(state) = self.state.get() else {
            bail!("State not found on threshold keyshare. This should not happen.");
        };

        info!(
            "Setting up key collector for addr: {} and {} nodes",
            state.address, state.threshold_n
        );
        let e3_id = state.e3_id.clone();
        let threshold_n = state.threshold_n;
        let addr = self
            .decryption_key_collector
            .get_or_insert_with(|| ThresholdShareCollector::setup(self_addr, threshold_n, e3_id));
        Ok(addr.clone())
    }

    pub fn ensure_encryption_key_collector(
        &mut self,
        self_addr: Addr<Self>,
    ) -> Result<Addr<EncryptionKeyCollector>> {
        let Some(state) = self.state.get() else {
            bail!("State not found on threshold keyshare. This should not happen.");
        };

        info!(
            "Setting up encryption key collector for addr: {} and {} nodes",
            state.address, state.threshold_n
        );
        let e3_id = state.e3_id.clone();
        let threshold_n = state.threshold_n;
        let addr = self
            .encryption_key_collector
            .get_or_insert_with(|| EncryptionKeyCollector::setup(self_addr, threshold_n, e3_id));
        Ok(addr.clone())
    }

    /// Create or return the DecryptionKeySharedCollector.
    /// Uses honest_parties from the struct.
    pub fn ensure_decryption_key_shared_collector(
        &mut self,
        self_addr: Addr<Self>,
    ) -> Result<Addr<DecryptionKeySharedCollector>> {
        let state = self.state.try_get()?;
        let my_party_id = state.party_id;

        let honest = self
            .honest_parties
            .as_ref()
            .ok_or_else(|| anyhow!("honest_parties not set when creating collector"))?;

        let expected: HashSet<u64> = honest
            .iter()
            .filter(|&&pid| pid != my_party_id)
            .copied()
            .collect();

        let e3_id = state.e3_id.clone();
        let addr = self
            .decryption_key_shared_collector
            .get_or_insert_with(|| DecryptionKeySharedCollector::setup(self_addr, expected, e3_id));
        Ok(addr.clone())
    }

    pub fn handle_threshold_share_created(
        &mut self,
        msg: TypedEvent<ThresholdShareCreated>,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        let state = self.state.try_get()?;
        let my_party_id = state.party_id;

        // Filter: only process shares intended for this party
        if msg.target_party_id != my_party_id {
            return Ok(());
        }

        info!(
            "Received ThresholdShareCreated from party {} for us (party {}), forwarding to collector!",
            msg.share.party_id, my_party_id
        );
        let collector = self.ensure_collector(self_addr)?;
        info!("got collector address!");
        collector.do_send(msg);
        Ok(())
    }

    pub fn handle_encryption_key_created(
        &mut self,
        msg: TypedEvent<EncryptionKeyCreated>,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        info!("Received EncryptionKeyCreated forwarding to encryption key collector!");
        let collector = self.ensure_encryption_key_collector(self_addr)?;
        collector.do_send(msg);
        Ok(())
    }

    /// Handle PkGenerationProofSigned - stores the signed C1 proof in state
    pub fn handle_pk_generation_proof_signed(
        &mut self,
        msg: TypedEvent<PkGenerationProofSigned>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;

        // Only accept proof for our own party
        if msg.party_id != state.party_id {
            return Ok(());
        }

        info!(
            "Received PkGenerationProofSigned for party {} E3 {}",
            msg.party_id, msg.e3_id
        );

        // Store the signed proof in AggregatingDecryptionKey state
        self.state.try_mutate(&ec, |s| {
            let current: AggregatingDecryptionKey = s.clone().try_into()?;
            s.new_state(KeyshareState::AggregatingDecryptionKey(
                AggregatingDecryptionKey {
                    signed_pk_generation_proof: Some(msg.signed_proof),
                    ..current
                },
            ))
        })?;

        Ok(())
    }

    /// Handle DkgProofSigned - stores the signed proof in state based on proof type (C2a, C2b, C3a or C3b)
    pub fn handle_share_computation_proof_signed(
        &mut self,
        msg: TypedEvent<DkgProofSigned>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;

        if msg.party_id != state.party_id {
            return Ok(());
        }

        let proof_type = msg.signed_proof.payload.proof_type;
        info!(
            "Received DkgProofSigned ({:?}) for party {} E3 {}",
            proof_type, msg.party_id, msg.e3_id
        );

        self.state.try_mutate(&ec, |s| {
            let current: AggregatingDecryptionKey = s.clone().try_into()?;
            let updated = match proof_type {
                ProofType::C2aSkShareComputation => AggregatingDecryptionKey {
                    signed_sk_share_computation_proof: Some(msg.signed_proof),
                    ..current
                },
                ProofType::C2bESmShareComputation => AggregatingDecryptionKey {
                    signed_e_sm_share_computation_proof: Some(msg.signed_proof),
                    ..current
                },
                ProofType::C3aSkShareEncryption => {
                    let mut updated = current;
                    updated
                        .signed_sk_share_encryption_proofs
                        .push(msg.signed_proof);
                    updated
                }
                ProofType::C3bESmShareEncryption => {
                    let mut updated = current;
                    updated
                        .signed_e_sm_share_encryption_proofs
                        .push(msg.signed_proof);
                    updated
                }
                other => {
                    warn!("Unexpected proof type {:?} in DkgProofSigned", other);
                    current
                }
            };
            s.new_state(KeyshareState::AggregatingDecryptionKey(updated))
        })?;

        Ok(())
    }

    pub fn handle_compute_response(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        match &msg.response {
            ComputeResponseKind::TrBFV(trbfv) => match trbfv {
                TrBFVResponse::GenEsiSss(_) => self.handle_gen_esi_sss_response(msg),
                TrBFVResponse::GenPkShareAndSkSss(_) => {
                    self.handle_gen_pk_share_and_sk_sss_response(msg)
                }
                TrBFVResponse::CalculateDecryptionKey(_) => {
                    self.handle_calculate_decryption_key_response(msg, self_addr)
                }
                TrBFVResponse::CalculateDecryptionShare(_) => {
                    self.handle_calculate_decryption_share_response(msg)
                }
                _ => Ok(()),
            },
            // ZK responses (C4 proofs, share/decryption verification) are now
            // handled by ProofRequestActor and ShareVerificationActor respectively.
            _ => Ok(()),
        }
    }

    /// 1. CiphernodeSelected - Generate BFV keys and publish EncryptionKeyPending
    pub fn handle_ciphernode_selected(
        &mut self,
        msg: TypedEvent<CiphernodeSelected>,
        address: Addr<Self>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("CiphernodeSelected received.");
        // Ensure the collectors are created
        let _ = self.ensure_collector(address.clone());
        let _ = self.ensure_encryption_key_collector(address.clone());

        let params = BfvParamSet::from(self.share_enc_preset.clone()).build_arc();
        let mut rng = OsRng;
        let sk_bfv = SecretKey::random(&params, &mut rng);
        let pk_bfv = PublicKey::new(&sk_bfv, &mut rng);

        let sk_bytes = serialize_secret_key(&sk_bfv)?;
        let sk_bfv_encrypted = SensitiveBytes::new(sk_bytes, &self.cipher)?;
        let pk_bfv_bytes = ArcBytes::from_bytes(&pk_bfv.to_bytes());

        let state = self.state.try_get()?;
        let e3_id = state.e3_id.clone();

        self.state.try_mutate(&ec, |s| {
            s.new_state(KeyshareState::CollectingEncryptionKeys(
                CollectingEncryptionKeysData {
                    sk_bfv: sk_bfv_encrypted.clone(),
                    pk_bfv: pk_bfv_bytes.clone(),
                    ciphernode_selected: msg,
                },
            ))
        })?;

        self.bus.publish(
            EncryptionKeyPending {
                e3_id,
                key: Arc::new(EncryptionKey::new(state.party_id, pk_bfv_bytes)),
                params_preset: self.share_enc_preset,
            },
            ec,
        )?;

        Ok(())
    }

    /// 1a. AllEncryptionKeysCollected - All BFV keys received, start share generation
    pub fn handle_all_encryption_keys_collected(
        &mut self,
        msg: TypedEvent<AllEncryptionKeysCollected>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!(
            "AllEncryptionKeysCollected - {} keys received",
            msg.keys.len()
        );

        let current: CollectingEncryptionKeysData = self.state.try_get()?.try_into()?;

        self.state.try_mutate(&ec, |s| {
            s.new_state(KeyshareState::GeneratingThresholdShare(
                GeneratingThresholdShareData {
                    sk_sss: None,
                    pk_share: None,
                    esi_sss: None,
                    e_sm_raw: None,
                    sk_bfv: current.sk_bfv,
                    pk_bfv: current.pk_bfv,
                    collected_encryption_keys: msg.keys,
                    ciphernode_selected: Some(current.ciphernode_selected.clone()),
                    proof_request_data: None,
                },
            ))
        })?;

        self.handle_gen_pk_share_and_sk_sss_requested(TypedEvent::new(
            GenPkShareAndSkSss(current.ciphernode_selected),
            ec,
        ))?;

        Ok(())
    }

    /// 2. GenPkShareAndSkSss
    pub fn handle_gen_pk_share_and_sk_sss_requested(
        &self,
        msg: TypedEvent<GenPkShareAndSkSss>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("GenPkShareAndSkSss on ThresholdKeyshare");
        let CiphernodeSelected { e3_id, .. } = msg.0;
        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config: TrBFVConfig = state.get_trbfv_config();

        let crp = ArcBytes::from_bytes(
            &create_deterministic_crp_from_default_seed(&trbfv_config.params()).to_bytes(),
        );

        let threshold_preset = self
            .share_enc_preset
            .threshold_counterpart()
            .ok_or_else(|| anyhow!("No threshold counterpart for {:?}", self.share_enc_preset))?;
        let defaults = threshold_preset
            .search_defaults()
            .ok_or_else(|| anyhow!("No search defaults for {:?}", threshold_preset))?;

        let event = ComputeRequest::trbfv(
            TrBFVRequest::GenPkShareAndSkSss(
                GenPkShareAndSkSssRequest {
                    trbfv_config,
                    crp,
                    lambda: defaults.lambda as usize,
                    num_ciphertexts: defaults.z as usize,
                }
                .into(),
            ),
            CorrelationId::new(),
            e3_id,
        );

        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// 2a. GenPkShareAndSkSss result
    pub fn handle_gen_pk_share_and_sk_sss_response(
        &mut self,
        res: TypedEvent<ComputeResponse>,
    ) -> Result<()> {
        let (res, ec) = res.into_components();

        let output: GenPkShareAndSkSssResponse = res
            .try_into()
            .context("Error extracting data from compute process")?;

        let (pk_share, sk_sss, e_sm_raw) = (
            output.pk_share.clone(),
            output.sk_sss,
            output.e_sm_raw.clone(),
        );

        // Store proof request data for later use by ProofRequestActor
        let proof_request_data = ProofRequestData {
            pk0_share_raw: output.pk0_share_raw,
            sk_raw: output.sk_raw,
            eek_raw: output.eek_raw,
        };

        self.state.try_mutate(&ec, |s| {
            info!("try_store_pk_share_and_sk_sss");
            let current: GeneratingThresholdShareData = s.clone().try_into()?;
            s.new_state(KeyshareState::GeneratingThresholdShare(
                GeneratingThresholdShareData {
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                    e_sm_raw: Some(e_sm_raw.clone()),
                    proof_request_data: Some(proof_request_data),
                    ..current
                },
            ))
        })?;

        // Fire gen_esi_sss with the e_sm_raw
        let current_state: GeneratingThresholdShareData = self.state.try_get()?.try_into()?;
        if let Some(ciphernode_selected) = current_state.ciphernode_selected {
            self.handle_gen_esi_sss_requested(TypedEvent::new(
                GenEsiSss {
                    ciphernode_selected,
                    e_sm_raw: current_state
                        .e_sm_raw
                        .expect("e_sm_raw should be set at this point"),
                },
                ec.clone(),
            ))?;
        }

        Ok(())
    }

    /// 3. GenEsiSss
    pub fn handle_gen_esi_sss_requested(&self, msg: TypedEvent<GenEsiSss>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("GenEsiSss on ThresholdKeyshare");

        let e_sm_raw = msg.e_sm_raw;
        let CiphernodeSelected { e3_id, .. } = msg.ciphernode_selected;

        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config = state.get_trbfv_config();

        let event = ComputeRequest::trbfv(
            TrBFVRequest::GenEsiSss(GenEsiSssRequest {
                trbfv_config,
                e_sm_raw,
            }),
            CorrelationId::new(),
            e3_id,
        );

        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// 3a. GenEsiSss result
    pub fn handle_gen_esi_sss_response(&mut self, res: TypedEvent<ComputeResponse>) -> Result<()> {
        let (res, ec) = res.into_components();
        let output: GenEsiSssResponse = res.try_into()?;

        let esi_sss = output.esi_sss;

        // First store esi_sss in GeneratingThresholdShareData
        self.state.try_mutate(&ec, |s| {
            info!("try_store_esi_sss");
            let current: GeneratingThresholdShareData = s.clone().try_into()?;
            s.new_state(KeyshareState::GeneratingThresholdShare(
                GeneratingThresholdShareData {
                    esi_sss: Some(esi_sss),
                    ..current
                },
            ))
        })?;

        info!("esi stored");

        // Check if all data is ready, if so call handle_shares_generated BEFORE transitioning
        let current: GeneratingThresholdShareData = self.state.try_get()?.try_into()?;
        let ready = current.pk_share.is_some()
            && current.sk_sss.is_some()
            && current.esi_sss.is_some()
            && current.e_sm_raw.is_some()
            && current.proof_request_data.is_some();

        if ready {
            // Call handle_shares_generated while still in GeneratingThresholdShare state
            self.handle_shares_generated(ec.clone())?;

            // Now transition to AggregatingDecryptionKey with minimal state
            self.state.try_mutate(&ec, |s| {
                let current: GeneratingThresholdShareData = s.clone().try_into()?;
                s.new_state(KeyshareState::AggregatingDecryptionKey(
                    AggregatingDecryptionKey {
                        pk_share: current.pk_share.expect("pk_share checked above"),
                        sk_bfv: current.sk_bfv,
                        signed_pk_generation_proof: None,
                        signed_sk_share_computation_proof: None,
                        signed_e_sm_share_computation_proof: None,
                        signed_sk_share_encryption_proofs: Vec::new(),
                        signed_e_sm_share_encryption_proofs: Vec::new(),
                    },
                ))
            })?;
        }
        Ok(())
    }

    /// 4. SharesGenerated - Encrypt shares with BFV and publish
    pub fn handle_shares_generated(&mut self, ec: EventContext<Sequenced>) -> Result<()> {
        let Some(ThresholdKeyshareState {
            state:
                KeyshareState::GeneratingThresholdShare(GeneratingThresholdShareData {
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                    esi_sss: Some(esi_sss),
                    e_sm_raw: Some(e_sm_raw),
                    proof_request_data: Some(proof_request_data),
                    collected_encryption_keys,
                    ..
                }),
            party_id,
            e3_id,
            ..
        }) = self.state.get()
        else {
            bail!("Invalid state - expected GeneratingThresholdShare with all data");
        };

        // Get collected BFV public keys from all parties (from persisted state)
        let encryption_keys = &collected_encryption_keys;

        // Convert to BFV public keys using DKG params
        let threshold_preset = self
            .share_enc_preset
            .threshold_counterpart()
            .ok_or_else(|| anyhow!("No threshold counterpart for {:?}", self.share_enc_preset))?;
        let (_, params) = build_pair_for_preset(threshold_preset)?;
        let recipient_pks: Vec<PublicKey> = encryption_keys
            .iter()
            .map(|k| {
                PublicKey::from_bytes(&k.pk_bfv, &params)
                    .map_err(|e| anyhow!("Failed to deserialize BFV public key: {:?}", e))
            })
            .collect::<Result<_>>()?;

        // Decrypt our shares from local storage
        let decrypted_sk_sss: SharedSecret = sk_sss.decrypt(&self.cipher)?;
        let decrypted_esi_sss: Vec<SharedSecret> = esi_sss
            .into_iter()
            .map(|s| s.decrypt(&self.cipher))
            .collect::<Result<_>>()?;

        // Serialize for C2a/C2b proof requests (encrypted at rest)
        let sk_sss_raw = SensitiveBytes::new(
            bincode::serialize(&decrypted_sk_sss)
                .map_err(|e| anyhow!("Failed to serialize sk_sss: {}", e))?,
            &self.cipher,
        )?;
        let esi_sss_raw: Vec<SensitiveBytes> = decrypted_esi_sss
            .iter()
            .map(|s| {
                let bytes = bincode::serialize(s)
                    .map_err(|e| anyhow!("Failed to serialize esi_sss: {}", e))?;
                SensitiveBytes::new(bytes, &self.cipher)
            })
            .collect::<Result<_>>()?;

        // Encrypt shares for all recipients using BFV (extended to capture randomness for C3 proofs)
        let mut rng = OsRng;
        let (encrypted_sk_sss, sk_witnesses) = BfvEncryptedShares::encrypt_all_extended(
            &decrypted_sk_sss,
            &recipient_pks,
            &params,
            &mut rng,
        )?;

        let (encrypted_esi_sss, esi_witnesses): (Vec<_>, Vec<_>) = decrypted_esi_sss
            .iter()
            .map(|esi| {
                BfvEncryptedShares::encrypt_all_extended(esi, &recipient_pks, &params, &mut rng)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .unzip();

        // Create the full share with all parties' encrypted data
        let full_share = ThresholdShare {
            party_id,
            pk_share,
            sk_sss: encrypted_sk_sss,
            esi_sss: encrypted_esi_sss,
        };

        // Build C1 request (PkGenerationProof)
        let proof_request = PkGenerationProofRequest::new(
            proof_request_data.pk0_share_raw.clone(),
            proof_request_data.sk_raw.clone(),
            proof_request_data.eek_raw.clone(),
            e_sm_raw.clone(),
            threshold_preset,
            CiphernodesCommitteeSize::Small, // TODO: derive from config
        );

        // Build C2a request (SkShareComputation)
        let sk_share_computation_request = ShareComputationProofRequest {
            secret_raw: proof_request_data.sk_raw.clone(),
            secret_sss_raw: sk_sss_raw,
            dkg_input_type: DkgInputType::SecretKey,
            params_preset: threshold_preset,
            committee_size: CiphernodesCommitteeSize::Small, // TODO: derive from config
        };

        // Build C2b request (ESmShareComputation)
        let e_sm_share_computation_request = ShareComputationProofRequest {
            secret_raw: e_sm_raw.clone(),
            secret_sss_raw: esi_sss_raw
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("esi_sss_raw is empty — expected at least one entry"))?,
            dkg_input_type: DkgInputType::SmudgingNoise,
            params_preset: threshold_preset,
            committee_size: CiphernodesCommitteeSize::Small, // TODO: derive from config
        };

        // Build C3a proof requests (SK share encryption) from witnesses
        let mut sk_share_encryption_requests = Vec::new();
        for (recipient_idx, recipient_witnesses) in sk_witnesses.iter().enumerate() {
            for (row_idx, witness) in recipient_witnesses.iter().enumerate() {
                sk_share_encryption_requests.push(ShareEncryptionProofRequest {
                    share_row_raw: SensitiveBytes::new(
                        bincode::serialize(&witness.share_row)
                            .map_err(|e| anyhow!("Failed to serialize share_row: {}", e))?,
                        &self.cipher,
                    )?,
                    ciphertext_raw: ArcBytes::from_bytes(&witness.ciphertext.to_bytes()),
                    recipient_pk_raw: ArcBytes::from_bytes(
                        &recipient_pks[recipient_idx].to_bytes(),
                    ),
                    u_rns_raw: SensitiveBytes::new(witness.u_rns.to_bytes(), &self.cipher)?,
                    e0_rns_raw: SensitiveBytes::new(witness.e0_rns.to_bytes(), &self.cipher)?,
                    e1_rns_raw: SensitiveBytes::new(witness.e1_rns.to_bytes(), &self.cipher)?,
                    dkg_input_type: DkgInputType::SecretKey,
                    params_preset: threshold_preset,
                    committee_size: CiphernodesCommitteeSize::Small,
                    recipient_party_id: recipient_idx,
                    row_index: row_idx,
                    esi_index: 0,
                });
            }
        }

        // Build C3b proof requests (E_SM share encryption) from witnesses
        let mut e_sm_share_encryption_requests = Vec::new();
        for (esi_idx, esi_recipient_witnesses) in esi_witnesses.iter().enumerate() {
            for (recipient_idx, recipient_witnesses) in esi_recipient_witnesses.iter().enumerate() {
                for (row_idx, witness) in recipient_witnesses.iter().enumerate() {
                    e_sm_share_encryption_requests.push(ShareEncryptionProofRequest {
                        share_row_raw: SensitiveBytes::new(
                            bincode::serialize(&witness.share_row)
                                .map_err(|e| anyhow!("Failed to serialize share_row: {}", e))?,
                            &self.cipher,
                        )?,
                        ciphertext_raw: ArcBytes::from_bytes(&witness.ciphertext.to_bytes()),
                        recipient_pk_raw: ArcBytes::from_bytes(
                            &recipient_pks[recipient_idx].to_bytes(),
                        ),
                        u_rns_raw: SensitiveBytes::new(witness.u_rns.to_bytes(), &self.cipher)?,
                        e0_rns_raw: SensitiveBytes::new(witness.e0_rns.to_bytes(), &self.cipher)?,
                        e1_rns_raw: SensitiveBytes::new(witness.e1_rns.to_bytes(), &self.cipher)?,
                        dkg_input_type: DkgInputType::SmudgingNoise,
                        params_preset: threshold_preset,
                        committee_size: CiphernodesCommitteeSize::Small,
                        recipient_party_id: recipient_idx,
                        row_index: row_idx,
                        esi_index: esi_idx,
                    });
                }
            }
        }

        let total_proofs =
            3 + sk_share_encryption_requests.len() + e_sm_share_encryption_requests.len();
        info!(
            "Publishing ThresholdSharePending for E3 {} ({} proofs: C1, C2a, C2b + {} C3a + {} C3b)",
            e3_id, total_proofs,
            sk_share_encryption_requests.len(),
            e_sm_share_encryption_requests.len()
        );

        // Publish ThresholdSharePending - ProofRequestActor will generate proof, sign, and publish ThresholdShareCreated
        self.bus.publish(
            ThresholdSharePending {
                e3_id: e3_id.clone(),
                full_share: Arc::new(full_share),
                proof_request,
                sk_share_computation_request,
                e_sm_share_computation_request,
                sk_share_encryption_requests,
                e_sm_share_encryption_requests,
            },
            ec.clone(),
        )?;

        Ok(())
    }

    /// 5. AllThresholdSharesCollected - Verify C2/C3 proofs, then decrypt and aggregate
    pub fn handle_all_threshold_shares_collected(
        &mut self,
        msg: TypedEvent<AllThresholdSharesCollected>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("AllThresholdSharesCollected");
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let own_party_id = state.party_id;

        // Derive expected proof counts from our own share (trusted source).
        // All parties use the same BFV params, so moduli counts are identical.
        // Using the sender's share would let a malicious party manipulate expected counts.
        let own_share = msg
            .shares
            .iter()
            .find(|s| s.party_id == own_party_id)
            .ok_or_else(|| anyhow!("Own share not found in AllThresholdSharesCollected"))?;
        let expected_c3a = own_share
            .sk_sss
            .get_share(0)
            .map(|s| s.num_moduli())
            .unwrap_or(0);
        let expected_c3b: usize = own_share
            .esi_sss
            .iter()
            .map(|esi| esi.get_share(0).map(|s| s.num_moduli()).unwrap_or(0))
            .sum();
        let expected_num_esi = own_share.esi_sss.len();

        // Build verification requests for other parties' proofs
        let mut party_proofs_to_verify: Vec<PartyProofsToVerify> = Vec::new();
        let mut no_proof_parties: HashSet<u64> = HashSet::new();
        let mut incomplete_proof_parties: HashSet<u64> = HashSet::new();
        for (share, proofs) in msg.shares.iter().zip(msg.share_proofs.iter()) {
            if share.party_id == own_party_id {
                continue;
            }

            let has_any_proof = proofs.signed_c2a_proof.is_some()
                || proofs.signed_c2b_proof.is_some()
                || !proofs.signed_c3a_proofs.is_empty()
                || !proofs.signed_c3b_proofs.is_empty();

            if !has_any_proof {
                no_proof_parties.insert(share.party_id);
                continue;
            }

            // Validate proof set completeness against trusted expected counts.
            // A malicious sender could omit proofs that would fail verification,
            // so we must check that all expected proofs are present.
            let is_complete = proofs.signed_c2a_proof.is_some()
                && proofs.signed_c2b_proof.is_some()
                && proofs.signed_c3a_proofs.len() == expected_c3a
                && proofs.signed_c3b_proofs.len() == expected_c3b
                && share.esi_sss.len() == expected_num_esi;

            if !is_complete {
                warn!(
                    "Party {} has incomplete proof set (c2a={}, c2b={}, c3a={}/{}, c3b={}/{}, esi={}/{}), treating as dishonest",
                    share.party_id,
                    proofs.signed_c2a_proof.is_some(),
                    proofs.signed_c2b_proof.is_some(),
                    proofs.signed_c3a_proofs.len(), expected_c3a,
                    proofs.signed_c3b_proofs.len(), expected_c3b,
                    share.esi_sss.len(), expected_num_esi,
                );
                incomplete_proof_parties.insert(share.party_id);
                continue;
            }

            // Complete proof set — collect for verification
            let mut signed_proofs = Vec::new();
            // SAFETY: is_complete guarantees c2a and c2b are Some
            signed_proofs.push(proofs.signed_c2a_proof.clone().unwrap());
            signed_proofs.push(proofs.signed_c2b_proof.clone().unwrap());
            signed_proofs.extend(proofs.signed_c3a_proofs.iter().cloned());
            signed_proofs.extend(proofs.signed_c3b_proofs.iter().cloned());

            party_proofs_to_verify.push(PartyProofsToVerify {
                sender_party_id: share.party_id,
                signed_proofs,
            });
        }

        // Store shares on the actor for use after verification completes
        self.pending_shares = msg.shares.iter().map(|arc| (**arc).clone()).collect();

        // Backward compat: only when ALL non-self parties have zero proofs
        // AND none have incomplete proofs (incomplete proofs are always dishonest)
        if party_proofs_to_verify.is_empty() && incomplete_proof_parties.is_empty() {
            if no_proof_parties.is_empty() {
                info!(
                    "No C2/C3 proofs to verify for E3 {} — proceeding with all parties",
                    e3_id
                );
                return self.proceed_with_decryption_key_calculation(None, ec);
            }
            info!(
                "No C2/C3 proofs from any party for E3 {} — proceeding with all parties (backward compat)",
                e3_id
            );
            return self.proceed_with_decryption_key_calculation(None, ec);
        }

        // Merge no-proof and incomplete-proof parties — both are pre-dishonest
        let mut pre_dishonest: BTreeSet<u64> = BTreeSet::new();
        pre_dishonest.extend(incomplete_proof_parties);
        pre_dishonest.extend(no_proof_parties);
        if !pre_dishonest.is_empty() {
            warn!(
                "{} parties have missing/incomplete C2/C3 proofs for E3 {} — marking as pre-dishonest: {:?}",
                pre_dishonest.len(),
                e3_id,
                pre_dishonest
            );
        }

        if party_proofs_to_verify.is_empty() {
            // All non-self parties are dishonest (missing or incomplete proofs), none to verify
            let dishonest_set: HashSet<u64> = pre_dishonest.into_iter().collect();
            return self.proceed_with_decryption_key_calculation(Some(dishonest_set), ec);
        }

        info!(
            "Dispatching C2/C3 share verification for E3 {} ({} parties, {} pre-dishonest)",
            e3_id,
            party_proofs_to_verify.len(),
            pre_dishonest.len()
        );

        self.bus.publish(
            ShareVerificationDispatched {
                e3_id: e3_id.clone(),
                kind: VerificationKind::ShareProofs,
                share_proofs: party_proofs_to_verify,
                decryption_proofs: Vec::new(),
                pre_dishonest,
            },
            ec,
        )?;
        Ok(())
    }

    /// Handle ShareVerificationComplete from ShareVerificationActor.
    /// Dispatched for both C2/C3 and C4 verification.
    pub fn handle_share_verification_complete(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();

        match msg.kind {
            VerificationKind::ShareProofs => {
                // C2/C3 verification complete
                if msg.dishonest_parties.is_empty() {
                    info!(
                        "All parties passed C2/C3 verification for E3 {} — proceeding",
                        e3_id
                    );
                    self.proceed_with_decryption_key_calculation(None, ec)
                } else {
                    let threshold = state.threshold_m;
                    let total = state.threshold_n;
                    let honest_count = total - msg.dishonest_parties.len() as u64;

                    if honest_count < threshold {
                        warn!(
                            "Too few honest parties for E3 {} ({} honest < {} threshold) — cannot proceed",
                            e3_id, honest_count, threshold
                        );
                        // Clear pending shares
                        self.pending_shares.clear();
                        self.bus.publish(
                            E3Failed {
                                e3_id: e3_id.clone(),
                                failed_at_stage: E3Stage::CommitteeFinalized,
                                reason: FailureReason::InsufficientCommitteeMembers,
                            },
                            ec,
                        )?;
                        return Ok(());
                    }

                    let dishonest_set: HashSet<u64> = msg.dishonest_parties.into_iter().collect();
                    info!(
                        "Proceeding with {} honest parties for E3 {} ({} dishonest excluded)",
                        honest_count,
                        e3_id,
                        dishonest_set.len()
                    );
                    self.proceed_with_decryption_key_calculation(Some(dishonest_set), ec)
                }
            }
            VerificationKind::DecryptionProofs => {
                // C4 verification complete — update honest set and publish KeyshareCreated
                if !msg.dishonest_parties.is_empty() {
                    if let Some(ref mut honest) = self.honest_parties {
                        honest.retain(|pid| !msg.dishonest_parties.contains(pid));
                    }

                    let threshold = state.threshold_m;
                    let honest_count = self
                        .honest_parties
                        .as_ref()
                        .map(|h| h.len() as u64)
                        .unwrap_or(0);

                    if honest_count < threshold {
                        warn!(
                            "Too few honest parties after C4 for E3 {} ({} honest < {} threshold)",
                            e3_id, honest_count, threshold
                        );
                        self.bus.publish(
                            E3Failed {
                                e3_id: e3_id.clone(),
                                failed_at_stage: E3Stage::CommitteeFinalized,
                                reason: FailureReason::InsufficientCommitteeMembers,
                            },
                            ec,
                        )?;
                        return Ok(());
                    }

                    info!(
                        "Updated honest set after C4 for E3 {}: {} honest ({} removed)",
                        e3_id,
                        honest_count,
                        msg.dishonest_parties.len()
                    );
                } else {
                    info!(
                        "All parties passed C4 verification for E3 {} — publishing KeyshareCreated",
                        e3_id
                    );
                }

                self.publish_keyshare_created(ec)
            }
        }
    }

    /// After verification, decrypt shares from honest parties and compute decryption key.
    /// C4 proof generation is deferred to ProofRequestActor via DecryptionShareProofsPending.
    fn proceed_with_decryption_key_calculation(
        &mut self,
        dishonest_parties: Option<HashSet<u64>>,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let cipher = self.cipher.clone();
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let party_id = state.party_id as usize;
        let trbfv_config = state.get_trbfv_config();

        // Get our BFV secret key from state, pending shares from the actor
        let current: AggregatingDecryptionKey = state.clone().try_into()?;
        let shares = std::mem::take(&mut self.pending_shares);
        if shares.is_empty() {
            bail!("No pending verification shares");
        }
        let sk_bytes = current.sk_bfv.access(&cipher)?;
        let params = BfvParamSet::from(self.share_enc_preset.clone()).build_arc();
        let sk_bfv = deserialize_secret_key(&sk_bytes, &params)?;
        let degree = params.degree();

        // Filter to honest parties only
        let honest_shares: Vec<_> = shares
            .iter()
            .filter(|ts| {
                dishonest_parties
                    .as_ref()
                    .map_or(true, |dp| !dp.contains(&ts.party_id))
            })
            .collect();

        // Store honest party IDs in state
        let honest_party_ids: HashSet<u64> = honest_shares.iter().map(|s| s.party_id).collect();

        let num_honest = honest_shares.len();
        info!(
            "Decrypting shares from {} honest parties for E3 {}",
            num_honest, e3_id
        );

        // Collect ciphertext bytes for C4 proof requests (built here, sent after CalculateDecryptionKey)
        // C4a: sk_sss ciphertexts from honest parties [H * L]
        let mut sk_ciphertexts_raw = Vec::new();
        let mut num_moduli_sk: Option<usize> = None;
        for ts in &honest_shares {
            let idx = if ts.sk_sss.len() == 1 { 0 } else { party_id };
            let share = ts
                .sk_sss
                .clone_share(idx)
                .ok_or(anyhow!("No sk_sss share at index {}", idx))?;
            let moduli = share.num_moduli();
            match num_moduli_sk {
                Some(expected) if expected != moduli => {
                    bail!(
                        "Party {} has inconsistent sk num_moduli ({} vs expected {})",
                        ts.party_id,
                        moduli,
                        expected
                    );
                }
                None => num_moduli_sk = Some(moduli),
                _ => {}
            }
            for ct_bytes in share.ciphertext_bytes() {
                sk_ciphertexts_raw.push(ct_bytes.clone());
            }
        }
        let num_moduli_sk = num_moduli_sk.unwrap_or(0);

        // C4b: esi_sss ciphertexts from honest parties — one set per smudging noise
        let num_esi = honest_shares
            .first()
            .map(|ts| ts.esi_sss.len())
            .unwrap_or(0);
        for ts in &honest_shares {
            if ts.esi_sss.len() != num_esi {
                bail!(
                    "Party {} has inconsistent esi_sss count ({} vs expected {})",
                    ts.party_id,
                    ts.esi_sss.len(),
                    num_esi
                );
            }
        }
        let mut esi_ciphertexts_raw: Vec<Vec<ArcBytes>> = vec![Vec::new(); num_esi];
        let mut num_moduli_esi: Option<usize> = None;
        for ts in &honest_shares {
            for (esi_idx, esi_shares) in ts.esi_sss.iter().enumerate() {
                let idx = if esi_shares.len() == 1 { 0 } else { party_id };
                let share = esi_shares
                    .clone_share(idx)
                    .ok_or(anyhow!("No esi_sss share at index {}", idx))?;
                let moduli = share.num_moduli();
                match num_moduli_esi {
                    Some(expected) if expected != moduli => {
                        bail!(
                            "Party {} has inconsistent esi num_moduli at esi_idx {} ({} vs expected {})",
                            ts.party_id, esi_idx, moduli, expected
                        );
                    }
                    None => num_moduli_esi = Some(moduli),
                    _ => {}
                }
                for ct_bytes in share.ciphertext_bytes() {
                    esi_ciphertexts_raw[esi_idx].push(ct_bytes.clone());
                }
            }
        }
        let num_moduli_esi = num_moduli_esi.unwrap_or(0);

        // Decrypt our share from each honest sender using BFV
        let sk_sss_collected: Vec<ShamirShare> = honest_shares
            .iter()
            .map(|ts| {
                let idx = if ts.sk_sss.len() == 1 { 0 } else { party_id };
                let encrypted = ts
                    .sk_sss
                    .clone_share(idx)
                    .ok_or(anyhow!("No sk_sss share at index {}", idx))?;
                encrypted.decrypt(&sk_bfv, &params, degree)
            })
            .collect::<Result<_>>()?;

        let esi_sss_collected: Vec<Vec<ShamirShare>> = honest_shares
            .iter()
            .map(|ts| {
                ts.esi_sss
                    .iter()
                    .map(|esi_shares| {
                        let idx = if esi_shares.len() == 1 { 0 } else { party_id };
                        let encrypted = esi_shares
                            .clone_share(idx)
                            .ok_or(anyhow!("No esi_sss share at index {}", idx))?;
                        encrypted.decrypt(&sk_bfv, &params, degree)
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<_>>()?;

        // Publish CalculateDecryptionKey request
        let request = CalculateDecryptionKeyRequest {
            trbfv_config,
            esi_sss_collected: esi_sss_collected
                .into_iter()
                .map(|s| s.encrypt(&cipher))
                .collect::<Result<_>>()?,
            sk_sss_collected: sk_sss_collected.encrypt(&cipher)?,
        };

        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateDecryptionKey(request),
            CorrelationId::new(),
            e3_id.clone(),
        );
        self.bus.publish(event, ec.clone())?;

        // Build C4 proof requests — stored for sending after CalculateDecryptionKey completes
        let threshold_preset = self
            .share_enc_preset
            .threshold_counterpart()
            .ok_or_else(|| anyhow!("No threshold counterpart for {:?}", self.share_enc_preset))?;

        let sk_request = DkgShareDecryptionProofRequest {
            sk_bfv: current.sk_bfv.clone(),
            honest_ciphertexts_raw: sk_ciphertexts_raw,
            num_honest_parties: num_honest,
            num_moduli: num_moduli_sk,
            dkg_input_type: DkgInputType::SecretKey,
            params_preset: threshold_preset,
        };

        let esm_requests: Vec<DkgShareDecryptionProofRequest> = esi_ciphertexts_raw
            .into_iter()
            .map(|esi_cts| DkgShareDecryptionProofRequest {
                sk_bfv: current.sk_bfv.clone(),
                honest_ciphertexts_raw: esi_cts,
                num_honest_parties: num_honest,
                num_moduli: num_moduli_esi,
                dkg_input_type: DkgInputType::SmudgingNoise,
                params_preset: threshold_preset,
            })
            .collect();

        // Store honest parties and C4 data on the actor (transient coordination)
        self.honest_parties = Some(honest_party_ids);
        self.pending_c4_proof_data = Some((sk_request, esm_requests));

        Ok(())
    }

    /// 5a. CalculateDecryptionKeyResponse — transition to ReadyForDecryption,
    /// then publish DecryptionShareProofsPending so ProofRequestActor can
    /// generate C4 proofs, sign them, and publish DecryptionKeyShared.
    pub fn handle_calculate_decryption_key_response(
        &mut self,
        res: TypedEvent<ComputeResponse>,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        let (res, ec) = res.into_components();
        let output: CalculateDecryptionKeyResponse = res
            .try_into()
            .context("Error extracting data from compute process")?;

        let (sk_poly_sum, es_poly_sum) = (output.sk_poly_sum, output.es_poly_sum);

        // Extract C4 data from the actor (stored by proceed_with_decryption_key_calculation)
        let (sk_request, esm_requests) = self
            .pending_c4_proof_data
            .take()
            .ok_or_else(|| anyhow!("No pending C4 proof data — CalculateDecryptionKey responded before proof requests were built"))?;

        // Take early shares from the actor before transitioning
        let early_shares = std::mem::take(&mut self.early_decryption_key_shares);

        // Transition to ReadyForDecryption
        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;
            info!("Try store decryption key");

            let current: AggregatingDecryptionKey = s.clone().try_into()?;

            let next = K::ReadyForDecryption(ReadyForDecryption {
                pk_share: current.pk_share,
                sk_poly_sum,
                es_poly_sum,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
                signed_sk_share_computation_proof: current.signed_sk_share_computation_proof,
                signed_e_sm_share_computation_proof: current.signed_e_sm_share_computation_proof,
                signed_sk_share_encryption_proofs: current.signed_sk_share_encryption_proofs,
                signed_e_sm_share_encryption_proofs: current.signed_e_sm_share_encryption_proofs,
            });

            s.new_state(next)
        })?;

        // Publish DecryptionShareProofsPending to ProofRequestActor
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let party_id = state.party_id;
        let node = state.address.clone();

        let ready: ReadyForDecryption = state.clone().try_into()?;
        let sk_poly_sum_bytes = ready.sk_poly_sum.access(&self.cipher)?;
        let es_poly_sum_bytes: Vec<ArcBytes> = ready
            .es_poly_sum
            .iter()
            .map(|s| {
                let bytes = s.access(&self.cipher)?;
                Ok(ArcBytes::from_bytes(&bytes))
            })
            .collect::<Result<_>>()?;

        info!(
            "Publishing DecryptionShareProofsPending for E3 {} party {} (1 SK + {} ESM requests)",
            e3_id,
            party_id,
            esm_requests.len()
        );

        self.bus.publish(
            DecryptionShareProofsPending {
                e3_id: e3_id.clone(),
                party_id,
                node,
                sk_poly_sum: ArcBytes::from_bytes(&sk_poly_sum_bytes),
                es_poly_sum: es_poly_sum_bytes,
                sk_request,
                esm_requests,
            },
            ec.clone(),
        )?;

        // Create collector and replay any early-arriving DecryptionKeyShared events
        let state = self.state.try_get()?;
        let my_party_id = state.party_id;
        let honest = self.honest_parties.as_ref().cloned().unwrap_or_default();
        let expected: HashSet<u64> = honest
            .iter()
            .filter(|&&pid| pid != my_party_id)
            .copied()
            .collect();

        if !expected.is_empty() {
            let collector = self.ensure_decryption_key_shared_collector(self_addr)?;
            for (_pid, share) in early_shares {
                collector.do_send(TypedEvent::new(share, ec.clone()));
            }
        }

        Ok(())
    }

    /// Handle an external DecryptionKeyShared event while in AggregatingDecryptionKey state.
    /// Store it for later processing when we transition to ReadyForDecryption.
    fn handle_early_decryption_key_share(
        &mut self,
        data: DecryptionKeyShared,
        _ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let party_id = data.party_id;
        info!(
            "Storing early DecryptionKeyShared from party {} (state: AggregatingDecryptionKey)",
            party_id
        );
        self.early_decryption_key_shares.insert(party_id, data);
        Ok(())
    }

    /// Dispatch C4 verification for all collected DecryptionKeyShared events.
    /// Shares are provided by the DecryptionKeySharedCollector.
    fn dispatch_c4_verification(
        &mut self,
        collected_shares: HashMap<u64, DecryptionKeyShared>,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let ready: ReadyForDecryption = state.clone().try_into()?;

        info!(
            "All DecryptionKeyShared collected for E3 {} ({} shares)",
            e3_id,
            collected_shares.len()
        );

        // Validate ESM proof count — each party must provide exactly
        // one C4b proof per smudging noise index.
        let expected_esm = ready.es_poly_sum.len();
        let mut c4_count_dishonest: HashSet<u64> = HashSet::new();
        let party_proofs: Vec<PartyShareDecryptionProofsToVerify> = collected_shares
            .iter()
            .filter_map(|(&party_id, share)| {
                if share.signed_esm_decryption_proofs.len() != expected_esm {
                    warn!(
                        "Party {} has wrong ESM proof count ({} vs expected {}) for E3 {} — treating as dishonest",
                        party_id,
                        share.signed_esm_decryption_proofs.len(),
                        expected_esm,
                        e3_id
                    );
                    c4_count_dishonest.insert(party_id);
                    None
                } else {
                    Some(PartyShareDecryptionProofsToVerify {
                        sender_party_id: party_id,
                        signed_sk_decryption_proof: share.signed_sk_decryption_proof.clone(),
                        signed_esm_decryption_proofs: share.signed_esm_decryption_proofs.clone(),
                    })
                }
            })
            .collect();

        // Evict pre-dishonest parties (wrong ESM count) from honest set
        if !c4_count_dishonest.is_empty() {
            if let Some(ref mut honest) = self.honest_parties {
                honest.retain(|pid| !c4_count_dishonest.contains(pid));
            }
        }

        if party_proofs.is_empty() {
            info!("No C4 proofs to verify — publishing KeyshareCreated directly");
            return self.publish_keyshare_created(ec);
        }

        let pre_dishonest: BTreeSet<u64> = c4_count_dishonest.into_iter().collect();

        info!(
            "Dispatching C4 share verification for E3 {} ({} parties, {} pre-dishonest)",
            e3_id,
            party_proofs.len(),
            pre_dishonest.len()
        );

        self.bus.publish(
            ShareVerificationDispatched {
                e3_id: e3_id.clone(),
                kind: VerificationKind::DecryptionProofs,
                share_proofs: Vec::new(),
                decryption_proofs: party_proofs,
                pre_dishonest,
            },
            ec,
        )?;
        Ok(())
    }

    /// Publish KeyshareCreated (Exchange #4) with pk_share and signed C1 proof.
    fn publish_keyshare_created(&mut self, ec: EventContext<Sequenced>) -> Result<()> {
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let address = state.get_address().to_owned();
        let current: ReadyForDecryption = state.clone().try_into()?;

        info!("Publishing Exchange #4 (KeyshareCreated) for E3 {}", e3_id);

        self.bus.publish(
            KeyshareCreated {
                pubkey: current.pk_share,
                e3_id: e3_id.clone(),
                node: address,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
            },
            ec,
        )?;

        Ok(())
    }

    /// CiphertextOutputPublished
    pub fn handle_ciphertext_output_published(
        &mut self,
        msg: TypedEvent<CiphertextOutputPublished>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        // Set state to decrypting
        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;

            let current: ReadyForDecryption = s.clone().try_into()?;

            let next = K::Decrypting(Decrypting {
                pk_share: current.pk_share,
                sk_poly_sum: current.sk_poly_sum,
                es_poly_sum: current.es_poly_sum,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
                signed_sk_share_computation_proof: current.signed_sk_share_computation_proof,
                signed_e_sm_share_computation_proof: current.signed_e_sm_share_computation_proof,
                signed_sk_share_encryption_proofs: current.signed_sk_share_encryption_proofs,
                signed_e_sm_share_encryption_proofs: current.signed_e_sm_share_encryption_proofs,
            });

            s.new_state(next)
        })?;

        let ciphertext_output = msg.ciphertext_output;
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let decrypting: Decrypting = state.clone().try_into()?;
        let trbfv_config = state.get_trbfv_config();
        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateDecryptionShare(
                CalculateDecryptionShareRequest {
                    name: format!("party_id({})", state.party_id),
                    ciphertexts: ciphertext_output,
                    sk_poly_sum: decrypting.sk_poly_sum,
                    es_poly_sum: decrypting.es_poly_sum,
                    trbfv_config,
                }
                .into(),
            ),
            CorrelationId::new(),
            e3_id.clone(),
        );
        self.bus.publish(event, ec)?; // CalculateDecryptionShareRequest
        Ok(())
    }

    /// CalculateDecryptionShareResponse
    pub fn handle_calculate_decryption_share_response(
        &mut self,
        res: TypedEvent<ComputeResponse>,
    ) -> Result<()> {
        let (res, ec) = res.into_components();
        let msg: CalculateDecryptionShareResponse = res.try_into()?;
        let state = self.state.try_get()?;
        let party_id = state.party_id;
        let node = state.address;
        let e3_id = state.e3_id;
        let decryption_share = msg.d_share_poly;

        let event = DecryptionshareCreated {
            party_id,
            node,
            e3_id,
            decryption_share,
        };

        // send the decryption share
        self.bus.publish(event, ec.clone())?;

        // mark as complete
        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;
            info!("Decryption share sending process is complete");

            s.new_state(K::Completed)
        })?;

        Ok(())
    }
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::CiphernodeSelected(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CiphertextOutputPublished(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ThresholdShareCreated(data) => {
                let _ =
                    self.handle_threshold_share_created(TypedEvent::new(data, ec), ctx.address());
            }
            EnclaveEventData::EncryptionKeyCreated(data) => {
                let _ =
                    self.handle_encryption_key_created(TypedEvent::new(data, ec), ctx.address());
            }
            EnclaveEventData::PkGenerationProofSigned(data) => {
                let _ = self.handle_pk_generation_proof_signed(TypedEvent::new(data, ec));
            }
            EnclaveEventData::DkgProofSigned(data) => {
                let _ = self.handle_share_computation_proof_signed(TypedEvent::new(data, ec));
            }
            EnclaveEventData::E3RequestComplete(data) => self.notify_sync(ctx, data),
            EnclaveEventData::E3Failed(data) => {
                warn!(
                    "E3 failed: {:?}. Shutting down ThresholdKeyshare for e3_id={}",
                    data.reason, data.e3_id
                );
                self.notify_sync(ctx, E3RequestComplete { e3_id: data.e3_id });
            }
            EnclaveEventData::E3StageChanged(data) => {
                use e3_events::E3Stage;
                match &data.new_stage {
                    E3Stage::Complete | E3Stage::Failed => {
                        info!("E3 reached terminal stage {:?}. Shutting down ThresholdKeyshare for e3_id={}", data.new_stage, data.e3_id);
                        self.notify_sync(ctx, E3RequestComplete { e3_id: data.e3_id });
                    }
                    _ => {
                        trace!(
                            "E3 stage changed to {:?} for e3_id={}",
                            data.new_stage,
                            data.e3_id
                        );
                    }
                }
            }
            EnclaveEventData::DecryptionKeyShared(data) => {
                if data.external {
                    // Route based on current state
                    if let Some(state) = self.state.get() {
                        let result = match &state.state {
                            KeyshareState::AggregatingDecryptionKey(_) => {
                                self.handle_early_decryption_key_share(data, ec)
                            }
                            KeyshareState::ReadyForDecryption(_) => {
                                // Delegate to the collector actor
                                if let Some(ref collector) = self.decryption_key_shared_collector {
                                    collector.do_send(TypedEvent::new(data, ec));
                                    Ok(())
                                } else {
                                    warn!(
                                        "DecryptionKeyShared from party {} in ReadyForDecryption but no collector",
                                        data.party_id
                                    );
                                    Ok(())
                                }
                            }
                            other => {
                                trace!(
                                    "DecryptionKeyShared from party {} in unexpected state {:?}, ignoring",
                                    data.party_id,
                                    other.variant_name()
                                );
                                Ok(())
                            }
                        };
                        if let Err(err) = result {
                            error!("Failed to handle DecryptionKeyShared: {err}");
                        }
                    }
                } else {
                    // Own DecryptionKeyShared published by ProofRequestActor.
                    // A3 fast-path: if no other honest parties, publish KeyshareCreated directly.
                    if let Some(state) = self.state.get() {
                        if data.party_id == state.party_id {
                            if let KeyshareState::ReadyForDecryption(_) = state.state {
                                let others = self
                                    .honest_parties
                                    .as_ref()
                                    .map(|h| h.iter().filter(|&&pid| pid != state.party_id).count())
                                    .unwrap_or(0);
                                if others == 0 {
                                    info!(
                                        "No other honest parties for E3 {} — publishing KeyshareCreated directly",
                                        data.e3_id
                                    );
                                    if let Err(err) = self.publish_keyshare_created(ec) {
                                        error!("Failed to publish KeyshareCreated: {err}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            EnclaveEventData::ShareVerificationComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_compute_response(msg, ctx.address()),
        )
    }
}

impl Handler<TypedEvent<CiphernodeSelected>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<CiphernodeSelected>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_ciphernode_selected(msg, ctx.address()),
        )
    }
}

impl Handler<TypedEvent<AllEncryptionKeysCollected>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<AllEncryptionKeysCollected>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_all_encryption_keys_collected(msg),
        )
    }
}

impl Handler<TypedEvent<ShareVerificationComplete>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_share_verification_complete(msg),
        )
    }
}

impl Handler<TypedEvent<AllThresholdSharesCollected>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<AllThresholdSharesCollected>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_all_threshold_shares_collected(msg),
        )
    }
}

impl Handler<TypedEvent<AllDecryptionKeySharesCollected>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<AllDecryptionKeySharesCollected>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_all_decryption_key_shares_collected(msg),
        )
    }
}

impl Handler<DecryptionKeySharedCollectionFailed> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: DecryptionKeySharedCollectionFailed,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            warn!(
                e3_id = %msg.e3_id,
                missing_parties = ?msg.missing_parties,
                "DecryptionKeyShared collection failed: {}",
                msg.reason
            );

            // Clear the collector reference since it's stopped
            self.decryption_key_shared_collector = None;

            if let Err(err) = self.bus.publish_without_context(E3Failed {
                e3_id: msg.e3_id.clone(),
                failed_at_stage: E3Stage::CommitteeFinalized,
                reason: FailureReason::InsufficientCommitteeMembers,
            }) {
                error!("Failed to publish E3Failed: {err}");
            }
            Ok(())
        })
    }
}

impl Handler<TypedEvent<CiphertextOutputPublished>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<CiphertextOutputPublished>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_ciphertext_output_published(msg),
        )
    }
}

impl Handler<EncryptionKeyCollectionFailed> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: EncryptionKeyCollectionFailed,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            warn!(
                e3_id = %msg.e3_id,
                missing_parties = ?msg.missing_parties,
                "Encryption key collection failed: {}",
                msg.reason
            );

            // Clear the collector reference since it's stopped
            self.encryption_key_collector = None;

            // Publish failure event to event bus for sync tracking
            self.bus.publish_without_context(msg)?;

            // Stop this actor since we can't proceed without all encryption keys
            ctx.stop();
            Ok(())
        })
    }
}

impl Handler<ThresholdShareCollectionFailed> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: ThresholdShareCollectionFailed,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            warn!(
                e3_id = %msg.e3_id,
                missing_parties = ?msg.missing_parties,
                "Threshold share collection failed: {}",
                msg.reason
            );

            // Clear the collector reference since it's stopped
            self.decryption_key_collector = None;

            // Publish failure event to event bus for sync tracking
            self.bus.publish_without_context(msg)?;

            ctx.stop();
            Ok(())
        })
    }
}

impl Handler<TypedEvent<AllDecryptionKeySharesCollected>> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<AllDecryptionKeySharesCollected>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || {
                let (msg, ec) = msg.into_components();
                self.decryption_key_shared_collector = None;
                self.dispatch_c4_verification(msg.shares, ec)
            },
        )
    }
}

impl Handler<DecryptionKeySharedCollectionFailed> for ThresholdKeyshare {
    type Result = ();
    fn handle(
        &mut self,
        msg: DecryptionKeySharedCollectionFailed,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(EType::KeyGeneration, &self.bus.clone(), || {
            warn!(
                e3_id = %msg.e3_id,
                missing_parties = ?msg.missing_parties,
                "DecryptionKeyShared collection failed: {}",
                msg.reason
            );

            self.decryption_key_shared_collector = None;

            self.bus.publish_without_context(E3Failed {
                e3_id: msg.e3_id.clone(),
                failed_at_stage: E3Stage::CommitteeFinalized,
                reason: FailureReason::InsufficientCommitteeMembers,
            })?;

            ctx.stop();
            Ok(())
        })
    }
}

impl Handler<E3RequestComplete> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, _: E3RequestComplete, ctx: &mut Self::Context) -> Self::Result {
        self.encryption_key_collector = None;
        self.decryption_key_collector = None;
        self.decryption_key_shared_collector = None;
        self.pending_shares.clear();
        self.pending_c4_proof_data = None;
        self.honest_parties = None;
        self.early_decryption_key_shares.clear();
        self.notify_sync(ctx, Die);
    }
}

impl Handler<Die> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        warn!("ThresholdKeyshare is shutting down");
        ctx.stop();
    }
}
