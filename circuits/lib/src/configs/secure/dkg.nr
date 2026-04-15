// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::secure::threshold::{
    L as L_THRESHOLD, QIS as QIS_THRESHOLD,
    THRESHOLD_SHARE_DECRYPTION_BIT_SK as SHARE_DECRYPTION_BIT_AGG,
};
use crate::core::dkg::share_computation::Configs as ShareComputationConfigs;
use crate::core::dkg::share_encryption::Configs as ShareEncryptionConfigs;

// Global configs for DKG secure preset
pub global N: u32 = 8192;
pub global L: u32 = 2;
pub global QIS: [Field; L] = [2305843009242923009, 2305843009240301569];
pub global PLAINTEXT_MODULUS: Field = 1152921504606846976;
pub global Q_MOD_T: Field = 777715553927169;
pub global Q_MOD_T_CENTERED: Field = 777715553927169;

pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [
    [[1, 288230376173076479, 1, 0], [2, 288230376173076478, 0, 1]],
    [[1, 288230376167047167, 1, 0], [2, 288230376167047166, 0, 1]],
    [[1, 288230376161279999, 1, 0], [2, 288230376161279998, 0, 1]],
];

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters

pub global PK_BIT_PK: u32 = 61;

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

// share_computation_sk - bit parameters
pub global SHARE_COMPUTATION_BIT_SHARE: u32 = 59;
pub global SHARE_COMPUTATION_SK_BIT_SECRET: u32 = 1;

// share_computation_sk - configs
pub global SHARE_COMPUTATION_SK_CONFIGS: ShareComputationConfigs<L_THRESHOLD> =
    ShareComputationConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_computation_e_sm (CIRCUIT 2b)
-------------------------------------
************************************/

// share_computation_e_sm - bit parameters
pub global SHARE_COMPUTATION_E_SM_BIT_SECRET: u32 = 152;

// verify_shares - configs
pub global SHARE_COMPUTATION_E_SM_CONFIGS: ShareComputationConfigs<L_THRESHOLD> =
    ShareComputationConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_encryption_sk (CIRCUIT 3a)
share_encryption_e_sm (CIRCUIT 3b)
-------------------------------------
************************************/

pub global SHARE_ENCRYPTION_BIT_PK: u32 = 61;
pub global SHARE_ENCRYPTION_BIT_CT: u32 = 61;
pub global SHARE_ENCRYPTION_BIT_U: u32 = 1;
pub global SHARE_ENCRYPTION_BIT_E0: u32 = 5;
pub global SHARE_ENCRYPTION_BIT_E1: u32 = 5;
pub global SHARE_ENCRYPTION_BIT_MSG: u32 = 60;
pub global SHARE_ENCRYPTION_BIT_R1: u32 = 59;
pub global SHARE_ENCRYPTION_BIT_R2: u32 = 61;
pub global SHARE_ENCRYPTION_BIT_P1: u32 = 13;
pub global SHARE_ENCRYPTION_BIT_P2: u32 = 61;

pub global SHARE_ENCRYPTION_K0IS: [Field; L] = [1591479343852802974, 636423477871775445];
pub global SHARE_ENCRYPTION_PK_BOUNDS: [Field; L] = [1152921504621461504, 1152921504620150784];
pub global SHARE_ENCRYPTION_E0_BOUND: Field = 20;
pub global SHARE_ENCRYPTION_E1_BOUND: Field = 20;
pub global SHARE_ENCRYPTION_U_BOUND: Field = 1;
pub global SHARE_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [397869835958161409, 159105869466112001];
pub global SHARE_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [397869835958161408, 159105869466112001];
pub global SHARE_ENCRYPTION_R2_BOUNDS: [Field; L] = [1152921504621461504, 1152921504620150784];
pub global SHARE_ENCRYPTION_P1_BOUNDS: [Field; L] = [4096, 4096];
pub global SHARE_ENCRYPTION_P2_BOUNDS: [Field; L] = [1152921504621461504, 1152921504620150784];
pub global SHARE_ENCRYPTION_MSG_BOUND: Field = 1152921504606846975;

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

pub global SHARE_DECRYPTION_BIT_MSG: u32 = 60;
// SHARE_DECRYPTION_BIT_AGG: see `pub use` of `THRESHOLD_SHARE_DECRYPTION_BIT_SK` (C6 `BIT_SK`).
