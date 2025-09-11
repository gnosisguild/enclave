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
    CiphernodeSelected, ComputeRequest, ComputeResponse, E3id, EnclaveEvent, EventBus,
    KeyshareCreated, PartyId, ThresholdShare, ThresholdShareCreated,
};
use e3_fhe::create_crp;
use e3_multithread::Multithread;
use e3_trbfv::{
    calculate_decryption_key::CalculateDecryptionKeyRequest, gen_esi_sss::GenEsiSssRequest,
    gen_pk_share_and_sk_sss::GenPkShareAndSkSssRequest, SharedRng, TrBFVConfig, TrBFVRequest,
    TrBFVResponse,
};
use fhe_traits::Serialize;
use zeroize::Zeroizing;

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
        // At this point all parties should be accounted for here

        // extract a vector
        let mut pairs: Vec<_> = value.into_iter().collect();

        // Ensure keys are sorted
        pairs.sort_by_key(|&(key, _)| key);

        // Extract to Vec of ThresholdShares in order
        let shares = pairs.into_iter().map(|(_, value)| value).collect();

        AllThresholdSharesCollected { shares }
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GeneratingThresholdShareData {
    pk_share: Option<Arc<Vec<u8>>>,
    sk_sss: Option<Vec<SensitiveBytes>>,
    esi_sss: Option<Vec<Vec<SensitiveBytes>>>,
}

// TODO: Add GeneratingPvwKey state
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum KeyshareState {
    // Before anything
    Init,
    // Generating TrBFV share material
    GeneratingThresholdShare {
        pk_share: Option<Arc<Vec<u8>>>,
        sk_sss: Option<Vec<SensitiveBytes>>,
        esi_sss: Option<Vec<Vec<SensitiveBytes>>>,
    },
    // Collecting remaining TrBFV shares to aggregate decryption key
    AggregatingDecryptionKey {
        pk_share: Arc<Vec<u8>>,
        sk_sss: Vec<SensitiveBytes>,
        esi_sss: Vec<Vec<SensitiveBytes>>,
    },
    // Awaiting decryption
    ReadyForDecryption {
        pk_share: Arc<Vec<u8>>,
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: Vec<SensitiveBytes>,
    },
    Decrypting, // Decrypting something
    Completed,  // Finished
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
                    (K::Init, K::GeneratingThresholdShare { .. }) => true,
                    (K::GeneratingThresholdShare { .. }, K::AggregatingDecryptionKey { .. }) => {
                        true
                    }
                    (K::AggregatingDecryptionKey { .. }, K::ReadyForDecryption { .. }) => true,
                    (K::ReadyForDecryption { .. }, K::Decrypting) => true,
                    (K::Decrypting, K::Completed) => true,
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
            Self::GeneratingThresholdShare { .. } => "GeneratingThresholdShare",
            Self::AggregatingDecryptionKey { .. } => "AggregatingDecryptionKey",
            Self::ReadyForDecryption { .. } => "ReadyForDecryption",
            Self::Decrypting => "Decrypting",
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
    pub params: Arc<Vec<u8>>,
}

impl ThresholdKeyshareState {
    pub fn new(
        e3_id: E3id,
        party_id: PartyId,
        state: KeyshareState,
        threshold_m: u64,
        threshold_n: u64,
        params: Arc<Vec<u8>>,
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
}

impl From<ThresholdKeyshareState> for TrBFVConfig {
    fn from(value: ThresholdKeyshareState) -> Self {
        TrBFVConfig::new(value.params, value.threshold_n, value.threshold_m)
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
        println!("Created ThresholdKeyshare!");
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

        println!(
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
        let trbfv_config = state.into();

        // Shares are in order of party_id
        let mut sk_sss_collected = vec![];
        let mut esi_sss_collected = vec![];
        for share in msg.shares {
            sk_sss_collected.push(SensitiveBytes::new(
                share.sk_sss[party_id].clone(),
                &cipher,
            )?);
            esi_sss_collected.push(SensitiveBytes::try_from_vec(
                share.esi_sss[party_id].clone(),
                &cipher,
            )?);
        }

        Ok(CalculateDecryptionKeyRequest {
            trbfv_config,
            esi_sss_collected,
            sk_sss_collected,
        })
    }

    pub fn send_to_decryption_key_collector(
        &mut self,
        msg: ThresholdShareCreated,
        self_addr: Addr<Self>,
    ) -> Result<()> {
        println!("Sending ThresholdShareCreated to collector!");
        let collector = self.ensure_collector(self_addr)?;
        println!("got collector address!");
        collector.do_send(msg);
        Ok(())
    }

    pub fn try_store_pk_share_and_sk_sss(
        &mut self,
        pk_share: Arc<Vec<u8>>,
        sk_sss: Vec<SensitiveBytes>,
    ) -> Result<()> {
        use KeyshareState as K;
        self.state.try_mutate(|s| {
            println!("TRY STORE PK");

            let next = match s.state.clone() {
                // If the esi shares are here then transition to aggregation
                K::GeneratingThresholdShare {
                    esi_sss: Some(esi_sss),
                    ..
                } => K::AggregatingDecryptionKey {
                    esi_sss,
                    pk_share,
                    sk_sss,
                },
                // If esi shares are not here yet then don't transition
                K::GeneratingThresholdShare { esi_sss: None, .. } => K::GeneratingThresholdShare {
                    esi_sss: None,
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                },
                _ => bail!("Inconsistent state!"),
            };
            s.new_state(next)
        })
    }

    pub fn try_store_esi_sss(&mut self, esi_sss: Vec<Vec<SensitiveBytes>>) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;

            println!("TRY STORE ESI");

            let next = match s.state.clone() {
                // If the other shares are here then transition to aggregation
                K::GeneratingThresholdShare {
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                    ..
                } => K::AggregatingDecryptionKey {
                    esi_sss,
                    pk_share,
                    sk_sss,
                },
                // If the other shares are not here yet then dont transition
                K::GeneratingThresholdShare {
                    sk_sss: None,
                    pk_share: None,
                    ..
                } => K::GeneratingThresholdShare {
                    esi_sss: Some(esi_sss),
                    pk_share: None,
                    sk_sss: None,
                },
                _ => bail!("Inconsistent state!"),
            };

            s.new_state(next)
        })?;

