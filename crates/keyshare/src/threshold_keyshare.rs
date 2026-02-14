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
    ComputeResponse, ComputeResponseKind, CorrelationId, DecryptionshareCreated, Die,
    E3RequestComplete, E3id, EType, EnclaveEvent, EnclaveEventData, EncryptionKey,
    EncryptionKeyCollectionFailed, EncryptionKeyCreated, EncryptionKeyPending, EventContext,
    KeyshareCreated, PartyId, PkGenerationProofRequest, PkGenerationProofSigned, Sequenced,
    SignedProofPayload, ThresholdShare, ThresholdShareCollectionFailed, ThresholdShareCreated,
    ThresholdSharePending, TypedEvent,
};
use e3_fhe::create_crp;
use e3_fhe_params::{BfvParamSet, BfvPreset};
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
use e3_utils::{to_ordered_vec, utility_types::ArcBytes};
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use e3_zk_helpers::CiphernodesCommitteeSize;
use fhe::bfv::{PublicKey, SecretKey};
use fhe_traits::{DeserializeParametrized, Serialize};
use rand::{rngs::OsRng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::{
    collections::HashMap, mem, sync::{Arc, Mutex}
};
use tracing::{info, trace, warn};

use crate::encryption_key_collector::{AllEncryptionKeysCollected, EncryptionKeyCollector};
use crate::threshold_share_collector::ThresholdShareCollector;

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
}

impl From<HashMap<u64, Arc<ThresholdShare>>> for AllThresholdSharesCollected {
    fn from(value: HashMap<u64, Arc<ThresholdShare>>) -> Self {
        AllThresholdSharesCollected {
            shares: to_ordered_vec(value),
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
    pub a_raw: ArcBytes,
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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReadyForDecryption {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
    signed_pk_generation_proof: Option<SignedProofPayload>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decrypting {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
    signed_pk_generation_proof: Option<SignedProofPayload>,
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
    state: Persistable<ThresholdKeyshareState>,
    share_enc_preset: BfvPreset,
}

impl ThresholdKeyshare {
    pub fn new(params: ThresholdKeyshareParams) -> Self {
        Self {
            bus: params.bus,
            cipher: params.cipher,
            decryption_key_collector: None,
            encryption_key_collector: None,
            state: params.state,
            share_enc_preset: params.share_enc_preset,
        }
    }
}

impl Actor for ThresholdKeyshare {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
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

    /// Handle PkGenerationProofSigned - stores the signed T1 proof in state
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

    pub fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) -> Result<()> {
        match &msg.response {
            ComputeResponseKind::TrBFV(trbfv) => match trbfv {
                TrBFVResponse::GenEsiSss(_) => self.handle_gen_esi_sss_response(msg),
                TrBFVResponse::GenPkShareAndSkSss(_) => {
                    self.handle_gen_pk_share_and_sk_sss_response(msg)
                }
                TrBFVResponse::CalculateDecryptionKey(_) => {
                    self.handle_calculate_decryption_key_response(msg)
                }
                TrBFVResponse::CalculateDecryptionShare(_) => {
                    self.handle_calculate_decryption_share_response(msg)
                }
                _ => Ok(()),
            },
            ComputeResponseKind::Zk(zk) => match zk {
                _ => Ok(()),
            },
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
        let CiphernodeSelected { seed, e3_id, .. } = msg.0;
        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config: TrBFVConfig = state.get_trbfv_config();

        let crp = ArcBytes::from_bytes(
            &create_crp(
                trbfv_config.params(),
                Arc::new(Mutex::new(ChaCha20Rng::from_seed(seed.into()))),
            )
            .to_bytes(),
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
            a_raw: output.a_raw,
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

        let evt = msg.ciphernode_selected;
        let e_sm_raw_decrypted = ArcBytes::from_bytes(&msg.e_sm_raw.access_raw(&self.cipher)?);
        let CiphernodeSelected { e3_id, .. } = evt.clone();

        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config = state.get_trbfv_config();

        let event = ComputeRequest::trbfv(
            TrBFVRequest::GenEsiSss(
                GenEsiSssRequest {
                    trbfv_config,
                    e_sm_raw: e_sm_raw_decrypted,
                }
                .into(),
            ),
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
                    },
                ))
            })?;
        }

        Ok(())
    }

