// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::configs::default::{N_PARTIES, T};
pub use crate::configs::insecure::threshold::{L as L_THRESHOLD, QIS as QIS_THRESHOLD};
use crate::core::bfv_enc::Configs as BfvEncConfigs;
use crate::core::trbfv_verify_shares::Configs as VerifySharesConfigs;

// Global configs for DKG insecure preset
pub global N: u32 = 512;
pub global L: u32 = 1;
pub global PLAINTEXT_MODULUS: Field = 68719403009;
pub global QIS: [Field; L] = [2251799813554177];
pub global Q_MOD_T: Field = 1082658244788225;
pub global Q_INVERSE_MOD_T: Field = 39210117854;

/************************************
-------------------------------------
pk_bfv (CIRCUIT 0 - PUBLIC KEY BFV)
-------------------------------------
************************************/

// pk_bfv - bit parameters
pub global PK_BFV_BIT_PK: u32 = 50;

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
verify_shares (CIRCUIT 2a - VERIFY SHARES SK)
-------------------------------------
************************************/

// verify_shares - bit parameters
pub global VERIFY_SHARES_BIT_SHARE: u32 = 37;
pub global VERIFY_SHARES_BIT_SECRET_SK: u32 = 2;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_SK: VerifySharesConfigs<L_THRESHOLD> =
    VerifySharesConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
verify_shares (CIRCUIT 2b - VERIFY SHARES E_SM)
-------------------------------------
************************************/

// verify_shares - bit parameters
pub global VERIFY_SHARES_BIT_SECRET_E_SM: u32 = 18;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_E_SM: VerifySharesConfigs<L_THRESHOLD> =
    VerifySharesConfigs::new(QIS_THRESHOLD);

/************************************
-------------------------------------
enc_bfv (CIRCUIT 3a - BFV ENCRYPTION SK)
-------------------------------------
************************************/

// enc_bfv - bit parameters
pub global ENC_BFV_BIT_PK: u32 = 51;
pub global ENC_BFV_BIT_CT: u32 = 51;
pub global ENC_BFV_BIT_U: u32 = 2;
pub global ENC_BFV_BIT_E0: u32 = 4;
pub global ENC_BFV_BIT_E1: u32 = 4;
pub global ENC_BFV_BIT_MSG: u32 = 37;
pub global ENC_BFV_BIT_R1: u32 = 36;
pub global ENC_BFV_BIT_R2: u32 = 51;
pub global ENC_BFV_BIT_P1: u32 = 10;
pub global ENC_BFV_BIT_P2: u32 = 51;

// enc_bfv - bounds
pub global ENC_BFV_T: Field = 68719403009;
pub global ENC_BFV_Q_MOD_T: Field = 2415755265;
pub global ENC_BFV_K0IS: [Field; L] = [1284838520228573];
pub global ENC_BFV_PK_BOUNDS: [Field; L] = [1125899906777088];
pub global ENC_BFV_E0_BOUND: Field = 6;
pub global ENC_BFV_E1_BOUND: Field = 6;
pub global ENC_BFV_U_BOUND: Field = 1;
pub global ENC_BFV_R1_LOW_BOUNDS: [Field; L] = [19605059183];
pub global ENC_BFV_R1_UP_BOUNDS: [Field; L] = [19605059183];
pub global ENC_BFV_R2_BOUNDS: [Field; L] = [1125899906777088];
pub global ENC_BFV_P1_BOUNDS: [Field; L] = [256];
pub global ENC_BFV_P2_BOUNDS: [Field; L] = [1125899906777088];
pub global ENC_BFV_MSG_BOUND: Field = 68719403008;

// enc_bfv - configs
pub global ENC_BFV_CONFIGS_SK: BfvEncConfigs<L> = BfvEncConfigs::new(
    ENC_BFV_T,
    ENC_BFV_Q_MOD_T,
    QIS,
    ENC_BFV_K0IS,
    ENC_BFV_PK_BOUNDS,
    ENC_BFV_E0_BOUND,
    ENC_BFV_E1_BOUND,
    ENC_BFV_U_BOUND,
    ENC_BFV_R1_LOW_BOUNDS,
    ENC_BFV_R1_UP_BOUNDS,
    ENC_BFV_R2_BOUNDS,
    ENC_BFV_P1_BOUNDS,
    ENC_BFV_P2_BOUNDS,
    ENC_BFV_MSG_BOUND,
);

/************************************
-------------------------------------
enc_bfv (CIRCUIT 3b - BFV ENCRYPTION E_SM)
-------------------------------------
************************************/

// enc_bfv E_SM uses the same bit parameters and bounds as SK
pub global ENC_BFV_CONFIGS_E_SM: BfvEncConfigs<L> = BfvEncConfigs::new(
    ENC_BFV_T,
    ENC_BFV_Q_MOD_T,
    QIS,
    ENC_BFV_K0IS,
    ENC_BFV_PK_BOUNDS,
    ENC_BFV_E0_BOUND,
    ENC_BFV_E1_BOUND,
    ENC_BFV_U_BOUND,
    ENC_BFV_R1_LOW_BOUNDS,
    ENC_BFV_R1_UP_BOUNDS,
    ENC_BFV_R2_BOUNDS,
    ENC_BFV_P1_BOUNDS,
    ENC_BFV_P2_BOUNDS,
    ENC_BFV_MSG_BOUND,
);

/************************************
-------------------------------------
dec_bfv (CIRCUIT 4a - BFV DECRYPTION SK)
-------------------------------------
************************************/

// dec_bfv - bit parameters
pub global DEC_BFV_BIT_MSG_SK: u32 = 37;

/************************************
-------------------------------------
dec_bfv (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

// dec_bfv - bit parameters
pub global DEC_BFV_BIT_MSG_E_SM: u32 = 37;
