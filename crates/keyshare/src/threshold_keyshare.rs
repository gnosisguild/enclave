// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    collections::{HashMap, HashSet},
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

use actix::prelude::*;
use anyhow::{anyhow, bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_data::Persistable;
use e3_events::{
    CiphernodeSelected, CiphertextOutputPublished, ComputeRequest, ComputeResponse,
    DecryptionshareCreated, E3id, EnclaveEvent, EventBus, KeyshareCreated, PartyId, ThresholdShare,
    ThresholdShareCreated,
};
use e3_fhe::create_crp;
use e3_multithread::Multithread;
use e3_trbfv::{
    calculate_decryption_key::CalculateDecryptionKeyRequest,
    calculate_decryption_share::{
        CalculateDecryptionShareRequest, CalculateDecryptionShareResponse,
    },
    gen_esi_sss::{GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::GenPkShareAndSkSssRequest,
    shares::{EncryptableVec, Encrypted, PvwEncrypted, ShamirShare, SharedSecret},
    SharedRng, TrBFVConfig, TrBFVRequest, TrBFVResponse,
};
use e3_utils::{to_ordered_vec, utility_types::ArcBytes};
use fhe_traits::Serialize;
use tracing::{error, info};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct StartThresholdShareGeneration(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
struct GenPkShareAndSkSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
struct GenEsiSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct SharesGenerated;

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

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneratingThresholdShareData {
    pk_share: Option<ArcBytes>,
    sk_sss: Option<Encrypted<SharedSecret>>,
    esi_sss: Option<Vec<Encrypted<SharedSecret>>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AggregatingDecryptionKey {
    pk_share: ArcBytes,
    sk_sss: Encrypted<SharedSecret>,
    esi_sss: Vec<Encrypted<SharedSecret>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReadyForDecryption {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Decrypting {
    pk_share: ArcBytes,
    sk_poly_sum: SensitiveBytes,
    es_poly_sum: Vec<SensitiveBytes>,
}

// TODO: Add GeneratingPvwKey state
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum KeyshareState {
    // Before anything
    Init,
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
                    (K::Init, K::GeneratingThresholdShare(_)) => true,
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
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub cipher: Arc<Cipher>,
    pub multithread: Addr<Multithread>,
    pub rng: SharedRng,
    pub state: Persistable<ThresholdKeyshareState>,
}

pub struct ThresholdKeyshare {
    bus: Addr<EventBus<EnclaveEvent>>,
    cipher: Arc<Cipher>,
    decryption_key_collector: Option<Addr<DecryptionKeyCollector>>,
    multithread: Addr<Multithread>,
    rng: SharedRng,
    state: Persistable<ThresholdKeyshareState>,
}

impl ThresholdKeyshare {
    pub fn new(params: ThresholdKeyshareParams) -> Self {
        Self {
            bus: params.bus,
            cipher: params.cipher,
            decryption_key_collector: None,
            multithread: params.multithread,
            rng: params.rng,
            state: params.state,
        }
    }
}

impl Actor for ThresholdKeyshare {
    type Context = actix::Context<Self>;
}

impl ThresholdKeyshare {
    pub fn ensure_collector(
        &mut self,
        self_addr: Addr<Self>,
    ) -> Result<Addr<DecryptionKeyCollector>> {
        let Some(state) = self.state.get() else {
            bail!("State not found on threshold keyshare. This should not happen.");
        };

        info!(
            "Setting up key collector for addr: {} and {} nodes",
            state.address, state.threshold_n
        );
        let addr = self
            .decryption_key_collector
            .get_or_insert_with(|| DecryptionKeyCollector::setup(self_addr, state.threshold_n));
        Ok(addr.clone())
    }

    /// Extract collected shares from collector
    pub fn try_generate_compute_decryption_key_request(
        &self,
        msg: AllThresholdSharesCollected,
    ) -> Result<CalculateDecryptionKeyRequest> {
        let cipher = self.cipher.clone();
        let Some(state) = self.state.get() else {
            bail!("No state found");
        };

        let party_id = state.party_id as usize;
        let trbfv_config = state.get_trbfv_config();

        // Shares are in order of party_id
        let received_sss: Vec<SharedSecret> = msg
            .shares
            .clone()
            .into_iter()
            .map(|ts| ts.sk_sss.clone().pvw_decrypt())
            .collect::<Result<_>>()?;

        let received_esi_sss: Vec<Vec<SharedSecret>> = msg
            .shares
            .into_iter()
            .map(|ts| {
                ts.esi_sss
                    .clone()
                    .into_iter()
                    .map(|s| s.pvw_decrypt())
                    .collect()
            })
            .collect::<Result<_>>()?;

        let sk_sss_collected: Vec<ShamirShare> = received_sss
            .into_iter()
            .map(|s| s.extract_party_share(party_id))
            .collect::<Result<_>>()?;

        let esi_sss_collected: Vec<Vec<ShamirShare>> = received_esi_sss
            .into_iter()
            .map(|esi| {
                esi.into_iter()
                    .map(|s| s.extract_party_share(party_id))
                    .collect()
            })
            .collect::<Result<_>>()?;

        Ok(CalculateDecryptionKeyRequest {
            trbfv_config,
            esi_sss_collected: esi_sss_collected
                .into_iter()
                .map(|s| s.encrypt(&cipher))
                .collect::<Result<_>>()?,
            sk_sss_collected: sk_sss_collected.encrypt(&cipher)?,
        })
    }

    pub fn send_to_decryption_key_collector(
        &mut self,
        msg: ThresholdShareCreated,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        info!("Received ThresholdShareCreated forwarding to collector!");
        let collector = self.ensure_collector(self_addr)?;
        info!("got collector address!");
        collector.do_send(msg);
        Ok(())
    }

    pub fn try_store_pk_share_and_sk_sss(
        &mut self,
        pk_share: ArcBytes,
        sk_sss: Encrypted<SharedSecret>,
    ) -> Result<()> {
        use KeyshareState as K;
        self.state.try_mutate(|s| {
            info!("try_store_pk_share_and_sk_sss");
            let current: GeneratingThresholdShareData = s.clone().try_into()?;
            let esi_sss = current.esi_sss;
            let next = match esi_sss {
                // If the esi shares are here then transition to aggregation
                Some(esi_sss) => K::AggregatingDecryptionKey(AggregatingDecryptionKey {
                    esi_sss,
                    pk_share,
                    sk_sss,
                }),
                // If esi shares are not here yet then don't transition
                None => K::GeneratingThresholdShare(GeneratingThresholdShareData {
                    esi_sss: None,
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                }),
            };
            s.new_state(next)
        })
    }

    pub fn try_store_esi_sss(&mut self, esi_sss: Vec<Encrypted<SharedSecret>>) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;

            info!("try_store_esi_sss");

            let current: GeneratingThresholdShareData = s.clone().try_into()?;
            let pk_share = current.pk_share;
            let sk_sss = current.sk_sss;
            let next = match (pk_share, sk_sss) {
                // If the other shares are here then transition to aggregation
                (Some(pk_share), Some(sk_sss)) => {
                    K::AggregatingDecryptionKey(AggregatingDecryptionKey {
                        esi_sss,
                        pk_share,
                        sk_sss,
                    })
                }
                // If the other shares are not here yet then dont transition
                (None, None) => K::GeneratingThresholdShare(GeneratingThresholdShareData {
                    esi_sss: Some(esi_sss),
                    pk_share: None,
                    sk_sss: None,
                }),
                _ => bail!("Inconsistent state!"),
            };

            s.new_state(next)
        })?;

        info!("esi stored");
        Ok(())
    }

    pub fn set_state_to_decrypting(&mut self) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;

            info!("TRY STORE ESI");
            let current: ReadyForDecryption = s.clone().try_into()?;

            let next = K::Decrypting(Decrypting {
                pk_share: current.pk_share,
                sk_poly_sum: current.sk_poly_sum,
                es_poly_sum: current.es_poly_sum,
            });

            s.new_state(next)
        })?;

        Ok(())
    }

    pub fn try_store_decryption_key(
        &mut self,
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: Vec<SensitiveBytes>,
    ) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;
            info!("Try store decryption key");

            let current: AggregatingDecryptionKey = s.clone().try_into()?;
            let next = K::ReadyForDecryption(ReadyForDecryption {
                pk_share: current.pk_share,
                sk_poly_sum,
                es_poly_sum,
            });

            s.new_state(next)
        })
    }

    pub fn dispatch_keyshare_created(&self) -> Result<()> {
        let state = self.state.get().ok_or(anyhow!("state not set"))?;
        let e3_id = state.get_e3_id().clone();
        let address = state.get_address().to_owned();
        let current: ReadyForDecryption = state.clone().try_into()?;

        self.bus.do_send(EnclaveEvent::from(KeyshareCreated {
            pubkey: current.pk_share.extract_bytes(), // TODO: change KeyshareCreated to accept ArcBytes
            e3_id,
            node: address,
        }));
        Ok(())
    }

    pub fn create_calculate_decryption_share_request(
        &mut self,
        ciphertext_output: Vec<ArcBytes>,
    ) -> Result<ComputeRequest> {
        self.set_state_to_decrypting()?;
        let state = self.state.get().ok_or(anyhow!("State not set."))?;
        let decrypting: Decrypting = state.clone().try_into()?;
        let trbfv_config = state.get_trbfv_config();
        let event = ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(
            CalculateDecryptionShareRequest {
                name: format!("party_id({})", state.party_id),
                ciphertexts: ciphertext_output,
                sk_poly_sum: decrypting.sk_poly_sum,
                es_poly_sum: decrypting.es_poly_sum,
                trbfv_config,
            }
            .into(),
        ));
        Ok(event)
    }

    pub fn create_decryption_share_event(
        &self,
        response: CalculateDecryptionShareResponse,
    ) -> Result<EnclaveEvent> {
        let state = self.state.get().ok_or(anyhow!("Failed to get state"))?;
        let party_id = state.party_id;
        let node = state.address;
        let e3_id = state.e3_id;
        let decryption_share = response.d_share_poly;
        Ok(EnclaveEvent::from(DecryptionshareCreated {
            party_id,
            node,
            e3_id,
            decryption_share,
        }))
    }

    pub fn try_send_decryption_share(&self, event: EnclaveEvent) -> Result<()> {
        let bus = self.bus.clone();
        bus.do_send(event);
        Ok(())
    }

    pub fn try_mark_decryption_share_sent(&mut self) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;
            info!("Decryption share sending process is complete");

            s.new_state(K::Completed)
        })
    }
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => ctx.notify(data),
            EnclaveEvent::ThresholdShareCreated { data, .. } => {
                let _ = self.send_to_decryption_key_collector(data, ctx.address());
            }
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeSelected, ctx: &mut Self::Context) -> Self::Result {
        let _ = self.ensure_collector(ctx.address());
        ctx.notify(StartThresholdShareGeneration(msg));
    }
}

