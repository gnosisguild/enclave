// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::secure::threshold::{L as L_THRESHOLD, QIS as QIS_THRESHOLD};
use crate::core::dkg::share_computation::Configs as ShareComputationConfigs;
use crate::core::dkg::share_encryption::Configs as ShareEncryptionConfigs;

// Global configs for DKG secure preset
pub global N: u32 = 8192;
pub global L: u32 = 2;
pub global PLAINTEXT_MODULUS: Field = 18014398509481984;
pub global QIS: [Field; L] = [72057594082099201, 72057594062438401];
pub global Q_MOD_T: Field = 1082658244788225;
pub global Q_INVERSE_MOD_T: Field = 3634521516277761;
pub global T_INVERSE_MOD_Q: Field = 4144717358717138174408418870545186;

/************************************
-------------------------------------
pk (CIRCUIT 0)
-------------------------------------
************************************/

// pk - bit parameters

pub global PK_BIT_PK: u32 = 58;

/// Parity check matrix for each modulus.
/// PARITY_MATRIX[modulus_idx][row][col] where size is [L][N_PARTIES-T][N_PARTIES+1]
pub global PARITY_MATRIX: [[[Field; N_PARTIES + 1]; N_PARTIES - T]; L_THRESHOLD] = [
    [
        [2251799822204928, 3, 2251799822204926, 1, 0, 0],
        [2251799822204926, 8, 2251799822204923, 0, 1, 0],
        [2251799822204923, 15, 2251799822204919, 0, 0, 1],
    ],
    [
        [4503599627763712, 3, 4503599627763710, 1, 0, 0],
        [4503599627763710, 8, 4503599627763707, 0, 1, 0],
        [4503599627763707, 15, 4503599627763703, 0, 0, 1],
    ],
    [
        [4503599631433728, 3, 4503599631433726, 1, 0, 0],
        [4503599631433726, 8, 4503599631433723, 0, 1, 0],
        [4503599631433723, 15, 4503599631433719, 0, 0, 1],
    ],
    [
        [4503599634579456, 3, 4503599634579454, 1, 0, 0],
        [4503599634579454, 8, 4503599634579451, 0, 1, 0],
        [4503599634579451, 15, 4503599634579447, 0, 0, 1],
    ],
];

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

// share_computation_sk - bit parameters
pub global SHARE_COMPUTATION_BIT_SHARE: u32 = 53;
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
pub global SHARE_COMPUTATION_E_SM_BIT_SECRET: u32 = 192;

// verify_shares - configs
pub global SHARE_COMPUTATION_E_SM_CONFIGS: ShareComputationConfigs<L_THRESHOLD> =
    ShareComputationConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
share_encryption_sk (CIRCUIT 3a)
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

pub global SHARE_ENCRYPTION_T: Field = 18014398509481984;
pub global SHARE_ENCRYPTION_Q_MOD_T: Field = 1082658244788225;
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
share_decryption_sk (CIRCUIT 4a - BFV DECRYPTION SK)
share_decryption_e_sm (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

pub global SHARE_DECRYPTION_BIT_MSG: u32 = 55;
