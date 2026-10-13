// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::hash::Hash;
use std::ops::Deref;

use crate::helpers::try_poly_from_sensitive_bytes;
/// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
use crate::TrBFVConfig;
use anyhow::*;
use e3_crypto::{Cipher, SensitiveBytes};
use e3_fhe_params::{
    fresh_smudging_inputs, sample_fresh_smudging_error, smudging_noise_from_signed_coefficients,
    BfvPreset,
};
use e3_utils::utility_types::ArcBytes;
use e3_zk_helpers::circuits::threshold::share_decryption::encode_signed_coefficients;
use fhe::bfv::Ciphertext;
use fhe::trbfv::{PartyPrfKeyTransport, PartyPrfKeys, SynchronizedDecryptor};
use fhe_math::rq::{Poly, PowerBasis};
use fhe_traits::DeserializeParametrized;
use fhe_traits::Serialize;
use tracing::info;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionShareRequest {
    /// Name to identify the job.
    pub name: String,
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<ArcBytes>,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// Zero-based party index.
    pub party_idx: u32,
    /// Strictly increasing 1-based decryptor ids.
    pub decryptors: Vec<u32>,
    /// Outgoing PRF keys indexed by recipient. Encrypted at rest.
    pub outgoing_prf_keys: Vec<SensitiveBytes>,
    /// Incoming PRF keys indexed by sender. Encrypted at rest.
    pub incoming_prf_keys: Vec<SensitiveBytes>,
}

struct InnerRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<Ciphertext>,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: Poly<PowerBasis>,
}