impl Handler<CiphertextOutputPublished> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: CiphertextOutputPublished, _: &mut Self::Context) -> Self::Result {
        let CiphertextOutputPublished {
            ciphertext_output, ..
        } = msg;

        let event = match self.create_calculate_decryption_share_request(ciphertext_output) {
            Ok(request) => request,
            Err(e) => {
                error!("{e}");
                return e3_utils::bail(self);
            }
        };

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, _| {
                    let c = || -> Result<()> {
                        let res = res??;
                        let event = act.create_decryption_share_event(res.try_into()?)?;
                        // send the decryption share
                        act.try_send_decryption_share(event)?;

                        // mark as complete
                        act.try_mark_decryption_share_sent()?;

                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(e) => error!("{:?}", e),
                    }
                }),
        )
    }
}

impl Handler<StartThresholdShareGeneration> for ThresholdKeyshare {
    type Result = Result<()>;
    fn handle(
        &mut self,
        msg: StartThresholdShareGeneration,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let CiphernodeSelected { .. } = msg.0.clone();

        // Initialize State
        self.state.try_mutate(|s| {
            s.new_state(KeyshareState::GeneratingThresholdShare(
                GeneratingThresholdShareData {
                    sk_sss: None,
                    pk_share: None,
                    esi_sss: None,
                },
            ))
        })?;

        // Run both simultaneously
        ctx.notify(GenPkShareAndSkSss(msg.0.clone()));
        ctx.notify_later(GenEsiSss(msg.0), Duration::from_millis(1));
        Ok(())
    }
}

