// SPDX-License-Identifier: LGPL-3.0-only

//! Correlated non-prefix DKG material. Production preparation stays in zk-witness.

use e3_committee::CiphernodesCommitteeSize;
use e3_fhe_params::{create_deterministic_crp_from_default_seed, BfvPreset};
use e3_polynomial::CrtPolynomial;
use e3_polynomial_fhe::FromFhePolynomial;
use e3_zk_config::DkgInputType;
use e3_zk_witness::{dkg, threshold};
use fhe::{
    bfv::{Ciphertext, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, PublicKeyShare},
    trbfv::ShareManager,
};
use fhe_traits::{FheEncoder, FheEncrypter};
use ndarray::Array2;
use num_bigint::BigInt;
use rand::RngCore;

pub struct CorrelatedDkg {
    pub preset: BfvPreset,
    pub committee: CiphernodesCommitteeSize,
    pub ids: Vec<u64>,
    pub c0: Vec<PublicKey>,
    pub c1: Vec<threshold::pk_generation::PkGenerationCircuitData>,
    pub c2: Vec<dkg::share_computation::ShareComputationCircuitData>,
    pub c3: Vec<Vec<(u32, dkg::share_encryption::ShareEncryptionCircuitData)>>,
    pub c4: Vec<dkg::share_decryption::ShareDecryptionCircuitData>,
    pub c5: threshold::pk_aggregation::PkAggregationCircuitData,
    shares: Vec<Vec<Array2<u64>>>,
    keys: Vec<Vec<Vec<u8>>>,
}

