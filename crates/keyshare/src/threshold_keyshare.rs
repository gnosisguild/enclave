// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{mem, sync::Arc};

use actix::prelude::*;
use anyhow::{anyhow, bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_data::Persistable;
use e3_events::{
    CiphernodeSelected, ComputeRequest, ComputeResponse, E3id, EnclaveEvent, EventBus,
    ThresholdShare, ThresholdShareCreated,
};
use e3_fhe::set_up_crp;
use e3_multithread::Multithread;
use e3_trbfv::{gen_pk_share_and_sk_sss, SharedRng, TrBFVConfig, TrBFVRequest, TrBFVResponse};
use fhe_traits::Serialize;
use zeroize::Zeroizing;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct StartThresholdShareGeneration(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct GenPkShareAndSkSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct GenEsiSss(CiphernodeSelected);

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[rtype(result = "Result<()>")]
struct SharesGenerated;

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
        party_id: u64,
        pk_share: Option<Arc<Vec<u8>>>,
        sk_sss: Option<Vec<SensitiveBytes>>,
        esi_sss: Option<Vec<Vec<SensitiveBytes>>>,
    },
    // Collecting remaining TrBFV shares to aggregate decryption key
    AggregatingDecryptionKey {
        party_id: u64,
        pk_share: Arc<Vec<u8>>,
        sk_sss: Vec<SensitiveBytes>,
        esi_sss: Vec<Vec<SensitiveBytes>>,
    },
    ReadyForDecryption, // Awaiting decryption
    Decrypting,         // Decrypting something
    Completed,          // Finished
}

impl KeyshareState {
    pub fn next(self: &KeyshareState, new_state: KeyshareState) -> Result<KeyshareState> {
        use KeyshareState as K;
        let valid = {
            // If we are in the same branch the new state is valid
            if mem::discriminant(self) == mem::discriminant(&new_state) {
                true
            } else {
                match (self, &new_state) {
                    (K::Init, K::GeneratingThresholdShare { .. }) => true,
                    (K::AggregatingDecryptionKey { .. }, K::ReadyForDecryption) => true,
                    (K::ReadyForDecryption, K::Decrypting) => true,
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
                self,
                new_state
            ))
        }
    }
}

impl Default for KeyshareState {
    fn default() -> Self {
        KeyshareState::Init
    }
}

pub struct ThresholdKeyshare {
    e3_id: E3id,
    bus: Addr<EventBus<EnclaveEvent>>,
    multithread: Addr<Multithread>,
    cipher: Arc<Cipher>,
    state: Persistable<KeyshareState>,
    decryption_key_collector: Option<Addr<DecryptionKeyCollector>>,
    rng: SharedRng,
}

impl Actor for ThresholdKeyshare {
    type Context = actix::Context<Self>;
}

impl ThresholdKeyshare {
    pub fn ensure_collector(&mut self, self_addr: Addr<Self>) -> Addr<DecryptionKeyCollector> {
        let addr = self
            .decryption_key_collector
            .get_or_insert_with(|| DecryptionKeyCollector { parent: self_addr }.start());
        addr.clone()
    }

    pub fn send_to_decryption_key_collector(
        &mut self,
        msg: ThresholdShareCreated,
        self_addr: Addr<Self>,
    ) {
        let collector = self.ensure_collector(self_addr);
        collector.do_send(msg);
    }

    pub fn try_store_pk_share_and_sk_sss(
        &mut self,
        pk_share: Arc<Vec<u8>>,
        sk_sss: Vec<SensitiveBytes>,
    ) -> Result<()> {
        use KeyshareState as K;
        self.state.try_mutate(|s| {
            let K::GeneratingThresholdShare {
                party_id, esi_sss, ..
            } = s.clone()
            else {
                bail!("Cannot store pkshare and sk sss on state {:?}", s);
            };

            match esi_sss {
                Some(esi_sss) => Ok(s.next(K::AggregatingDecryptionKey {
                    party_id,
                    esi_sss,
                    pk_share,
                    sk_sss,
                })?),
                None => Ok(s.next(K::GeneratingThresholdShare {
                    party_id,
                    esi_sss,
                    pk_share: Some(pk_share),
                    sk_sss: Some(sk_sss),
                })?),
            }
        })
    }