impl Handler<GenEsiSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenEsiSss, _: &mut Self::Context) -> Self::Result {
        info!("GenEsiSss on ThresholdKeyshare");
        let CiphernodeSelected {
            // TODO: should these be on meta? These seem TrBFV specific. perhaps it is best to
            // bundle them in with the params
            error_size,
            esi_per_ct,
            ..
        } = msg.0;

        let Some(state) = self.state.get() else {
            return e3_utils::bail(self);
        };

        let trbfv_config = state.get_trbfv_config();

        let event = ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(
            GenEsiSssRequest {
                trbfv_config,
                error_size,
                esi_per_ct: esi_per_ct as u64,
            }
            .into(),
        ));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let c = || -> Result<()> {
                        info!("\nRECEIVED GEN ESI SSS");

                        let output: GenEsiSssResponse = res??.try_into()?;

                        info!("\nSTORING GEN ESI SSS...");

                        act.try_store_esi_sss(output.esi_sss)?;

                        let Some(state) = act.state.get() else {
                            bail!("State not found.")
                        };

                        match state.variant_name() {
                            "AggregatingDecryptionKey" => ctx.notify(SharesGenerated),
                            _ => bail!("State expected to be AggregatingDecryptionKey"),
                        };

                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(e) => error!("There was an error: GenEsiSss, {}", e),
                    }
                }),
        )
    }
}