impl TryFrom<(&Cipher, CalculateDecryptionShareRequest)> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(
        value: (&Cipher, CalculateDecryptionShareRequest),
    ) -> std::result::Result<InnerRequest, Self::Error> {
        let trbfv_config = value.1.trbfv_config.clone();
        let ciphertexts = value
            .1
            .ciphertexts
            .into_iter()
            .map(|ciphertext| {
                Ciphertext::from_bytes(&ciphertext, &trbfv_config.params())
                    .context("Could not parse ciphertext")
            })
            .collect::<Result<Vec<Ciphertext>>>()?;

        let sk_poly_sum =
            try_poly_from_sensitive_bytes(value.1.sk_poly_sum, trbfv_config.params(), value.0)?;

        Ok(InnerRequest {
            sk_poly_sum,
            ciphertexts,
            trbfv_config,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionShareResponse {
    /// The decryption share for the given ciphertext
    pub d_share_poly: Vec<ArcBytes>,
    /// Fresh noise used in each share. The C6 witness must reuse these bytes.
    pub e_fresh: Vec<SensitiveBytes>,
    /// Signed coefficients of each fresh-noise polynomial, low degree first.
    pub e_signed: Vec<SensitiveBytes>,
    /// Digest of the sorted decryptor ids and the ciphertext bytes this share opens.
    pub context_digest: [u8; 32],
}

struct InnerResponse {
    pub d_share_poly: Vec<Poly<PowerBasis>>,
    pub e_fresh: Vec<Poly<PowerBasis>>,
    pub e_signed: Vec<Vec<num_bigint::BigInt>>,
    pub context_digest: [u8; 32],
}

impl InnerResponse {
    fn encrypt(self, cipher: &Cipher) -> Result<CalculateDecryptionShareResponse> {
        Ok(CalculateDecryptionShareResponse {
            d_share_poly: self
                .d_share_poly
                .into_iter()
                .map(|p| ArcBytes::from_bytes(&p.to_bytes()))
                .collect(),
            e_fresh: self
                .e_fresh
                .into_iter()
                .map(|p| SensitiveBytes::new(p.to_bytes(), cipher))
                .collect::<Result<Vec<_>>>()?,
            e_signed: self
                .e_signed
                .into_iter()
                .map(|coeffs| SensitiveBytes::new(encode_signed_coefficients(&coeffs), cipher))
                .collect::<Result<Vec<_>>>()?,
            context_digest: self.context_digest,
        })
    }
}

pub fn calculate_decryption_share(
    cipher: &Cipher,
    req: CalculateDecryptionShareRequest,
) -> Result<CalculateDecryptionShareResponse> {
    info!("Calculating decryption share: `{}`...", req.name);
    let reconstruction = req.trbfv_config.threshold() as usize + 1;
    if req.decryptors.len() != reconstruction {
        bail!(
            "decryptor count {} must equal threshold + 1 ({reconstruction})",
            req.decryptors.len()
        );
    }
    let party_idx = req.party_idx as usize;
    let decryptors = req.decryptors.clone();
    let outgoing_prf_keys = open_prf_keys(cipher, &req.outgoing_prf_keys)?;
    let incoming_prf_keys = open_prf_keys(cipher, &req.incoming_prf_keys)?;
    let mut digest_ids = decryptors.clone();
    digest_ids.sort_unstable();
    let context_digest = crate::calculate_threshold_decryption::decryption_context_digest(
        &digest_ids,
        &req.ciphertexts,
    );
    let req: InnerRequest = (cipher, req).try_into()?;

    let params = req.trbfv_config.params();
    let preset =
        BfvPreset::from_threshold_parameters(params.degree(), params.plaintext(), params.moduli())
            .context("threshold parameters do not match a supported preset")?;
    let noise = fresh_smudging_inputs(preset, req.trbfv_config.num_parties() as usize)
        .map_err(|error| anyhow!(error))?;
    let sk_ntt = req.sk_poly_sum.clone().into_ntt();
    let mut e_fresh = Vec::with_capacity(req.ciphertexts.len());
    let mut e_signed = Vec::with_capacity(req.ciphertexts.len());
    for _ in 0..req.ciphertexts.len() {
        let coeffs = sample_fresh_smudging_error(
            params.clone(),
            noise.num_parties,
            noise.num_ciphertexts,
            noise.mult_depth,
            noise.lambda,
        )?;
        let polynomial = Poly::<PowerBasis>::from_bigints(&coeffs, params.context_at_level(0)?)?;
        e_signed.push(coeffs);
        e_fresh.push(polynomial.deref().clone());
    }

    let committee_size = req.trbfv_config.num_parties() as usize;
    let party_id = party_idx + 1;
    let decryptor = SynchronizedDecryptor::new(
        committee_size,
        req.trbfv_config.threshold() as usize,
        params.clone(),
    )?;
    let decryptor_ids: Vec<usize> = decryptors.iter().map(|id| *id as usize).collect();
    let prf_keys = party_prf_keys(
        party_id,
        committee_size,
        &outgoing_prf_keys,
        &incoming_prf_keys,
    )?;
    info!("Calculating d_share_poly...");
    let mut d_share_poly = Vec::with_capacity(req.ciphertexts.len());
    for (index, ciphertext) in req.ciphertexts.iter().enumerate() {
        info!("Create decryption share for ct index {}...", index);
        let coeffs = e_signed
            .get(index)
            .context("fresh noise is missing for this ciphertext")?;
        let noise = smudging_noise_from_signed_coefficients(
            params.clone(),
            noise.num_parties,
            noise.num_ciphertexts,
            noise.mult_depth,
            noise.lambda,
            coeffs,
        )?;
        let share = decryptor.decryption_share(
            ciphertext,
            &sk_ntt,
            party_id,
            &decryptor_ids,
            noise,
            &prf_keys,
        )?;
        let (poly, _, _) = share.into_parts();
        d_share_poly.push(poly);
    }
    info!("Returning successful result...");

    InnerResponse {
        d_share_poly,
        e_fresh,
        e_signed,
        context_digest,
    }
    .encrypt(cipher)
}

fn open_prf_keys(cipher: &Cipher, keys: &[SensitiveBytes]) -> Result<Vec<Vec<u8>>> {
    keys.iter()
        .map(|key| {
            key.access_raw(cipher)
                .map_err(|error| anyhow!("cannot decrypt a PRF key: {error}"))
        })
        .collect()
}

fn party_prf_keys(
    party_id: usize,
    committee_size: usize,
    outgoing_keys: &[Vec<u8>],
    incoming_keys: &[Vec<u8>],
) -> Result<PartyPrfKeys> {
    let keys_i_j = pad_prf_keys(outgoing_keys, committee_size)?;
    let keys_j_i = pad_prf_keys(incoming_keys, committee_size)?;
    let transport = PartyPrfKeyTransport::new(party_id, committee_size, keys_i_j, keys_j_i)?;
    Ok(PartyPrfKeys::from_transport(transport)?)
}

fn pad_prf_keys(keys: &[Vec<u8>], committee_size: usize) -> Result<Vec<[u8; 32]>> {
    if keys.len() != committee_size {
        bail!(
            "PRF key count {} must equal the committee size {committee_size}",
            keys.len()
        );
    }
    keys.iter().map(|key| pad_prf_key(key)).collect()
}

fn pad_prf_key(key: &[u8]) -> Result<[u8; 32]> {
    if key.len() > 32 {
        bail!("PRF key is longer than 32 bytes");
    }
    let mut padded = [0u8; 32];
    padded[..key.len()].copy_from_slice(key);
    Ok(padded)
}
