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
pub global PLAINTEXT_MODULUS: Field = 68719403009;
pub global QIS: [Field; L] = [2251799813554177];
pub global Q_MOD_T: Field = 1082658244788225;
pub global Q_INVERSE_MOD_T: Field = 39210117854;

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters
pub global PK_BIT_PK: u32 = 50;

/// Parity check matrix for each modulus.
/// PARITY_MATRIX[modulus_idx][row][col] where size is [L][N_PARTIES-T][N_PARTIES+1]
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
-------------------------------------
************************************/

// share_encryption_sk - bit parameters
pub global SHARE_ENCRYPTION_BIT_PK: u32 = 51;
pub global SHARE_ENCRYPTION_BIT_CT: u32 = 51;
pub global SHARE_ENCRYPTION_BIT_U: u32 = 2;
pub global SHARE_ENCRYPTION_BIT_E0: u32 = 4;
pub global SHARE_ENCRYPTION_BIT_E1: u32 = 4;
pub global SHARE_ENCRYPTION_BIT_MSG: u32 = 37;
pub global SHARE_ENCRYPTION_BIT_R1: u32 = 36;
pub global SHARE_ENCRYPTION_BIT_R2: u32 = 51;
pub global SHARE_ENCRYPTION_BIT_P1: u32 = 10;
pub global SHARE_ENCRYPTION_BIT_P2: u32 = 51;

// share_encryption_sk - bounds
pub global SHARE_ENCRYPTION_T: Field = 68719403009;
pub global SHARE_ENCRYPTION_Q_MOD_T: Field = 2415755265;
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

// share_encryption_sk - configs
pub global SHARE_ENCRYPTION_CONFIGS_SK: ShareEncryptionConfigs<L> = ShareEncryptionConfigs::new(
    SHARE_ENCRYPTION_T,
    SHARE_ENCRYPTION_Q_MOD_T,
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
share_encryption_e_sm (CIRCUIT 3b)
-------------------------------------
************************************/

// share_encryption_e_sm uses the same bit parameters and bounds as share_encryption_sk
pub global SHARE_ENCRYPTION_CONFIGS_E_SM: ShareEncryptionConfigs<L> = ShareEncryptionConfigs::new(
    SHARE_ENCRYPTION_T,
    SHARE_ENCRYPTION_Q_MOD_T,
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
share_decryption_sk (CIRCUIT 4a)
-------------------------------------
************************************/

// share_decryption_sk - bit parameters
pub global SHARE_DECRYPTION_BIT_MSG_SK: u32 = 37;

/************************************
-------------------------------------
share_decryption_e_sm (CIRCUIT 4b)
-------------------------------------
************************************/

// share_decryption_e_sm - bit parameters
pub global SHARE_DECRYPTION_BIT_MSG_E_SM: u32 = 37;
