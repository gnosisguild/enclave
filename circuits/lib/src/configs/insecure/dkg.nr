// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::insecure::threshold::{L as L_THRESHOLD, QIS as QIS_THRESHOLD};
use crate::core::dkg::share_computation::Configs as ShareComputationConfigs;
use crate::core::dkg::share_encryption::Configs as ShareEncryptionConfigs;

// Global configs for DKG insecure preset
pub global N: u32 = 512;
pub global L: u32 = 1;
pub global QIS: [Field; L] = [2251799813554177];

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters
pub global PK_BIT_PK: u32 = 50;

pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [
    [
        [68719403008, 3, 68719403006, 1, 0, 0],
        [68719403006, 8, 68719403003, 0, 1, 0],
        [68719403003, 15, 68719402999, 0, 0, 1],
    ],
    [
        [68719230976, 3, 68719230974, 1, 0, 0],
        [68719230974, 8, 68719230971, 0, 1, 0],
        [68719230971, 15, 68719230967, 0, 0, 1],
    ],
];

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

// share_computation_sk - bit parameters
pub global SHARE_COMPUTATION_BIT_SHARE: u32 = 36;
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
pub global SHARE_COMPUTATION_E_SM_BIT_SECRET: u32 = 24;

// verify_shares - configs
pub global SHARE_COMPUTATION_E_SM_CONFIGS: ShareComputationConfigs<L_THRESHOLD> =
    ShareComputationConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_encryption_sk (CIRCUIT 3a)
share_encryption_e_sm (CIRCUIT 3b)
-------------------------------------
************************************/

pub global DKG_SHARE_ENCRYPTION_BIT_PK: u32 = 50;
pub global DKG_SHARE_ENCRYPTION_BIT_CT: u32 = 50;
pub global DKG_SHARE_ENCRYPTION_BIT_U: u32 = 1;
pub global DKG_SHARE_ENCRYPTION_BIT_E0: u32 = 3;
pub global DKG_SHARE_ENCRYPTION_BIT_E1: u32 = 3;
pub global DKG_SHARE_ENCRYPTION_BIT_MSG: u32 = 36;
pub global DKG_SHARE_ENCRYPTION_BIT_R1: u32 = 35;
pub global DKG_SHARE_ENCRYPTION_BIT_R2: u32 = 50;
pub global DKG_SHARE_ENCRYPTION_BIT_P1: u32 = 9;
pub global DKG_SHARE_ENCRYPTION_BIT_P2: u32 = 50;

pub global DKG_SHARE_ENCRYPTION_T: Field = 68719403009;
pub global DKG_SHARE_ENCRYPTION_Q_MOD_T: Field = 2415755265;
pub global DKG_SHARE_ENCRYPTION_K0IS: [Field; L] = [1284838520228573];
pub global DKG_SHARE_ENCRYPTION_PK_BOUNDS: [Field; L] = [1125899906777088];
pub global DKG_SHARE_ENCRYPTION_E0_BOUND: Field = 6;
pub global DKG_SHARE_ENCRYPTION_E1_BOUND: Field = 6;
pub global DKG_SHARE_ENCRYPTION_U_BOUND: Field = 1;
pub global DKG_SHARE_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [19605059183];
pub global DKG_SHARE_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [19605059183];
pub global DKG_SHARE_ENCRYPTION_R2_BOUNDS: [Field; L] = [1125899906777088];
pub global DKG_SHARE_ENCRYPTION_P1_BOUNDS: [Field; L] = [256];
pub global DKG_SHARE_ENCRYPTION_P2_BOUNDS: [Field; L] = [1125899906777088];
pub global DKG_SHARE_ENCRYPTION_MSG_BOUND: Field = 68719403008;

pub global DKG_SHARE_ENCRYPTION_CONFIGS: ShareEncryptionConfigs<L> = ShareEncryptionConfigs::new(
    DKG_SHARE_ENCRYPTION_T,
    DKG_SHARE_ENCRYPTION_Q_MOD_T,
    QIS,
    DKG_SHARE_ENCRYPTION_K0IS,
    DKG_SHARE_ENCRYPTION_PK_BOUNDS,
    DKG_SHARE_ENCRYPTION_E0_BOUND,
    DKG_SHARE_ENCRYPTION_E1_BOUND,
    DKG_SHARE_ENCRYPTION_U_BOUND,
    DKG_SHARE_ENCRYPTION_R1_LOW_BOUNDS,
    DKG_SHARE_ENCRYPTION_R1_UP_BOUNDS,
    DKG_SHARE_ENCRYPTION_R2_BOUNDS,
    DKG_SHARE_ENCRYPTION_P1_BOUNDS,
    DKG_SHARE_ENCRYPTION_P2_BOUNDS,
    DKG_SHARE_ENCRYPTION_MSG_BOUND,
);

/************************************
-------------------------------------
share_decryption_sk (CIRCUIT 4a - BFV DECRYPTION SK)
share_decryption_e_sm (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

pub global SHARE_DECRYPTION_BIT_MSG: u32 = 36;
