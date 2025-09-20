use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use e3_bfv_helpers::{build_bfv_params_arc, encode_bfv_params};
use e3_crypto::Cipher;
use e3_events::{DecryptionshareCreated, ThresholdShare};
use e3_fhe::create_crp;
use e3_test_helpers::{create_seed_from_u64, create_shared_rng_from_u64};
use e3_trbfv::{
    calculate_decryption_key::{
        calculate_decryption_key, CalculateDecryptionKeyRequest, CalculateDecryptionKeyResponse,
    },
    calculate_decryption_share::{calculate_decryption_share, CalculateDecryptionShareRequest},
    gen_esi_sss::{gen_esi_sss, GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::{
        gen_pk_share_and_sk_sss, GenPkShareAndSkSssRequest, GenPkShareAndSkSssResponse,
    },
    helpers::calculate_error_size,
    shares::{PvwShareSetCollection, ShareSetCollection},
    TrBFVConfig,
};
use e3_utils::ArcBytes;
use fhe_traits::Serialize;
use num_bigint::BigUint;

// TODO: Write a test of the trbfv share swapping algorhythm without the use of any events
#[tokio::test]
async fn test_trbfv_isolation() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let subscriber = fmt()
        .with_env_filter(EnvFilter::new("info"))
        .with_test_writer()
        .finish();

    let _guard = tracing::subscriber::set_default(subscriber);
    let rng = create_shared_rng_from_u64(42);

    let (degree, plaintext_modulus, moduli) = (
        8192usize,
        16384u64,
        &[
            0x1FFFFFFEA0001u64, // 562949951979521
            0x1FFFFFFE88001u64, // 562949951881217
            0x1FFFFFFE48001u64, // 562949951619073
            0xfffffebc001u64,   //
        ] as &[u64],
    );

    let params_raw = build_bfv_params_arc(degree, plaintext_modulus, moduli);
    let params = ArcBytes::from_bytes(encode_bfv_params(&params_raw.clone()));

    let crp = create_crp(params_raw.clone(), rng.clone());
    let cipher = Arc::new(Cipher::from_password("I am the music man.").await?);
    let seed = create_seed_from_u64(123);
    let error_size = ArcBytes::from_bytes(BigUint::to_bytes_be(&calculate_error_size(
        params_raw.clone(),
        5,
        3,
    )?));

    // let e3_id = E3id::new("0", 1);

    let threshold_m = 2;
    let threshold_n = 5;
    let esi_per_ct = 3;

    let trbfv_config = TrBFVConfig::new(params, threshold_n, threshold_m);
    let crp = ArcBytes::from_bytes(crp.to_bytes());
    let mut shares_hash_map = HashMap::new();

    for party_id in 0u64..threshold_n {
        let GenEsiSssResponse { esi_sss } = gen_esi_sss(
            &rng,
            &cipher,
            GenEsiSssRequest {
                esi_per_ct,
                error_size: error_size.clone(),
                trbfv_config: trbfv_config.clone(),
            },
        )?;

        let GenPkShareAndSkSssResponse { sk_sss, pk_share } = gen_pk_share_and_sk_sss(
            &rng,
            &cipher,
            GenPkShareAndSkSssRequest {
                trbfv_config: trbfv_config.clone(),
                crp: crp.clone(),
            },
        )?;

        // Simulate actor boundry and SharesGenerated
        let sk_sss: PvwShareSetCollection = sk_sss.decrypt(&cipher)?.try_into()?;
        let esi_sss: Vec<PvwShareSetCollection> = esi_sss
            .into_iter()
            .map(|s| s.decrypt(&cipher)?.try_into())
            .collect::<Result<_>>()?;

        shares_hash_map.insert(
            party_id,
            ThresholdShare {
                party_id,
                esi_sss,
                sk_sss,
                pk_share,
            },
        );
    }

    // All shares_hash_map should receive the same encrypted list from all other shares_hash_map
    let shares = to_ordered_vec(shares_hash_map);
    let received_sss: Vec<ShareSetCollection> = shares
        .iter()
        .map(|ts| ts.sk_sss.clone().try_into())
        .collect::<Result<_>>()?;

    let received_esi_sss: Vec<Vec<ShareSetCollection>> = shares
        .into_iter()
        .map(|ts| {
            ts.esi_sss
                .clone()
                .into_iter()
                .map(|s| s.try_into())
                .collect()
        })
        .collect::<Result<_>>()?;

    // Individualize based on node
    let mut decryption_keys = HashMap::new();
    for party_id in 0..threshold_n as usize {
        let sk_sss_collected = ShareSetCollection::from_received(received_sss.clone(), party_id)?;

        let esi_sss_collected: Vec<ShareSetCollection> = received_esi_sss
            .clone()
            .into_iter()
            .map(|s| ShareSetCollection::from_received(s, party_id))
            .collect::<Result<_>>()?;

        let CalculateDecryptionKeyResponse {
            es_poly_sum,
            sk_poly_sum,
        } = calculate_decryption_key(
            &cipher,
            CalculateDecryptionKeyRequest {
                trbfv_config: trbfv_config.clone(),
                esi_sss_collected: esi_sss_collected
                    .into_iter()
                    .map(|s| s.encrypt(&cipher))
                    .collect::<Result<_>>()?,
                sk_sss_collected: sk_sss_collected.encrypt(&cipher)?,
            },
        )?;
        decryption_keys.insert(party_id, (es_poly_sum, sk_poly_sum));
    }

    let mut decryption_shares = HashMap::new();
    for party_id in 0..threshold_m as usize {
        let (_, sk_poly_sum) = decryption_keys.get(&party_id).unwrap();
        calculate_decryption_share(
            &cipher,
            CalculateDecryptionShareRequest {
                sk_poly_sum: sk_poly_sum.clone(),
                trbfv_config: trbfv_config.clone(),
                ciphertexts,
            },
        )?;
    }
    Ok(())
}

// TODO: move to utils and use in AllThresholdSharesCollected
fn to_ordered_vec<K, T>(source: HashMap<K, T>) -> Vec<T>
where
    K: Ord + Copy,
{
    // extract a vector
    let mut pairs: Vec<_> = source.into_iter().collect();

    // Ensure keys are sorted
    pairs.sort_by_key(|&(key, _)| key);

    // Extract to Vec of ThresholdShares in order
    pairs.into_iter().map(|(_, value)| value).collect()
}