impl Handler<GenPkShareAndSkSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenPkShareAndSkSss, _: &mut Self::Context) -> Self::Result {
        info!("GenPkShareAndSkSss on ThresholdKeyshare");
        let CiphernodeSelected { .. } = msg.0;
        let Some(state) = self.state.get() else {
            return e3_utils::bail(self);
        };

        let trbfv_config: TrBFVConfig = state.get_trbfv_config();

        let crp =
            ArcBytes::from_bytes(create_crp(trbfv_config.params(), self.rng.clone()).to_bytes());
        let event = ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            GenPkShareAndSkSssRequest { trbfv_config, crp }.into(),
        ));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let c = || {
                        info!("\nRECEIVED GEN PK SHARE AND SK SSS");
                        let Ok(ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(output))) =
                            res?
                        else {
                            bail!("Error extracting data from compute process")
                        };

                        info!("\nSTORING GEN PK SHARE AND SK SSS...");
                        act.try_store_pk_share_and_sk_sss(output.pk_share, output.sk_sss)?;
                        if let Some(ThresholdKeyshareState {
                            state: KeyshareState::AggregatingDecryptionKey { .. },
                            ..
                        }) = act.state.get()
                        {
                            ctx.notify(SharesGenerated);
                        }
                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(_) => error!("There was an error: GenPkShareAndSkSss"),
                    };
                }),
        )
    }
}