     /// 3. GenEsiSss
     pub fn handle_gen_esi_sss_requested(&self, msg: TypedEvent<GenEsiSss>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("GenEsiSss on ThresholdKeyshare");

        let evt = msg.ciphernode_selected;
        let e_sm_raw = msg.e_sm_raw;
        let CiphernodeSelected {
            e3_id,
            ..
        } = evt.clone();

        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config = state.get_trbfv_config();

        let event = ComputeRequest::trbfv(
            TrBFVRequest::GenEsiSss(
                GenEsiSssRequest {
                    trbfv_config,
                    e_sm_raw,
                }
                .into(),
            ),
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

        // Convert to BFV public keys
        let params = BfvParamSet::from(self.share_enc_preset.clone()).build_arc();
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

        // Encrypt shares for all recipients using BFV
        let mut rng = OsRng;
        let encrypted_sk_sss =
            BfvEncryptedShares::encrypt_all(&decrypted_sk_sss, &recipient_pks, &params, &mut rng)?;

        let encrypted_esi_sss: Vec<BfvEncryptedShares> = decrypted_esi_sss
            .iter()
            .map(|esi| BfvEncryptedShares::encrypt_all(esi, &recipient_pks, &params, &mut rng))
            .collect::<Result<_>>()?;

        // Create the full share with all parties' encrypted data
        let full_share = ThresholdShare {
            party_id,
            pk_share,
            sk_sss: encrypted_sk_sss,
            esi_sss: encrypted_esi_sss,
        };

        // Build the proof request
        let threshold_preset = self
            .share_enc_preset
            .threshold_counterpart()
            .ok_or_else(|| anyhow!("No threshold counterpart for {:?}", self.share_enc_preset))?;

        let proof_request = PkGenerationProofRequest::new(
            proof_request_data.pk0_share_raw.clone(),
            proof_request_data.a_raw.clone(),
            proof_request_data.sk_raw.clone(),
            proof_request_data.eek_raw.clone(),
            e_sm_raw.clone(),
            threshold_preset,
            CiphernodesCommitteeSize::Small, // TODO: derive from config
        );

        info!(
            "Publishing ThresholdSharePending for E3 {} (proof will be generated and signed by ProofRequestActor)",
            e3_id
        );

        // Publish ThresholdSharePending - ProofRequestActor will generate proof, sign, and publish ThresholdShareCreated
        self.bus.publish(
            ThresholdSharePending {
                e3_id: e3_id.clone(),
                full_share: Arc::new(full_share),
                proof_request,
            },
            ec.clone(),
        )?;

        Ok(())
    }

    /// 5. AllThresholdSharesCollected. This is fired after the ThresholdShareCreated events are
    ///    aggregateed in the decryption_key_collector::ThresholdShareCollector
    /// 5. AllThresholdSharesCollected - Decrypt received shares using BFV and aggregate
    pub fn handle_all_threshold_shares_collected(
        &self,
        msg: TypedEvent<AllThresholdSharesCollected>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("AllThresholdSharesCollected");
        let cipher = self.cipher.clone();
        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let party_id = state.party_id as usize;
        let trbfv_config = state.get_trbfv_config();

        // Get our BFV secret key from state
        let current: AggregatingDecryptionKey = state.clone().try_into()?;
        let sk_bytes = current.sk_bfv.access(&cipher)?;
        let params = BfvParamSet::from(self.share_enc_preset.clone()).build_arc();
        let sk_bfv = deserialize_secret_key(&sk_bytes, &params)?;
        let degree = params.degree();

        // Decrypt our share from each sender using BFV
        // Local share (from self) has all parties' shares, network shares are pre-extracted
        let sk_sss_collected: Vec<ShamirShare> = msg
            .shares
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

        // Similarly decrypt esi_sss for each ciphertext
        let esi_sss_collected: Vec<Vec<ShamirShare>> = msg
            .shares
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

        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// 5a. CalculateDecryptionKeyResponse -> KeyshareCreated
    pub fn handle_calculate_decryption_key_response(
        &mut self,
        res: TypedEvent<ComputeResponse>,
    ) -> Result<()> {
        let (res, ec) = res.into_components();
        let output: CalculateDecryptionKeyResponse = res
            .try_into()
            .context("Error extracting data from compute process")?;

        let (sk_poly_sum, es_poly_sum) = (output.sk_poly_sum, output.es_poly_sum);

        self.state.try_mutate(&ec, |s| {
            use KeyshareState as K;
            info!("Try store decryption key");

            // Get pk_share and signed proof from current state
            let current: AggregatingDecryptionKey = s.clone().try_into()?;

            // Transition to ReadyForDecryption, carrying the signed proof
            let next = K::ReadyForDecryption(ReadyForDecryption {
                pk_share: current.pk_share,
                sk_poly_sum,
                es_poly_sum,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
            });

            s.new_state(next)
        })?;

        let state = self.state.try_get()?;
        let e3_id = state.get_e3_id();
        let address = state.get_address().to_owned();
        let current: ReadyForDecryption = state.clone().try_into()?;

        self.bus.publish(
            KeyshareCreated {
                pubkey: current.pk_share,
                e3_id: e3_id.clone(),
                node: address,
                signed_pk_generation_proof: current.signed_pk_generation_proof,
            },
            ec.clone(),
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
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<ComputeResponse>, _: &mut Self::Context) -> Self::Result {
        trap(
            EType::KeyGeneration,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_compute_response(msg),
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

impl Handler<E3RequestComplete> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, _: E3RequestComplete, ctx: &mut Self::Context) -> Self::Result {
        self.encryption_key_collector = None;
        self.decryption_key_collector = None;
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