        println!("ESI STORED");
        Ok(())
    }

    pub fn try_store_decryption_key(
        &mut self,
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: Vec<SensitiveBytes>,
    ) -> Result<()> {
        self.state.try_mutate(|s| {
            use KeyshareState as K;
            println!("Try store decryption key");

            let next = match s.state.clone() {
                K::AggregatingDecryptionKey { pk_share, .. } => K::ReadyForDecryption {
                    pk_share,
                    sk_poly_sum,
                    es_poly_sum,
                },
                _ => bail!(
                    "State must be aggregating decryption key in order to store decryption key"
                ),
            };

            s.new_state(next)
        })
    }

    pub fn dispatch_keyshare_created(&self) {
        use KeyshareState as K;
        if let Some(ThresholdKeyshareState {
            state: K::ReadyForDecryption { pk_share, .. },
            e3_id,
            address,
            ..
        }) = self.state.get()
        {
            self.bus.do_send(EnclaveEvent::from(KeyshareCreated {
                pubkey: (*pk_share).clone(),
                e3_id,
                node: address.clone(),
            }));
        } else {
            println!("Could not dispatch KeyshareCreated");
        }
    }
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
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
            s.new_state(KeyshareState::GeneratingThresholdShare {
                sk_sss: None,
                pk_share: None,
                esi_sss: None,
            })
        })?;

        // Run both simultaneously
        ctx.notify(GenPkShareAndSkSss(msg.0.clone()));
        ctx.notify_later(GenEsiSss(msg.0), Duration::from_millis(1));
        Ok(())
    }
}

fn bail<T: Actor>(a: &T) -> ResponseActFuture<T, ()> {
    Box::pin(async {}.into_actor(a))
}

fn bail_result<T: Actor>(a: &T, msg: impl Into<String>) -> ResponseActFuture<T, Result<()>> {
    let m: String = msg.into();
    Box::pin(async { Err(anyhow!(m)) }.into_actor(a))
}

impl Handler<GenEsiSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenEsiSss, _: &mut Self::Context) -> Self::Result {
        println!("GenEsiSss on ThresholdKeyshare");
        let CiphernodeSelected {
            // TODO: should these be on meta? These seem TrBFV specific. perhaps it is best to
            // bundle them in with the params
            error_size,
            esi_per_ct,
            ..
        } = msg.0;

        let Some(state) = self.state.get() else {
            return bail(self);
        };

        let trbfv_config = state.into();

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
                    let c = || {
                        println!("\nRECEIVED GEN ESI SSS");
                        let Ok(ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(output))) = res?
                        else {
                            bail!("Error extracting data from compute process")
                        };

                        println!("\nSTORING GEN ESI SSS...");
                        act.try_store_esi_sss(output.esi_sss)?;

                        if let Some(ThresholdKeyshareState {
                            state: KeyshareState::AggregatingDecryptionKey { .. },
                            ..
                        }) = act.state.get()
                        {
                            ctx.notify(SharesGenerated)
                        }

                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(e) => println!("There was an error: GenEsiSss, {}", e),
                    };
                }),
        )
    }
}

impl Handler<GenPkShareAndSkSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, msg: GenPkShareAndSkSss, ctx: &mut Self::Context) -> Self::Result {
        println!("GenPkShareAndSkSss on ThresholdKeyshare");
        let CiphernodeSelected { .. } = msg.0;
        let Some(state) = self.state.get() else {
            return bail(self);
        };