    pub fn try_store_esi_sss(&mut self, esi_sss: Vec<Vec<SensitiveBytes>>) -> Result<()> {
        use KeyshareState as K;
        self.state.try_mutate(|s| {
            let K::GeneratingThresholdShare {
                sk_sss,
                pk_share,
                party_id,
                ..
            } = s.clone()
            else {
                bail!("Cannot store esi_sss on state {:?}", s);
            };
            match (sk_sss, pk_share) {
                (Some(sk_sss), Some(pk_share)) => Ok(s.next(K::AggregatingDecryptionKey {
                    party_id,
                    esi_sss,
                    pk_share,
                    sk_sss,
                })?),
                (None, None) => Ok(s.next(K::GeneratingThresholdShare {
                    party_id,
                    sk_sss: None,
                    pk_share: None,
                    esi_sss: Some(esi_sss),
                })?),
                _ => bail!("Inconsistent state!"),
            }
        })
    }
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::ThresholdShareCreated { data, .. } => {
                self.send_to_decryption_key_collector(data, ctx.address())
            }
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeSelected, ctx: &mut Self::Context) -> Self::Result {
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
        let party_id = msg.0.party_id;

        // Initialize State
        self.state.try_mutate(|s| {
            Ok(s.next(KeyshareState::GeneratingThresholdShare {
                party_id,
                sk_sss: None,
                pk_share: None,
                esi_sss: None,
            })?)
        })?;

        // Run both simultaneously
        ctx.notify(GenPkShareAndSkSss(msg.0.clone()));
        ctx.notify(GenEsiSss(msg.0));
        Ok(())
    }
}

impl Handler<GenEsiSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, Result<()>>;
    fn handle(&mut self, msg: GenEsiSss, ctx: &mut Self::Context) -> Self::Result {
        let CiphernodeSelected {
            params,
            threshold_n,
            threshold_m,
            error_size,
            esi_per_ct,
            ..
        } = msg.0;

        let trbfv_config = TrBFVConfig::new(params.clone(), threshold_n as u64, threshold_m as u64);
        let event = ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(
            e3_trbfv::gen_esi_sss::Request {
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
                    let Ok(ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(output))) = res? else {
                        bail!("Error extracting data from compute process")
                    };

                    act.try_store_esi_sss(output.esi_sss)?;
                    if let Some(KeyshareState::AggregatingDecryptionKey { .. }) = act.state.get() {
                        ctx.notify(SharesGenerated)
                    }
                    Ok(())
                }),
        )
    }
}

impl Handler<GenPkShareAndSkSss> for ThresholdKeyshare {
    type Result = ResponseActFuture<Self, Result<()>>;
    fn handle(&mut self, msg: GenPkShareAndSkSss, ctx: &mut Self::Context) -> Self::Result {
        let CiphernodeSelected {
            params,
            threshold_n,
            threshold_m,
            ..
        } = msg.0;

        let trbfv_config = TrBFVConfig::new(params.clone(), threshold_n as u64, threshold_m as u64);
        let crp = Arc::new(set_up_crp(trbfv_config.params(), self.rng.clone()).to_bytes());
        let event = ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            gen_pk_share_and_sk_sss::Request { trbfv_config, crp }.into(),
        ));

        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let Ok(e3_events::ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(
                        output,
                    ))) = res?
                    else {
                        bail!("Error extracting data from compute process")
                    };

                    act.try_store_pk_share_and_sk_sss(output.pk_share, output.sk_sss)?;
                    if let Some(KeyshareState::AggregatingDecryptionKey { .. }) = act.state.get() {
                        ctx.notify(SharesGenerated);
                    }
                    Ok(())
                }),
        )
    }
}

impl Handler<SharesGenerated> for ThresholdKeyshare {
    type Result = Result<()>;
    fn handle(&mut self, msg: SharesGenerated, ctx: &mut Self::Context) -> Self::Result {
        let Some(KeyshareState::AggregatingDecryptionKey {
            party_id,
            pk_share,
            sk_sss,
            esi_sss,
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
        let (pk_share, sk_sss, esi_sss) =
            _dangerously_remove_zeroizing_to_simulate_pvw_encryption((pk_share, sk_sss, esi_sss));

        self.bus.do_send(EnclaveEvent::from(ThresholdShareCreated {
            e3_id: self.e3_id.clone(),
            share: ThresholdShare {
                party_id,
                esi_sss,
                pk_share,
                sk_sss,
            },
        }));

        Ok(())
    }
}

pub struct DecryptionKeyCollector {
    parent: Addr<ThresholdKeyshare>,
}

impl DecryptionKeyCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>) -> Addr<Self> {
        let addr = Self { parent }.start();
        addr
    }
}

impl Actor for DecryptionKeyCollector {
    type Context = actix::Context<Self>;
}

impl Handler<ThresholdShareCreated> for DecryptionKeyCollector {
    type Result = ();
    fn handle(&mut self, msg: ThresholdShareCreated, ctx: &mut Self::Context) -> Self::Result {}
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
