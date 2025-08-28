use actix::prelude::*;
use anyhow::Result;
use e3_crypto::Cipher;
use e3_events::{ComputeRequest, ComputeRequested, CorrelationId, EnclaveEvent};
use e3_multithread::Multithread;
use e3_test_helpers::get_common_setup;
use e3_trbfv::{TrBFVConfig, TrBFVRequest};
use fhe_traits::Serialize;
use std::sync::Arc;

#[actix::test]
async fn test_trbfv() -> Result<()> {
    let (bus, rng, seed, params, crpoly, _, history_collector) = get_common_setup()?;
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let params_bytes = Arc::new(params.to_bytes());
    let crp_bytes = Arc::new(crpoly.to_bytes());
    let num_parties = 5;
    let threshold = 3;

    // Setup multithread processor
    // TODO: Currently only testing logic not setup on multithread yet
    let multi = Multithread::new(&bus, rng, cipher).start();

    // Generate initial pk and sk sss
    let gen_pk_share_and_sk_sss = EnclaveEvent::from(ComputeRequested {
        correlation_id: CorrelationId::new(),
        request: ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            e3_trbfv::gen_pk_share_and_sk_sss::Request {
                trbfv_config: TrBFVConfig::new(params_bytes, num_parties, threshold),
                crp: crp_bytes,
            },
        )),
    });
    bus.send(gen_pk_share_and_sk_sss).await?;

    // assert!(false);
    Ok(())
}
