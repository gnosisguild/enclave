use actix::prelude::*;
use actix::{Actor, Handler};
use e3_events::trbfv::TrBFVRequest;
use e3_events::{ComputeRequest, ComputeRequested, EnclaveEvent};

pub struct Multithread;

impl Multithread {
    pub fn new() -> Self {
        Self {}
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
    type Result = ();
    fn handle(&mut self, msg: ComputeRequested, _ctx: &mut Self::Context) -> Self::Result {
        // XXX:
        // handle_compute_request(msg.request)
    }
}

async fn handle_compute_request(request: ComputeRequest) {
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(_)) => e3_trbfv::gen_esi_sss().await,
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(_)) => {
            e3_trbfv::gen_pk_share_and_sk_sss().await
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(_)) => {
            e3_trbfv::calculate_decryption_key().await
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(_)) => {
            e3_trbfv::calculate_decryption_key().await
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(_)) => {
            e3_trbfv::calculate_threshold_decryption().await
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
