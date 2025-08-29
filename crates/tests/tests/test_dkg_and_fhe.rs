// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_events::{wait_for_event, EnclaveEvent};
use e3_multithread::Multithread;
use e3_sdk::bfv_helpers::encode_bfv_params;
use e3_test_helpers::get_common_setup;
use e3_trbfv::{TrBFVConfig, TrBFVRequest, TrBFVResponse};
use fhe_rs::{
    bfv,
    trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig},
};
use fhe_traits::Serialize;
use num_bigint::BigUint;
use std::{fs, sync::Arc};

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

pub fn calculate_error_size(
    params: Arc<bfv::BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
) -> Result<BigUint> {
    let config = SmudgingBoundCalculatorConfig::new(params, n, num_ciphertexts);
    let calculator = SmudgingBoundCalculator::new(config);
    Ok(calculator.calculate_sm_bound()?)
}

#[actix::test]
async fn test_trbfv() -> Result<()> {
    let (bus, rng, _seed, params, crpoly, _, _) = get_common_setup(Some((
        8192usize,
        16384u64,
        &[
            0x1FFFFFFEA0001u64, // 562949951979521
            0x1FFFFFFE88001u64, // 562949951881217
            0x1FFFFFFE48001u64, // 562949951619073
            0xfffffebc001u64,   //
        ] as &[u64],
    )))?;
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let params_bytes = Arc::new(encode_bfv_params(&params));
    let crp_bytes = Arc::new(crpoly.to_bytes());
    let num_parties = 5;
    let threshold = 3;

    let error_size_bigint = calculate_error_size(params, num_parties as usize, 3)?;
    let error_size = Arc::new(BigUint::to_bytes_be(&error_size_bigint));

    // Setup multithread processor
    // TODO: Currently only testing logic not setup on multithread yet
    let multi = Multithread::attach(&bus, rng, cipher.clone());

    // Generate initial pk and sk sss
    let gen_pk_share_and_sk_sss: EnclaveEvent = e3_trbfv::gen_pk_share_and_sk_sss::Request {
        trbfv_config: TrBFVConfig::new(params_bytes.clone(), num_parties, threshold),
        crp: crp_bytes,
    }
    .into();

    let correlation_id = gen_pk_share_and_sk_sss.correlation_id().unwrap();

    // Now lets setup a waiter to wait for the response
    let wait_for_response = wait_for_event(
        &bus,
        Box::new(move |e| match e {
            EnclaveEvent::ComputeRequestSucceeded { data, .. } => {
                data.correlation_id == correlation_id
            }
            _ => false,
        }),
    );

    // Send the event
    bus.do_send(gen_pk_share_and_sk_sss.clone());

    let response_event = wait_for_response.await??;

    let Some(TrBFVResponse::GenPkShareAndSkSss(response)) = response_event.trbfv_response() else {
        bail!("bad response");
    };

    let pk_share = response.pk_share.clone();
    // let sk_sss = SensitiveBytes::access_vec(response.sk_sss.clone(), &cipher)?;
    // let serialized_sk_sss = sk_sss[0].iter().map(|v| v).copied().collect::<Vec<_>>();

    // .iter().flat_map(|v| v.iter()).copied().collect();
    // NOTE: uncomment to save new snapshot. Note rng is deterministic so snapshots are possible
    // save_snapshot("fixtures/01_pk_share.bin", &pk_share[..]);
    // save_snapshot("fixtures/02_sk_sss.bin", &serialized_sk_sss);

    // Ensure that correct public key share has been created
    assert_eq!(
        pk_share,
        Arc::new(include_bytes!("fixtures/01_pk_share.bin").to_vec())
    );

    // TODO: verify encrypted sk_sss are correct
    // currently we dont do this as we need to decrypt the bytes
    // assert_eq!(response.sk_sss, Arc::new(expected.to_vec()));

    let gen_esi_sss: EnclaveEvent = e3_trbfv::gen_esi_sss::Request {
        trbfv_config: TrBFVConfig::new(params_bytes.clone(), num_parties, threshold),
        error_size,
        esi_per_ct: 1,
    }
    .into();

    bus.do_send(gen_esi_sss.clone());

    Ok(())
}
