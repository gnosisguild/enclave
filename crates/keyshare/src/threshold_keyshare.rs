// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::{anyhow, bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_data::Persistable;
use e3_events::{
    prelude::*, CiphernodeSelected, CiphertextOutputPublished, ComputeRequest, ComputeResponse,
    DecryptionshareCreated, E3id, EnclaveEvent, EnclaveEventData, EventManager, KeyshareCreated,
    PartyId, ThresholdShare, ThresholdShareCreated,
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
    TrBFVConfig, TrBFVRequest, TrBFVResponse,
};
use e3_utils::{bail, to_ordered_vec, utility_types::ArcBytes};
use fhe_traits::Serialize;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{
    collections::HashMap,
    mem,
    sync::{Arc, Mutex},
};
use tracing::{error, info};

use crate::threshold_share_collector::ThresholdShareCollector;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct StartThresholdShareGeneration(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub struct GenPkShareAndSkSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "()")]
pub struct GenEsiSss(CiphernodeSelected);

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
    pub bus: EventManager<EnclaveEvent>,
    pub cipher: Arc<Cipher>,
    pub multithread: Addr<Multithread>,
    pub state: Persistable<ThresholdKeyshareState>,
}

pub struct ThresholdKeyshare {
    bus: EventManager<EnclaveEvent>,
    cipher: Arc<Cipher>,
    decryption_key_collector: Option<Addr<ThresholdShareCollector>>,
    multithread: Addr<Multithread>,
    state: Persistable<ThresholdKeyshareState>,
}

impl ThresholdKeyshare {
    pub fn new(params: ThresholdKeyshareParams) -> Self {
        Self {
            bus: params.bus,
            cipher: params.cipher,
            decryption_key_collector: None,
            multithread: params.multithread,
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
    ) -> Result<Addr<ThresholdShareCollector>> {
        let Some(state) = self.state.get() else {
            bail!("State not found on threshold keyshare. This should not happen.");
        };

        info!(
            "Setting up key collector for addr: {} and {} nodes",
            state.address, state.threshold_n
        );
        let addr = self
            .decryption_key_collector
            .get_or_insert_with(|| ThresholdShareCollector::setup(self_addr, state.threshold_n));
        Ok(addr.clone())
    }

    pub fn handle_threshold_share_created(
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

    /// 1. CiphernodeSelected
    pub fn handle_ciphernode_selected(
        &mut self,
        msg: CiphernodeSelected,
        address: Addr<Self>,
    ) -> Result<()> {
        // Ensure the collector is created
        let _ = self.ensure_collector(address.clone());
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

        address.do_send(GenEsiSss(msg.clone()));
        address.do_send(GenPkShareAndSkSss(msg));
        Ok(())
    }

    /// 2. GenEsiSss
    pub fn handle_gen_esi_sss_requested(&self, msg: GenEsiSss) -> Result<ComputeRequest> {
        info!("GenEsiSss on ThresholdKeyshare");

        let evt = msg.0;
        let CiphernodeSelected {
            // TODO: should these be on meta? These seem TrBFV specific. perhaps it is best to
            // bundle them in with the params
            error_size,
            esi_per_ct,
            ..
        } = evt.clone();

        let state = self
            .state
            .get()
            .ok_or(anyhow!("State not found on ThrehsoldKeyshare"))?;

        let trbfv_config = state.get_trbfv_config();

        let event = ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(
            GenEsiSssRequest {
                trbfv_config,
                error_size,
                esi_per_ct: esi_per_ct as u64,
            }
            .into(),
        ));

        Ok(event)
    }

    /// 2a. GenEsiSss result
    pub fn handle_gen_esi_sss_response(&mut self, res: ComputeResponse) -> Result<()> {
        let output: GenEsiSssResponse = res.try_into()?;

        let esi_sss = output.esi_sss;

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
        if let Some(ThresholdKeyshareState {
            state: KeyshareState::AggregatingDecryptionKey { .. },
            ..
        }) = self.state.get()
        {
            self.handle_shares_generated()?;
        }
        Ok(())
    }

    /// 3. GenPkShareAndSkSss
    pub fn handle_gen_pk_share_and_sk_sss_requested(
        &self,
        msg: GenPkShareAndSkSss,
    ) -> Result<ComputeRequest> {
        info!("GenPkShareAndSkSss on ThresholdKeyshare");
        let CiphernodeSelected { seed, .. } = msg.0;
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
        let event = ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            GenPkShareAndSkSssRequest { trbfv_config, crp }.into(),
        ));