pub fn generate(preset: BfvPreset, committee: CiphernodesCommitteeSize) -> CorrelatedDkg {
    let size = committee.values();
    let (params, transport) = preset.build_pair().unwrap();
    let ids: Vec<u64> = (0..size.h - 1)
        .chain(std::iter::once(size.n - 1))
        .map(|id| id as u64)
        .collect();
    let mut rng = rand::rng();
    let transport_sk: Vec<_> = (0..size.n)
        .map(|_| SecretKey::random(&transport, &mut rng))
        .collect();
    let transport_pk: Vec<_> = transport_sk
        .iter()
        .map(|sk| PublicKey::new(sk, &mut rng))
        .collect();
    let mut manager = ShareManager::new(size.n, size.threshold, params.clone()).unwrap();
    let crp = create_deterministic_crp_from_default_seed(&params);
    let mut c1 = Vec::new();
    let mut c2 = Vec::new();
    let mut shares = Vec::new();
    let mut keys = Vec::new();
    let mut contributions = Vec::new();
    for _ in &ids {
        let sk = SecretKey::random(&params, &mut rng);
        let (pk, intermediates) =
            PublicKeyShare::new_with_intermediates(&sk, crp.clone(), &mut rng).unwrap();
        let raw_sk = manager.coeffs_to_poly_level0(sk.coeffs.as_ref()).unwrap();
        let sss = manager
            .generate_secret_shares_from_poly(raw_sk, &mut rng)
            .unwrap();
        let mut secret = CrtPolynomial::from_mod_q_polynomial(
            &sk.coeffs
                .iter()
                .map(|c| BigInt::from(*c))
                .collect::<Vec<_>>(),
            params.moduli(),
        );
        secret.center(params.moduli()).unwrap();
        let row: Vec<Vec<u8>> = (0..size.n)
            .map(|_| {
                let mut key = e3_fhe_params::zero_prf_key(preset);
                rng.fill_bytes(&mut key);
                key
            })
            .collect();
        c1.push(threshold::pk_generation::PkGenerationCircuitData {
            pk0_share: CrtPolynomial::from_fhe_polynomial(pk.p0_share()),
            eek: CrtPolynomial::from_fhe_polynomial(intermediates.error()),
            sk: CrtPolynomial::from_fhe_polynomial(intermediates.secret_key()),
        });
        c2.push(dkg::share_computation::ShareComputationCircuitData {
            prf_keys: row.clone(),
            dkg_input_type: DkgInputType::SecretKey,
            secret,
            secret_sss: sss.iter().map(|matrix| matrix.mapv(BigInt::from)).collect(),
        });
        shares.push(sss);
        keys.push(row);
        contributions.push(pk);
    }
    let public_key: PublicKey = contributions.into_iter().aggregate().unwrap();
    let mut c3 = Vec::new();
    let mut encrypted: Vec<Vec<Option<Vec<Ciphertext>>>> = Vec::new();
    for (dealer, &id) in ids.iter().enumerate() {
        let mut proofs = Vec::new();
        let mut ciphertexts = vec![None; size.n];
        for recipient in 0..size.n {
            if recipient == id as usize {
                continue;
            }
            let mut limbs = Vec::new();
            for row in 0..params.moduli().len() {
                let raw = shares[dealer][row].row(recipient).to_vec();
                let message = if row == 0 {
                    e3_fhe_params::pack_prf_key(
                        preset,
                        &raw,
                        &keys[dealer][recipient],
                        params.moduli()[0],
                    )
                    .unwrap()
                } else {
                    raw
                };
                let plaintext =
                    Plaintext::try_encode(&message, Encoding::poly(), &transport).unwrap();
                let (ciphertext, noise) = transport_pk[recipient]
                    .try_encrypt_with_intermediates(&plaintext, &mut rng)
                    .unwrap();
                proofs.push((
                    (recipient * params.moduli().len() + row) as u32,
                    dkg::share_encryption::ShareEncryptionCircuitData {
                        recipient_party_idx: recipient as u32,
                        row_idx: row as u32,
                        prf_key: keys[dealer][recipient].clone(),
                        plaintext,
                        ciphertext: ciphertext.clone(),
                        public_key: transport_pk[recipient].clone(),
                        u_rns: noise.randomness().clone(),
                        e0_rns: noise.error_0().clone(),
                        e1_rns: noise.error_1().clone(),
                    },
                ));
                limbs.push(ciphertext);
            }
            ciphertexts[recipient] = Some(limbs);
        }
        c3.push(proofs);
        encrypted.push(ciphertexts);
    }
    let c4 = ids
        .iter()
        .enumerate()
        .map(
            |(own, &recipient)| dkg::share_decryption::ShareDecryptionCircuitData {
                recipient_party_idx: recipient as u32,
                prf_keys: keys
                    .iter()
                    .map(|row| row[recipient as usize].clone())
                    .collect(),
                secret_key: transport_sk[recipient as usize].clone(),
                honest_ciphertexts: encrypted
                    .iter()
                    .map(|dealer| dealer[recipient as usize].clone())
                    .collect(),
                own_plaintext_share: shares[own]
                    .iter()
                    .map(|row| row.row(recipient as usize).to_vec())
                    .collect(),
            },
        )
        .collect();
    let c0 = ids
        .iter()
        .map(|id| transport_pk[*id as usize].clone())
        .collect();
    let c5 = threshold::pk_aggregation::PkAggregationCircuitData {
        public_key,
        pk0_shares: c1.iter().map(|data| data.pk0_share.clone()).collect(),
    };
    CorrelatedDkg {
        preset,
        committee,
        ids,
        c0,
        c1,
        c2,
        c3,
        c4,
        c5,
        shares,
        keys,
    }
}