impl Handler<AllThresholdSharesCollected> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: AllThresholdSharesCollected, _: &mut Self::Context) -> Self::Result {
        info!("AllThresholdSharesCollected");
        let Ok(request) = self.try_generate_compute_decryption_key_request(msg) else {
            return e3_utils::bail(self);
        };
        let event = ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(request));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, _| {
                    let c = || {
                        let Ok(ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionKey(
                            output,
                        ))) = res?
                        else {
                            bail!("Error extracting data from compute process")
                        };

                        info!("\nSTORING DECRYPTION KEY...");

                        act.try_store_decryption_key(output.sk_poly_sum, output.es_poly_sum)?;

                        act.dispatch_keyshare_created()?;
                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(_) => error!("There was an error: CalculateDecryptionKey"),
                    };
                }),
        )
    }
}

impl Handler<SharesGenerated> for ThresholdKeyshare {
    type Result = Result<()>;
    fn handle(&mut self, _: SharesGenerated, _: &mut Self::Context) -> Self::Result {
        let Some(ThresholdKeyshareState {
            state:
                KeyshareState::AggregatingDecryptionKey(AggregatingDecryptionKey {
                    pk_share,
                    sk_sss,
                    esi_sss,
                    ..
                }),
            party_id,
            e3_id,
            ..
        }) = self.state.get()
        else {
            bail!("Invalid state!");
        };

        let decrypted = sk_sss.decrypt(&self.cipher)?;
        let sk_sss: PvwEncrypted<SharedSecret> = PvwEncrypted::new(decrypted)?;
        let esi_sss = esi_sss
            .into_iter()
            .map(|s| PvwEncrypted::new(s.decrypt(&self.cipher)?))
            .collect::<Result<_>>()?;

        info!(">>>> THRESHOLD SHARE ABOUT TO BE CREATED FOR {}!", party_id);
        self.bus.do_send(EnclaveEvent::from(ThresholdShareCreated {
            e3_id,
            share: Arc::new(ThresholdShare {
                party_id,
                esi_sss,
                pk_share,
                sk_sss,
            }),
        }));

        Ok(())
    }
}

pub enum CollectorState {
    Collecting { total: u64 },
    Finished,
}

pub struct DecryptionKeyCollector {
    todo: HashSet<PartyId>,
    parent: Addr<ThresholdKeyshare>,
    state: CollectorState,
    shares: HashMap<PartyId, Arc<ThresholdShare>>,
}

impl DecryptionKeyCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64) -> Addr<Self> {
        let addr = Self {
            todo: (0..total).collect(),
            parent,
            state: CollectorState::Collecting { total },
            shares: HashMap::new(),
        }
        .start();
        addr
    }
}

impl Actor for DecryptionKeyCollector {
    type Context = actix::Context<Self>;
}

impl Handler<ThresholdShareCreated> for DecryptionKeyCollector {
    type Result = ();
    fn handle(&mut self, msg: ThresholdShareCreated, _: &mut Self::Context) -> Self::Result {
        let start = Instant::now();
        info!("DecryptionKeyCollector: ThresholdShareCreated received by collector");
        if let CollectorState::Finished = self.state {
            info!("DecryptionKeyCollector is finished so ignoring!");
            return;
        };

        let pid = msg.share.party_id;
        info!("DecryptionKeyCollector party id: {}", pid);
        let Some(_) = self.todo.take(&pid) else {
            info!(
                "Error: {} was not in decryption key collectors ID list",
                pid
            );
            return;
        };
        info!("Inserting... waiting on: {}", self.todo.len());
        self.shares.insert(pid, msg.share);
        if self.todo.len() == 0 {
            info!("We have recieved all the things");
            self.state = CollectorState::Finished;
            let event: AllThresholdSharesCollected = self.shares.clone().into();
            self.parent.do_send(event)
        }
        info!(
            "Finished processing ThresholdShareCreated in {:?}",
            start.elapsed()
        );
    }
}