        Ok(event)
    }

    /// 3a. GenPkShareAndSkSss
    pub fn handle_gen_pk_share_and_sk_sss_response(&mut self, res: ComputeResponse) -> Result<()> {
        let ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(output)) = res else {
            bail!("Error extracting data from compute process")
        };

        let (pk_share, sk_sss) = (output.pk_share, output.sk_sss);

        self.state.try_mutate(|s| {
            info!("try_store_pk_share_and_sk_sss");
            let current: GeneratingThresholdShareData = s.clone().try_into()?;
            let esi_sss = current.esi_sss;
            let next = match esi_sss {
                // If the esi shares are here then transition to aggregation
                Some(esi_sss) => {
                    KeyshareState::AggregatingDecryptionKey(AggregatingDecryptionKey {
                        esi_sss,
                        pk_share,
                        sk_sss,
                    })
                }
                // If esi shares are not here yet then don't transition
                None => KeyshareState::GeneratingThresholdShare(GeneratingThresholdShareData {
                    esi_sss: None,
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                }),
            };
            s.new_state(next)
        })?;

        if let Some(ThresholdKeyshareState {
            state: KeyshareState::AggregatingDecryptionKey { .. },
            ..
        }) = self.state.get()
        {
            self.handle_shares_generated()?;
        }
        Ok(())
    }

    /// 4. SharesGenerated
    pub fn handle_shares_generated(&self) -> Result<()> {
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

        self.bus.dispatch(ThresholdShareCreated {
            e3_id,
            share: Arc::new(ThresholdShare {
                party_id,
                esi_sss,
                pk_share,
                sk_sss,
            }),
            external: false,
        });

        Ok(())
    }

    /// 5. AllThresholdSharesCollected. This is fired after the ThresholdShareCreated events are
    ///    aggregateed in the decryption_key_collector::ThresholdShareCollector
    pub fn handle_all_threshold_shares_collected(
        &self,
        msg: AllThresholdSharesCollected,
    ) -> Result<ComputeRequest> {
        info!("AllThresholdSharesCollected");
        let cipher = self.cipher.clone();
        let state = self.state.get().ok_or(anyhow!("No state found"))?;
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

        let request = CalculateDecryptionKeyRequest {
            trbfv_config,
            esi_sss_collected: esi_sss_collected
                .into_iter()
                .map(|s| s.encrypt(&cipher))
                .collect::<Result<_>>()?,
            sk_sss_collected: sk_sss_collected.encrypt(&cipher)?,
        };

        let event = ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(request));

        Ok(event)
    }

    /// 5a. CalculateDecryptionKeyResponse -> KeyshareCreated
    pub fn handle_calculate_decryption_key_response(&mut self, res: ComputeResponse) -> Result<()> {
        let ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionKey(output)) = res else {
            bail!("Error extracting data from compute process")
        };

        let (sk_poly_sum, es_poly_sum) = (output.sk_poly_sum, output.es_poly_sum);

        self.state.try_mutate(|s| {
            use KeyshareState as K;
            info!("Try store decryption key");

            // Attempt to get pk_share from current state
            let current: AggregatingDecryptionKey = s.clone().try_into()?;

            // Transition to ReadyForDecryption
            let next = K::ReadyForDecryption(ReadyForDecryption {
                pk_share: current.pk_share,
                sk_poly_sum,
                es_poly_sum,
            });

            s.new_state(next)
        })?;

        let state = self.state.get().ok_or(anyhow!("No state found"))?;
        let e3_id = state.get_e3_id().clone();
        let address = state.get_address().to_owned();
        let current: ReadyForDecryption = state.clone().try_into()?;

        self.bus.dispatch(KeyshareCreated {
            pubkey: current.pk_share,
            e3_id,
            node: address,
        });

        Ok(())
    }

    /// CiphertextOutputPublished
    pub fn handle_ciphertext_output_published(
        &mut self,
        msg: CiphertextOutputPublished,
    ) -> Result<ComputeRequest> {
        // Set state to decrypting
        self.state.try_mutate(|s| {
            use KeyshareState as K;

            let current: ReadyForDecryption = s.clone().try_into()?;

            let next = K::Decrypting(Decrypting {
                pk_share: current.pk_share,
                sk_poly_sum: current.sk_poly_sum,
                es_poly_sum: current.es_poly_sum,
            });

            s.new_state(next)
        })?;

        let ciphertext_output = msg.ciphertext_output;
        let state = self.state.get().ok_or(anyhow!("No state found"))?;
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

        Ok(event) // CalculateDecryptionShareRequest
    }

    /// CalculateDecryptionShareResponse
    pub fn handle_calculate_decryption_share_response(
        &mut self,
        res: ComputeResponse,
    ) -> Result<()> {
        let msg: CalculateDecryptionShareResponse = res.try_into()?;
        let state = self.state.get().ok_or(anyhow!("No state found"))?;
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
        self.bus.dispatch(event);

        // mark as complete
        self.state.try_mutate(|s| {
            use KeyshareState as K;
            info!("Decryption share sending process is complete");

            s.new_state(K::Completed)
        })?;

        Ok(())
    }

    /// This is handling some of the dark arts of actix
    /// This effectively calls the request function which
    /// generates a ComputeRequest message and then runs the request
    /// on the multithread actor and trigggers the response
    /// handler with the results. Errors at this stage are simply
    /// logged. Eventually we will need to configure a policy here
    /// For example retry with exponential backoff
    fn multithread_request<F, R>(
        &mut self,
        request_fn: F,
        response_fn: R,
    ) -> ResponseActFuture<Self, ()>
    where
        F: FnOnce(&mut Self) -> Result<ComputeRequest>,
        R: FnOnce(&mut Self, ComputeResponse, &mut <Self as Actor>::Context) -> Result<()>
            + 'static,
    {
        // When handling futures in actix you need a pinned box
        // This is so that the future stays in the same spot in memory
        Box::pin(
            // Run the request function and print if there is an error
            match request_fn(self) {
                Ok(evt) => self.multithread.send(evt),
                Err(e) => {
                    error!("{e}");
                    return bail(self);
                }
            }
            .into_actor(self)
            .map(move |res, act, ctx| {
                // Run the response function and print if there is an error
                match (|| -> Result<()> { response_fn(act, res??, ctx) })() {
                    Ok(_) => (),
                    Err(e) => error!("{e}"),
                }
            }),
        )
    }
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::CiphernodeSelected(data) => ctx.notify(data),
            EnclaveEventData::CiphertextOutputPublished(data) => ctx.notify(data),
            EnclaveEventData::ThresholdShareCreated(data) => {
                let _ = self.handle_threshold_share_created(data, ctx.address());
            }
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeSelected, ctx: &mut Self::Context) -> Self::Result {
        match self.handle_ciphernode_selected(msg, ctx.address()) {
            Err(e) => error!("{e}"),
            Ok(_) => (),
        }
    }
}

impl Handler<GenEsiSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenEsiSss, _: &mut Self::Context) -> Self::Result {
        self.multithread_request(
            |act| act.handle_gen_esi_sss_requested(msg),
            |act, res, _| act.handle_gen_esi_sss_response(res),
        )
    }
}

impl Handler<GenPkShareAndSkSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenPkShareAndSkSss, _: &mut Self::Context) -> Self::Result {
        self.multithread_request(
            |act| act.handle_gen_pk_share_and_sk_sss_requested(msg),
            |act, res, _| act.handle_gen_pk_share_and_sk_sss_response(res),
        )
    }
}

impl Handler<AllThresholdSharesCollected> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: AllThresholdSharesCollected, _: &mut Self::Context) -> Self::Result {
        self.multithread_request(
            |act| act.handle_all_threshold_shares_collected(msg),
            |act, res, _| act.handle_calculate_decryption_key_response(res),
        )
    }
}

impl Handler<CiphertextOutputPublished> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: CiphertextOutputPublished, _: &mut Self::Context) -> Self::Result {
        self.multithread_request(
            |act| act.handle_ciphertext_output_published(msg),
            |act, res, _| act.handle_calculate_decryption_share_response(res),
        )
    }
}