impl CorrelatedDkg {
    pub fn decryption(
        &self,
    ) -> (
        Vec<threshold::share_decryption::ShareDecryptionCircuitData>,
        threshold::decrypted_shares_aggregation::DecryptedSharesAggregationCircuitData,
    ) {
        use e3_zk_witness::threshold::{
            decrypted_shares_aggregation::DecryptedSharesAggregationCircuitData,
            share_decryption::ShareDecryptionCircuitData,
        };
        use fhe::trbfv::SynchronizedDecryptor;
        let size = self.committee.values();
        // Retain the key's exact parameter owner through encryption and reconstruction.
        let params = self.c5.public_key.params.clone();
        let mut rng = rand::rng();
        let manager = ShareManager::new(size.n, size.threshold, params.clone()).unwrap();
        let ciphertext = self
            .c5
            .public_key
            .try_encrypt(
                &Plaintext::try_encode(&[5u64], Encoding::poly(), &params).unwrap(),
                &mut rng,
            )
            .unwrap();
        let ids: Vec<u32> = self
            .ids
            .iter()
            .take(size.threshold + 1)
            .map(|id| *id as u32 + 1)
            .collect();
        let mut partials = Vec::new();
        let mut witnesses = Vec::new();
        for &id in self.ids.iter().take(size.threshold + 1) {
            let contributions: Vec<Array2<u64>> = self
                .shares
                .iter()
                .map(|dealer| {
                    let mut rows = Array2::zeros((params.moduli().len(), params.degree()));
                    for row in 0..rows.nrows() {
                        rows.row_mut(row).assign(&dealer[row].row(id as usize));
                    }
                    rows
                })
                .collect();
            let sk = manager.aggregate_collected_shares(&contributions).unwrap();
            let dealer = self.ids.iter().position(|party| *party == id).unwrap();
            let outgoing = self.keys[dealer].clone();
            let mut incoming = vec![e3_fhe_params::zero_prf_key(self.preset); size.n];
            for (row, &sender) in self.ids.iter().enumerate() {
                incoming[sender as usize] = self.keys[row][id as usize].clone();
            }
            let inputs = e3_fhe_params::fresh_smudging_inputs(self.preset, size.n).unwrap();
            let coefficients = e3_fhe_params::generate_smudging_error(
                params.clone(),
                size.n,
                inputs.num_ciphertexts,
                0,
                inputs.lambda,
                &mut rng,
            )
            .unwrap();
            let noise = e3_fhe_params::smudging_noise_from_signed_coefficients(
                params.clone(),
                size.n,
                inputs.num_ciphertexts,
                0,
                inputs.lambda,
                &coefficients,
            )
            .unwrap();
            let keys =
                e3_zk_witness::prf::party_keys(id as u32, size.n, &outgoing, &incoming).unwrap();
            let share = SynchronizedDecryptor::new(size.n, size.threshold, params.clone())
                .unwrap()
                .decryption_share(
                    &ciphertext,
                    &sk.clone().into_ntt(),
                    id as usize + 1,
                    &ids.iter().map(|id| *id as usize).collect::<Vec<_>>(),
                    noise,
                    &keys,
                )
                .unwrap();
            witnesses.push(ShareDecryptionCircuitData {
                ciphertext: ciphertext.clone(),
                public_key: self.c5.public_key.clone(),
                s: CrtPolynomial::from_fhe_polynomial(&sk),
                e: CrtPolynomial::from_mod_q_polynomial(&coefficients, params.moduli()),
                e_signed: coefficients,
                d_share: CrtPolynomial::from_fhe_polynomial(&share.clone().into_parts().0),
                domain_hi: 1,
                domain_lo: 2,
                committee: size.clone(),
                party_idx: id as u32,
                decryptors: ids.clone(),
                outgoing_prf_keys: outgoing,
                incoming_prf_keys: incoming,
            });
            partials.push(share);
        }
        let opened = SynchronizedDecryptor::new(size.n, size.threshold, params.clone())
            .unwrap()
            .decrypt_from_shares(&partials, &ciphertext)
            .unwrap();
        use fhe_traits::FheDecoder;
        let message_vec = Vec::<u64>::try_decode(&opened, Encoding::poly()).unwrap();
        assert_eq!(message_vec[0], 5);
        (
            witnesses,
            DecryptedSharesAggregationCircuitData {
                ct0: ciphertext[0].clone().into_power_basis(),
                d_share_polys: partials
                    .into_iter()
                    .map(|share| share.into_parts().0)
                    .collect(),
                reconstructing_parties: ids.iter().map(|id| *id as usize).collect(),
                message_vec,
            },
        )
    }
}
