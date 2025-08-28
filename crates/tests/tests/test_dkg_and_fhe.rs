// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_crypto::Cipher;
use e3_events::{wait_for_event, ComputeRequest, ComputeRequested, CorrelationId, EnclaveEvent};
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
    let _multi = Multithread::new(&bus, rng, cipher).start();
    let correlation_id = CorrelationId::new();
    // Generate initial pk and sk sss
    let gen_pk_share_and_sk_sss = EnclaveEvent::from(ComputeRequested {
        correlation_id: correlation_id.clone(),
        request: ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(
            e3_trbfv::gen_pk_share_and_sk_sss::Request {
                trbfv_config: TrBFVConfig::new(params_bytes, num_parties, threshold),
                crp: crp_bytes,
            },
        )),
    });

    let waiter = wait_for_event(
        &bus,
        Box::new(move |event| match event {
            EnclaveEvent::ComputeRequested {
                data:
                    ComputeRequested {
                        correlation_id: id, ..
                    },
                ..
            } => id == &correlation_id,
            _ => false,
        }),
    );
    bus.do_send(gen_pk_share_and_sk_sss);
    waiter.await?;

    Ok(())
}
