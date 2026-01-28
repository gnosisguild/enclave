// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::core::bfv_enc::Configs as BfvEncConfigs;

// Global configs for BFV parameter set
pub global N: u32 = 8192;
pub global L_TRBFV: u32 = 4;
pub global L_PRIME: u32 = 2;
pub global L: u32 = 2;
pub global QIS: [Field; L] = [72057594082099201, 72057594062438401];
pub global TRBFV_QIS: [Field; L_TRBFV] =
    [2251799822204929, 4503599627763713, 4503599631433729, 4503599634579457];
pub global Q_MOD_T: Field = 1082658244788225;
pub global PLAINTEXT_MODULUS: Field = 18014398509481984;
pub global Q_INVERSE_MOD_T: Field = 3634521516277761;
pub global T_INVERSE_MOD_Q: Field = 4144717358717138174408418870545186;

/************************************
-------------------------------------
pk_bfv (CIRCUIT 0 - PUBLIC KEY BFV)
-------------------------------------
************************************/

// pk_bfv - bit parameters

pub global PK_BFV_BIT_PK: u32 = 58;

/************************************
-------------------------------------
enc_bfv (CIRCUIT 3a - BFV ENCRYPTION SK)
-------------------------------------
************************************/

// enc_bfv - bit parameters
pub global ENC_BFV_BIT_PK: u32 = 57;
pub global ENC_BFV_BIT_CT: u32 = 57;
pub global ENC_BFV_BIT_U: u32 = 2;
pub global ENC_BFV_BIT_E0: u32 = 6;
pub global ENC_BFV_BIT_E1: u32 = 6;
pub global ENC_BFV_BIT_MSG: u32 = 55;
pub global ENC_BFV_BIT_R1: u32 = 54;
pub global ENC_BFV_BIT_R2: u32 = 57;
pub global ENC_BFV_BIT_P1: u32 = 14;
pub global ENC_BFV_BIT_P2: u32 = 57;

// enc_bfv - bounds
pub global ENC_BFV_T: Field = 18014398509481984;
pub global ENC_BFV_Q_MOD_T: Field = 1082658244788225;
pub global ENC_BFV_K0IS: [Field; L] = [70854796903366627, 47439047573780733];
pub global ENC_BFV_PK_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global ENC_BFV_E0_BOUND: Field = 20;
pub global ENC_BFV_E1_BOUND: Field = 20;
pub global ENC_BFV_U_BOUND: Field = 1;
pub global ENC_BFV_R1_LOW_BOUNDS: [Field; L] = [8856849607495681, 5929880944709633];
pub global ENC_BFV_R1_UP_BOUNDS: [Field; L] = [8856849607495680, 5929880944709632];
pub global ENC_BFV_R2_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global ENC_BFV_P1_BOUNDS: [Field; L] = [4096, 4096];
pub global ENC_BFV_P2_BOUNDS: [Field; L] = [36028797041049600, 36028797031219200];
pub global ENC_BFV_MSG_BOUND: Field = 18014398509481983;

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
pub global DEC_BFV_BIT_MSG_SK: u32 = 56;

/************************************
-------------------------------------
dec_bfv (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

// dec_bfv - bit parameters
pub global DEC_BFV_BIT_MSG_E_SM: u32 = 56;
