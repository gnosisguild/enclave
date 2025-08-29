// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Actor;
use anyhow::{bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_events::{wait_for_event, EnclaveEvent, EventBus, EventBusConfig};
use e3_multithread::Multithread;
use e3_sdk::bfv_helpers::encode_bfv_params;
use e3_test_helpers::get_common_setup;
use e3_trbfv::{TrBFVConfig, TrBFVResponse};
use fhe_rs::{
    bfv,
    trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig},
};
use fhe_traits::Serialize;
use num_bigint::BigUint;
use rand_chacha::ChaCha20Rng;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use zeroize::Zeroizing;

pub fn save_snapshot(file_name: &str, bytes: &[u8]) {
    println!("### WRITING SNAPSHOT TO `{file_name}` ###");
    fs::write(format!("tests/{file_name}"), bytes).unwrap();
}

fn serialize_z_vec_of_bytes(data: &Vec<Zeroizing<Vec<u8>>>) -> Vec<u8> {
    bincode::serialize(
        &data
            .iter()
            .map(|z| -> &Vec<u8> { z.as_ref() })
            .collect::<Vec<_>>(),
    )
    .unwrap()
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

// Act like a single party in multithread
#[derive(Clone)]
struct PartySharesResult {
    pk_share_and_sk_sss_event: EnclaveEvent,
    esi_sss_event: EnclaveEvent,
}
async fn generate_party_shares(
    rng: Arc<Mutex<ChaCha20Rng>>,
    params: Arc<Vec<u8>>,
    cipher: Arc<Cipher>,
    crp: Arc<Vec<u8>>,
    error_size: Arc<Vec<u8>>,
    num_parties: u64,
    threshold: u64,
) -> Result<PartySharesResult> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();

    // Setup multithread processor
    // TODO: Currently only testing logic not setup on multithread yet
    let _multi = Multithread::attach(&bus, rng, cipher.clone());

    /////////////////////////////////////////////
    // 1. Generate initial pk and sk sss
    /////////////////////////////////////////////

    let gen_pk_share_and_sk_sss: EnclaveEvent = e3_trbfv::gen_pk_share_and_sk_sss::Request {
        trbfv_config: TrBFVConfig::new(params.clone(), num_parties, threshold),
        crp,
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

    let pk_share_and_sk_sss_event = wait_for_response.await??;

    /////////////////////////////////////////////
    // 2. Generate smudging noise
    /////////////////////////////////////////////

    let gen_esi_sss: EnclaveEvent = e3_trbfv::gen_esi_sss::Request {
        trbfv_config: TrBFVConfig::new(params.clone(), num_parties, threshold),
        error_size,
        esi_per_ct: 1,
    }
    .into();

    let correlation_id = gen_esi_sss.correlation_id().unwrap();

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

    bus.do_send(gen_esi_sss.clone());

    let esi_sss_event = wait_for_response.await??;
    Ok(PartySharesResult {
        pk_share_and_sk_sss_event,
        esi_sss_event,
    })
}

async fn snapshot_test_events(party: PartySharesResult, cipher: &Cipher) -> Result<()> {
    let Some(TrBFVResponse::GenPkShareAndSkSss(res)) =
        party.pk_share_and_sk_sss_event.trbfv_response()
    else {
        bail!("bad response from GenPkShareAndSkSss");
    };

    // Ensure pk_share is correct
    let pk_share = res.pk_share.clone();

    // NOTE: uncomment the following to save new snapshot.
    // save_snapshot("fixtures/01_pk_share.bin", &pk_share[..]);

    // Check against snapshot
    assert_eq!(
        pk_share,
        Arc::new(include_bytes!("fixtures/01_pk_share.bin").to_vec())
    );

    // Ensure sk_sss is correct
    let sk_sss = SensitiveBytes::access_vec(res.sk_sss.clone(), &cipher)?;

    let serialized_sk_sss = serialize_z_vec_of_bytes(&sk_sss);

    // NOTE: uncomment the following to save new snapshot.
    // save_snapshot("fixtures/02_sk_sss.bin", &serialized_sk_sss);

    // Check against snapshot
    assert_eq!(
        serialized_sk_sss,
        include_bytes!("fixtures/02_sk_sss.bin").to_vec()
    );

    let Some(TrBFVResponse::GenEsiSss(res)) = party.esi_sss_event.trbfv_response() else {
        bail!("bad response from GenEsiSss");
    };

    let esi_sss = SensitiveBytes::access_vec(res.esi_sss.clone(), &cipher)?;

    let serialized_esi_sss = serialize_z_vec_of_bytes(&esi_sss);
    // NOTE: uncomment the following to save new snapshot.
    // save_snapshot("fixtures/03_esi_sss.bin", &serialized_esi_sss);

    assert_eq!(
        serialized_esi_sss,
        include_bytes!("fixtures/03_esi_sss.bin").to_vec()
    );

    Ok(())
}

#[actix::test]
async fn test_trbfv() -> Result<()> {
    // Generate basic setup params
    let (_, rng, _seed, params, crpoly, _, _) = get_common_setup(Some((
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
    let threshold = 2; // must be <= (num_parties - 1)/2
    let error_size_bigint = calculate_error_size(params, num_parties as usize, 3)?;
    let error_size = Arc::new(BigUint::to_bytes_be(&error_size_bigint));

    let mut parties: Vec<PartySharesResult> = vec![];

    // generate shamir events for party 1
    let party_0 = generate_party_shares(
        rng.clone(),
        params_bytes.clone(),
        cipher.clone(),
        crp_bytes.clone(),
        error_size.clone(),
        num_parties,
        threshold,
    )
    .await?;

    parties.push(party_0.clone());

    // snapshot test events
    snapshot_test_events(party_0.clone(), &cipher).await?;

    for _ in 1..num_parties {
        parties.push(
            generate_party_shares(
                rng.clone(),
                params_bytes.clone(),
                cipher.clone(),
                crp_bytes.clone(),
                error_size.clone(),
                num_parties,
                threshold,
            )
            .await?,
        );
    }

    Ok(())
}
