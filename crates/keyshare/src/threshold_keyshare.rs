use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Handler};
use e3_crypto::Cipher;
use e3_events::{CiphernodeSelected, EnclaveEvent, EventBus};
use e3_fhe::set_up_crp;
use e3_trbfv::{SharedRng, TrBFVConfig};
use fhe_traits::Serialize;

// XXX: correlation of compute events... this gets events via an e3_id filter
// Why run all compute events over the bus? For logging and to allow other components to react to -
// we could send events directly to the multithread actor which means the responses would come
// straight back
// directly eg let res = multithread_addr.send(ComputeRequested).await
// We may not actually need the correlation_id

// - [ ] Extract compute events from EnclaveEvent
// - [ ] Make test use EnclaveEvents to test workflow
// - [ ] Remove "wait for event"

pub struct ThresholdKeyshare {
    bus: Addr<EventBus<EnclaveEvent>>,
    cipher: Arc<Cipher>,
    rng: SharedRng,
}

impl Actor for ThresholdKeyshare {
    type Context = actix::Context<Self>;
}

// Will only receive events that are for this specific e3_id
impl Handler<EnclaveEvent> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<CiphernodeSelected> for ThresholdKeyshare {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeSelected, ctx: &mut Self::Context) -> Self::Result {
        let CiphernodeSelected {
            e3_id,
            params,
            threshold_n,
            threshold_m,
            ..
        } = msg;

        let trbfv_config = TrBFVConfig::new(params.clone(), threshold_n as u64, threshold_m as u64);
        let crp = Arc::new(set_up_crp(trbfv_config.params(), self.rng.clone()).to_bytes());

        // Need to trigger then correlate and it must be easy...
        // trigger DKG
        self.bus.do_send::<EnclaveEvent>(
            e3_trbfv::gen_pk_share_and_sk_sss::Request { trbfv_config, crp }.into(),
        )
    }
}
