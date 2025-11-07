// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use e3_events::ThresholdShare;
use e3_trbfv::{
    calculate_decryption_key::{
        calculate_decryption_key, CalculateDecryptionKeyRequest, CalculateDecryptionKeyResponse,
    },
    gen_esi_sss::{gen_esi_sss, GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::{
        gen_pk_share_and_sk_sss, GenPkShareAndSkSssRequest, GenPkShareAndSkSssResponse,
    },
    shares::{EncryptableVec, PvwEncrypted, PvwEncryptedVecExt, ShamirShare, SharedSecret},
    TrBFVConfig,
};
use e3_utils::{ArcBytes, SharedRng};
use fhe::{
    bfv::{BfvParameters, PublicKey},
    mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
};
use fhe_traits::Serialize;

// The following functions are designed to aid testing our usecases

pub fn generate_shares_hash_map(
    trbfv_config: &TrBFVConfig,
    esi_per_ct: u64,
    error_size: &ArcBytes,
    crp: &CommonRandomPoly,
    rng: &SharedRng,
    cipher: &Cipher,
) -> Result<HashMap<u64, ThresholdShare>> {
    let threshold_n = trbfv_config.num_parties();

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
                crp: ArcBytes::from_bytes(&crp.to_bytes()),
            },
        )?;

        // Simulate actor boundry and SharesGenerated
        let sk_sss = PvwEncrypted::new(sk_sss.decrypt(&cipher)?)?;
        let esi_sss: Vec<PvwEncrypted<SharedSecret>> = esi_sss
            .into_iter()
            .map(|s| PvwEncrypted::new(s.decrypt(&cipher)?))
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
    Ok(shares_hash_map)
}

pub fn get_public_key(
    shares_hash_map: &HashMap<u64, ThresholdShare>,
    params: Arc<BfvParameters>,
    crp: &CommonRandomPoly,
) -> Result<PublicKey> {
    Ok(shares_hash_map
        .clone()
        .into_iter()
        .map(|(_, v)| v.pk_share)
        .map(|k| {
            PublicKeyShare::deserialize(&k, &params, crp.clone())
                .context("Could not deserialize public key")
        })
        .collect::<Result<Vec<PublicKeyShare>>>()?
        .into_iter()
        .aggregate()?)
}

pub fn get_decryption_keys(
    shares: Vec<ThresholdShare>,
    cipher: &Cipher,
    trbfv_config: &TrBFVConfig,
) -> Result<HashMap<usize, (Vec<SensitiveBytes>, SensitiveBytes)>> {
    let threshold_n = trbfv_config.num_parties();
    let received_sss = shares
        .iter()
        .map(|ts| ts.sk_sss.clone().pvw_decrypt())
        .collect::<Result<Vec<SharedSecret>>>()?;

    let received_esi_sss: Vec<Vec<SharedSecret>> = shares
        .iter()
        .map(|ts| ts.esi_sss.clone().to_vec_decrypted())
        .collect::<Result<Vec<_>>>()?;

    // Individualize based on node
    let mut decryption_keys = HashMap::new();
    for party_id in 0..threshold_n as usize {
        let sk_sss_collected = received_sss
            .clone()
            .into_iter()
            .map(|sss| sss.extract_party_share(party_id))
            .collect::<Result<Vec<_>>>()?;

        let esi_sss_collected: Vec<Vec<ShamirShare>> = received_esi_sss
            .clone()
            .into_iter()
            .map(|s| {
                s.into_iter()
                    .map(|ss| ss.extract_party_share(party_id))
                    .collect()
            })
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
    Ok(decryption_keys)
}