        let trbfv_config: TrBFVConfig = state.into();

        let crp = Arc::new(create_crp(trbfv_config.params(), self.rng.clone()).to_bytes());
        let event = ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            GenPkShareAndSkSssRequest { trbfv_config, crp }.into(),
        ));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let c = || {
                        println!("\nRECEIVED GEN PK SHARE AND SK SSS");
                        let Ok(ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(output))) =
                            res?
                        else {
                            bail!("Error extracting data from compute process")
                        };

                        println!("\nSTORING GEN PK SHARE AND SK SSS...");
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
                        Err(e) => println!("There was an error: GenPkShareAndSkSss"),
                    };
                }),
        )
    }
}

impl Handler<AllThresholdSharesCollected> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(
        &mut self,
        msg: AllThresholdSharesCollected,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        println!("AllThresholdSharesCollected");
        let Ok(request) = self.try_generate_compute_decryption_key_request(msg) else {
            return bail(self);
        };
        let event = ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(request));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let c = || {
                        let Ok(ComputeResponse::TrBFV(TrBFVResponse::CalculateDecryptionKey(
                            output,
                        ))) = res?
                        else {
                            bail!("Error extracting data from compute process")
                        };

                        println!("\nSTORING DECRYPTION KEY...");

                        act.try_store_decryption_key(output.sk_poly_sum, output.es_poly_sum)?;

                        act.dispatch_keyshare_created();
                        Ok(())
                    };

                    match c() {
                        Ok(_) => (),
                        Err(e) => println!("There was an error: CalculateDecryptionKey"),
                    };
                }),
        )
    }
}

impl Handler<SharesGenerated> for ThresholdKeyshare {
    type Result = Result<()>;
    fn handle(&mut self, msg: SharesGenerated, ctx: &mut Self::Context) -> Self::Result {
        let Some(ThresholdKeyshareState {
            state:
                KeyshareState::AggregatingDecryptionKey {
                    pk_share,
                    sk_sss,
                    esi_sss,
                    ..
                },
            party_id,
            e3_id,
            ..
        }) = self.state.get()
        else {
            bail!("Invalid state!");
        };

        let sk_sss = SensitiveBytes::access_vec(sk_sss, &self.cipher)?;
        let esi_sss = esi_sss
            .into_iter()
            .map(|s| SensitiveBytes::access_vec(s, &self.cipher))
            .collect::<Result<Vec<_>>>()?;

        // TODO: pvw encrypt all data
        // Currently this removes zeroizing to create bytes
        let (pk_share, sk_sss, esi_sss) =
            _dangerously_remove_zeroizing_to_simulate_pvw_encryption((pk_share, sk_sss, esi_sss));
        println!(">>>> THRESHOLD SHARE ABOUT TO BE CREATED FOR {}!", party_id);
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
    fn handle(&mut self, msg: ThresholdShareCreated, ctx: &mut Self::Context) -> Self::Result {
        let start = Instant::now();
        println!("DecryptionKeyCollector received event");
        if let CollectorState::Finished = self.state {
            println!("DecryptionKeyCollector is finished so ignoring!");
            return;
        };

        let pid = msg.share.party_id;
        println!("DecryptionKeyCollector party id: {}", pid);
        let Some(_) = self.todo.take(&pid) else {
            println!(
                "Error: {} was not in decryption key collectors ID list",
                pid
            );
            return;
        };
        println!("Inserting... waiting on: {}", self.todo.len());
        self.shares.insert(pid, msg.share);
        if self.todo.len() == 0 {
            println!("We have recieved all the things");
            self.state = CollectorState::Finished;
            let event: AllThresholdSharesCollected = self.shares.clone().into();
            self.parent.do_send(event)
        }
        println!(
            "Finished processing ThresholdShareCreated in {:?}",
            start.elapsed()
        );
    }
}

// Function to prepare tuple to put on an event
fn _dangerously_remove_zeroizing_to_simulate_pvw_encryption(
    input: (
        Arc<Vec<u8>>,
        Vec<Zeroizing<Vec<u8>>>,
        Vec<Vec<Zeroizing<Vec<u8>>>>,
    ),
) -> (Arc<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<Vec<u8>>>) {
    let (first, second, third) = input;

    (
        first,                                            // Arc<Vec<u8>> stays the same
        second.into_iter().map(|z| z.to_vec()).collect(), // Vec<Zeroizing<Vec<u8>>> -> Vec<Vec<u8>>
        third
            .into_iter()
            .map(|outer_vec| outer_vec.into_iter().map(|z| z.to_vec()).collect())
            .collect(), // Vec<Vec<Zeroizing<Vec<u8>>>> -> Vec<Vec<Vec<u8>>>
    )
}
