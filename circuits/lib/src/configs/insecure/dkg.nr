// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::insecure::threshold::{
    L as L_THRESHOLD, QIS as QIS_THRESHOLD,
    THRESHOLD_SHARE_DECRYPTION_BIT_SK as SHARE_DECRYPTION_BIT_AGG,
};
use crate::core::dkg::share_computation::chunk::Configs as ShareComputationChunkConfigs;
use crate::core::dkg::share_encryption::Configs as ShareEncryptionConfigs;

// Global configs for DKG insecure preset
pub global N: u32 = 512;
pub global L: u32 = 1;
pub global QIS: [Field; L] = [2251799813554177];
pub global PLAINTEXT_MODULUS: Field = 68719403009;
pub global Q_MOD_T: Field = 2415755265;
pub global Q_MOD_T_CENTERED: Field = 2415755265;

pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [
    [[1, 68719403007, 1, 0], [2, 68719403006, 0, 1]],
    [[1, 68719230975, 1, 0], [2, 68719230974, 0, 1]],
];

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters
pub global PK_BIT_PK: u32 = 50;

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_BIT_SHARE: u32 = 36;
pub global SHARE_COMPUTATION_SK_BIT_SECRET: u32 = 1;

/************************************
-------------------------------------
share_computation_e_sm (CIRCUIT 2b)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_E_SM_BIT_SECRET: u32 = 24;

/************************************
-------------------------------------
share_computation_chunk (CIRCUIT 2c)
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_CHUNK_SIZE: u32 = 512;
pub global SHARE_COMPUTATION_N_CHUNKS: u32 = N / SHARE_COMPUTATION_CHUNK_SIZE;

pub global SHARE_COMPUTATION_CHUNKS_PER_BATCH: u32 = 1;
pub global SHARE_COMPUTATION_N_BATCHES: u32 =
    SHARE_COMPUTATION_N_CHUNKS / SHARE_COMPUTATION_CHUNKS_PER_BATCH;

pub global SHARE_COMPUTATION_CHUNK_CONFIGS: ShareComputationChunkConfigs<L_THRESHOLD> =
    ShareComputationChunkConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_computation final (C2) - combined VK hash (noir-recursive-no-zk)
Regenerate: zk_cli share_computation codegen with ENCLAVE_CIRCUITS_ROOT + bin/dkg/target/\*.vk_recursive_hash.
-------------------------------------
************************************/

pub global SHARE_COMPUTATION_EXPECTED_VK_HASH_SK: Field =
    0x166f69fac410d14aef0357cc8544aaa9e4466d50e2629c14c3e666b3a08fb25f;
pub global SHARE_COMPUTATION_EXPECTED_VK_HASH_ESM: Field =
    0x12d9d8102a32f8750061596b14144ac6209e039421e684e595082c92e31ceb71;

/************************************
-------------------------------------
share_encryption_sk (CIRCUIT 3a)
share_encryption_e_sm (CIRCUIT 3b)
-------------------------------------
************************************/

pub global SHARE_ENCRYPTION_BIT_PK: u32 = 50;
pub global SHARE_ENCRYPTION_BIT_CT: u32 = 50;
pub global SHARE_ENCRYPTION_BIT_U: u32 = 1;
pub global SHARE_ENCRYPTION_BIT_E0: u32 = 3;
pub global SHARE_ENCRYPTION_BIT_E1: u32 = 3;
pub global SHARE_ENCRYPTION_BIT_MSG: u32 = 36;
pub global SHARE_ENCRYPTION_BIT_R1: u32 = 35;
pub global SHARE_ENCRYPTION_BIT_R2: u32 = 50;
pub global SHARE_ENCRYPTION_BIT_P1: u32 = 9;
pub global SHARE_ENCRYPTION_BIT_P2: u32 = 50;

pub global SHARE_ENCRYPTION_K0IS: [Field; L] = [1284838520228573];
pub global SHARE_ENCRYPTION_PK_BOUNDS: [Field; L] = [1125899906777088];
pub global SHARE_ENCRYPTION_E0_BOUND: Field = 6;
pub global SHARE_ENCRYPTION_E1_BOUND: Field = 6;
pub global SHARE_ENCRYPTION_U_BOUND: Field = 1;
pub global SHARE_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [19605059183];
pub global SHARE_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [19605059183];
pub global SHARE_ENCRYPTION_R2_BOUNDS: [Field; L] = [1125899906777088];
pub global SHARE_ENCRYPTION_P1_BOUNDS: [Field; L] = [256];
pub global SHARE_ENCRYPTION_P2_BOUNDS: [Field; L] = [1125899906777088];
pub global SHARE_ENCRYPTION_MSG_BOUND: Field = 68719403008;

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

pub global SHARE_DECRYPTION_BIT_MSG: u32 = 36;
// SHARE_DECRYPTION_BIT_AGG: see `pub use` of `THRESHOLD_SHARE_DECRYPTION_BIT_SK` (C6 `BIT_SK`).
