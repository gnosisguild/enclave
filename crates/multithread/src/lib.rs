use std::sync::Arc;

use actix::prelude::*;
use actix::{Actor, Handler};
use anyhow::Result;
use e3_crypto::Cipher;
use e3_events::trbfv::TrBFVRequest;
use e3_events::{ComputeRequest, ComputeRequested, EnclaveEvent, EventBus, Subscribe};

/// Multithread actor
pub struct Multithread {
    bus: Addr<EventBus<EnclaveEvent>>,
    cipher: Arc<Cipher>,
}

impl Multithread {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent>>, cipher: Arc<Cipher>) -> Self {
        Self {
            cipher,
            bus: bus.clone(),
        }
    }

    pub fn attach(bus: &Addr<EventBus<EnclaveEvent>>, cipher: Arc<Cipher>) -> Addr<Self> {
        let addr = Self::new(bus, cipher).start();
        bus.do_send(Subscribe::new("ComputeRequested", addr.clone().recipient()));
        addr
    }
}

impl Actor for Multithread {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Multithread {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::ComputeRequested { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<ComputeRequested> for Multithread {
    type Result = ResponseFuture<Result<()>>;

    fn handle(&mut self, msg: ComputeRequested, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(async move {
            let _ = handle_compute_request(msg.request).await;
            Ok(())
        })
    }
}

async fn handle_compute_request(request: ComputeRequest) {
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(r)) => {
            let _ = e3_trbfv::gen_esi_sss(r.trbfv_config, r.error_size, r.esi_per_ct).await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(r)) => {
            let _ = e3_trbfv::gen_pk_share_and_sk_sss(r.trbfv_config, r.crp).await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(r)) => {
            let _ = e3_trbfv::calculate_decryption_key(
                r.trbfv_config,
                vec![vec![]], // XXX:
                vec![vec![]], // XXX:
            )
            .await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(r)) => {
            let _ = e3_trbfv::calculate_decryption_key(
                r.trbfv_config,
                vec![], // XXX:
                vec![], // XXX:
            )
            .await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(r)) => {
            let _ = e3_trbfv::calculate_threshold_decryption(
                r.trbfv_config,
                r.ciphertext,
                r.d_share_polys,
            )
            .await;
        }
        _ => (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        assert!(true);
    }
}
