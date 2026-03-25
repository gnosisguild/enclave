// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::secure::threshold::{L as L_THRESHOLD, QIS as QIS_THRESHOLD};
use crate::core::dkg::share_computation::chunk::Configs as ShareComputationChunkConfigs;
use crate::core::dkg::share_encryption::Configs as ShareEncryptionConfigs;

// Global configs for DKG secure preset
pub global N: u32 = 8192;
pub global L: u32 = 2;
pub global QIS: [Field; L] = [72057594082099201, 72057594062438401];
pub global PLAINTEXT_MODULUS: Field = 18014398509481984;
pub global Q_MOD_T: Field = 1082658244788225;
pub global Q_MOD_T_CENTERED: Field = 1082658244788225;

pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [
    [[1, 2251799822204927, 1, 0], [2, 2251799822204926, 0, 1]],
    [[1, 4503599627763711, 1, 0], [2, 4503599627763710, 0, 1]],
    [[1, 4503599631433727, 1, 0], [2, 4503599631433726, 0, 1]],
    [[1, 4503599634579455, 1, 0], [2, 4503599634579454, 0, 1]],
];

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters

pub global PK_BIT_PK: u32 = 56;

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_BIT_SHARE: u32 = 53;
pub global SHARE_COMPUTATION_SK_BIT_SECRET: u32 = 1;

/************************************
-------------------------------------
share_computation_e_sm (CIRCUIT 2b)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_E_SM_BIT_SECRET: u32 = 192;

/************************************
-------------------------------------
share_computation_chunk (CIRCUIT 2c)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_CHUNK_SIZE: u32 = 512;
pub global SHARE_COMPUTATION_N_CHUNKS: u32 = N / SHARE_COMPUTATION_CHUNK_SIZE;

pub global SHARE_COMPUTATION_CHUNKS_PER_BATCH: u32 = 4;
pub global SHARE_COMPUTATION_N_BATCHES: u32 =
    SHARE_COMPUTATION_N_CHUNKS / SHARE_COMPUTATION_CHUNKS_PER_BATCH;

pub global SHARE_COMPUTATION_CHUNK_CONFIGS: ShareComputationChunkConfigs<L_THRESHOLD> =
    ShareComputationChunkConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_computation final (C2) - combined VK hash (noir-recursive-no-zk)
Regenerate for secure preset after compiling lib with configs::secure::dkg as default.
-------------------------------------
************************************/

// Placeholder: set lib default to secure::dkg, run pnpm build:circuits (or bb write_vk per crate into
// circuits/bin/dkg/target as *.vk_recursive_hash), then compute-vk-hash (base,chunk,batch) per SK/ESM.
// TODO: update with actual hashes once we use secure parameters.
pub global SHARE_COMPUTATION_EXPECTED_VK_HASH_SK: Field =
    0x0000000000000000000000000000000000000000000000000000000000000001;
pub global SHARE_COMPUTATION_EXPECTED_VK_HASH_ESM: Field =
    0x0000000000000000000000000000000000000000000000000000000000000001;

/************************************
-------------------------------------
share_encryption_sk (CIRCUIT 3a)
share_encryption_e_sm (CIRCUIT 3b)
-------------------------------------
************************************/

pub global SHARE_ENCRYPTION_BIT_PK: u32 = 56;
pub global SHARE_ENCRYPTION_BIT_CT: u32 = 56;
pub global SHARE_ENCRYPTION_BIT_U: u32 = 1;
pub global SHARE_ENCRYPTION_BIT_E0: u32 = 5;
pub global SHARE_ENCRYPTION_BIT_E1: u32 = 5;
pub global SHARE_ENCRYPTION_BIT_MSG: u32 = 54;
pub global SHARE_ENCRYPTION_BIT_R1: u32 = 53;
pub global SHARE_ENCRYPTION_BIT_R2: u32 = 56;
pub global SHARE_ENCRYPTION_BIT_P1: u32 = 13;
pub global SHARE_ENCRYPTION_BIT_P2: u32 = 56;

pub global SHARE_ENCRYPTION_K0IS: [Field; L] = [70854796903366627, 47439047573780733];
pub global SHARE_ENCRYPTION_PK_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global SHARE_ENCRYPTION_E0_BOUND: Field = 20;
pub global SHARE_ENCRYPTION_E1_BOUND: Field = 20;
pub global SHARE_ENCRYPTION_U_BOUND: Field = 1;
pub global SHARE_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [8856849607495681, 5929880944709633];
pub global SHARE_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [8856849607495680, 5929880944709632];
pub global SHARE_ENCRYPTION_R2_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global SHARE_ENCRYPTION_P1_BOUNDS: [Field; L] = [4096, 4096];
pub global SHARE_ENCRYPTION_P2_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global SHARE_ENCRYPTION_MSG_BOUND: Field = 18014398509481983;

pub global SHARE_ENCRYPTION_CONFIGS: ShareEncryptionConfigs<L> = ShareEncryptionConfigs::new(
    PLAINTEXT_MODULUS,
    Q_MOD_T,
    QIS,
    SHARE_ENCRYPTION_K0IS,
    SHARE_ENCRYPTION_PK_BOUNDS,
    SHARE_ENCRYPTION_E0_BOUND,
    SHARE_ENCRYPTION_E1_BOUND,
    SHARE_ENCRYPTION_U_BOUND,
    SHARE_ENCRYPTION_R1_LOW_BOUNDS,
    SHARE_ENCRYPTION_R1_UP_BOUNDS,
    SHARE_ENCRYPTION_R2_BOUNDS,
    SHARE_ENCRYPTION_P1_BOUNDS,
    SHARE_ENCRYPTION_P2_BOUNDS,
    SHARE_ENCRYPTION_MSG_BOUND,
);

/************************************
-------------------------------------
share_decryption_sk (CIRCUIT 4a - BFV DECRYPTION SK)
share_decryption_e_sm (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

pub global SHARE_DECRYPTION_BIT_MSG: u32 = 55;
