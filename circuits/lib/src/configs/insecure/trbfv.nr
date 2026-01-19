// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::core::greco::Configs as GrecoConfigs;
use crate::core::trbfv_dec_share::Configs as DecShareTrBfvConfigs;
use crate::core::trbfv_dec_shares_agg::Configs as DecShareAggTrBfvConfigs;
use crate::core::trbfv_pk::Configs as TrbfvPkConfigs;
use crate::core::trbfv_pk_agg::Configs as PkAggTrBfvConfigs;
use crate::core::trbfv_verify_shares::Configs as VerifySharesConfigs;

// Global configs for BFV parameter set
pub global N: u32 = 512;
pub global L: u32 = 2;
pub global PLAINTEXT_MODULUS: Field = 10;
pub global QIS: [Field; L] = [68719403009, 68719230977];
pub global Q_MOD_T_MOD_P: Field = 3;
pub global Q_MOD_T: Field = 3;
pub global Q_INVERSE_MOD_T: Field = 7;
pub global T_INV_MOD_Q: Field = 1416703358393105942938;

/************************************
-------------------------------------
pk_trbfv (CIRCUIT 1 - PUBLIC KEY THRESHOLD BFV)
-------------------------------------
************************************/

// pk_trbfv - bit parameters
pub global PK_TRBFV_BIT_EEK: u32 = 6;
pub global PK_TRBFV_BIT_SK: u32 = 2;
pub global PK_TRBFV_BIT_E_SM: u32 = 18;
pub global PK_TRBFV_BIT_R1: u32 = 14;
pub global PK_TRBFV_BIT_R2: u32 = 36;
pub global PK_TRBFV_BIT_PK: u32 = 36;

// pk_trbfv - bounds
pub global PK_TRBFV_EEK_BOUND: Field = 20;
pub global PK_TRBFV_SK_BOUND: Field = 1;
pub global PK_TRBFV_E_SM_BOUND: Field = 122892;
pub global PK_TRBFV_R1_BOUNDS: [Field; L] = [5120, 5120];
pub global PK_TRBFV_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];

// pk_trbfv - configs
pub global PK_TRBFV_CONFIGS: TrbfvPkConfigs<N, L> = TrbfvPkConfigs::new(
    QIS,
    PK_TRBFV_EEK_BOUND,
    PK_TRBFV_SK_BOUND,
    PK_TRBFV_E_SM_BOUND,
    PK_TRBFV_R1_BOUNDS,
    PK_TRBFV_R2_BOUNDS,
);

/************************************
-------------------------------------
verify_shares (CIRCUIT 2a - VERIFY SHARES SK)
-------------------------------------
************************************/

// verify_shares - bit parameters
pub global VERIFY_SHARES_BIT_SHARE: u32 = 37;
pub global VERIFY_SHARES_BIT_SECRET_SK: u32 = 2;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_SK: VerifySharesConfigs<L> = VerifySharesConfigs::new(QIS);

/************************************
-------------------------------------
verify_shares (CIRCUIT 2b - VERIFY SHARES E_SM)
-------------------------------------
************************************/

// verify_shares - bit parameters
pub global VERIFY_SHARES_BIT_SECRET_E_SM: u32 = 18;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_E_SM: VerifySharesConfigs<L> = VerifySharesConfigs::new(QIS);

/************************************
-------------------------------------
pk_agg_trbfv (CIRCUIT 5 - PUBLIC KEY AGGREGATION TRBFV)
-------------------------------------
************************************/

// pk_agg_trbfv - bit parameters
pub global PK_AGG_TRBFV_BIT_PK: u32 = 36;

// pk_agg_trbfv - configs
pub global PK_AGG_TRBFV_CONFIGS: PkAggTrBfvConfigs<L> = PkAggTrBfvConfigs::new(QIS);

/************************************
-------------------------------------
greco (USED FOR ENCRYPTION TRBFV x PVSS)
-------------------------------------
************************************/

// greco - bit parameters
pub global GRECO_BIT_PK: u32 = 36;
pub global GRECO_BIT_CT: u32 = 36;
pub global GRECO_BIT_U: u32 = 2;
pub global GRECO_BIT_E0: u32 = 6;
pub global GRECO_BIT_E1: u32 = 6;
pub global GRECO_BIT_K: u32 = 4;
pub global GRECO_BIT_R1: u32 = 10;
pub global GRECO_BIT_R2: u32 = 36;
pub global GRECO_BIT_P1: u32 = 10;
pub global GRECO_BIT_P2: u32 = 36;

// greco - bounds
pub global GRECO_Q_MOD_T: Field = 3;
pub global GRECO_K0IS: [Field; L] = [61847462708, 20615769293];
pub global GRECO_PK_BOUNDS: [Field; L] = [34359701504, 34359615488];
pub global GRECO_E0_BOUND: Field = 20;
pub global GRECO_E1_BOUND: Field = 20;
pub global GRECO_U_BOUND: Field = 1;
pub global GRECO_K1_LOW_BOUND: Field = 5;
pub global GRECO_K1_UP_BOUND: Field = 4;
pub global GRECO_R1_LOW_BOUNDS: [Field; L] = [261, 258];
pub global GRECO_R1_UP_BOUNDS: [Field; L] = [260, 258];
pub global GRECO_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];
pub global GRECO_P1_BOUNDS: [Field; L] = [256, 256];
pub global GRECO_P2_BOUNDS: [Field; L] = [34359701504, 34359615488];

// greco - configs
pub global GRECO_CONFIGS: GrecoConfigs<N, L> = GrecoConfigs::new(
    GRECO_Q_MOD_T,
    QIS,
    GRECO_K0IS,
    GRECO_PK_BOUNDS,
    GRECO_E0_BOUND,
    GRECO_E1_BOUND,
    GRECO_U_BOUND,
    GRECO_R1_LOW_BOUNDS,
    GRECO_R1_UP_BOUNDS,
    GRECO_R2_BOUNDS,
    GRECO_P1_BOUNDS,
    GRECO_P2_BOUNDS,
    GRECO_K1_LOW_BOUND,
    GRECO_K1_UP_BOUND,
);

/************************************
-------------------------------------
dec_share_trbfv (CIRCUIT 6 - DECRYPTION SHARE TRBFV)
-------------------------------------
************************************/

// dec_share_trbfv - bit parameters
pub global DEC_SHARES_BIT_CT: u32 = 36;
pub global DEC_SHARES_BIT_S: u32 = 36;
pub global DEC_SHARES_BIT_E: u32 = 36;
pub global DEC_SHARES_BIT_R1: u32 = 44;
pub global DEC_SHARES_BIT_R2: u32 = 36;
pub global DEC_SHARES_BIT_D: u32 = 36;

// dec_share_trbfv - bounds
pub global DEC_SHARES_R1_BOUNDS: [Field; L] = [8796083584897, 8796061564801];
pub global DEC_SHARES_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];

// dec_share_trbfv - configs
pub global DEC_SHARES_CONFIGS: DecShareTrBfvConfigs<L> =
    DecShareTrBfvConfigs::new(QIS, DEC_SHARES_R1_BOUNDS, DEC_SHARES_R2_BOUNDS);

/************************************
-------------------------------------
dec_shares_agg_trbfv (CIRCUIT 7 - DECRYPTION SHARE AGGREGATION TRBFV)
-------------------------------------
************************************/

// dec_shares_agg_trbfv - bit parameters
pub global DEC_SHARES_AGG_BIT_NOISE: u32 = 69;

// dec_shares_agg_trbfv - configs
pub global DEC_SHARES_AGG_CONFIGS: DecShareAggTrBfvConfigs<L> =
    DecShareAggTrBfvConfigs::new(QIS, PLAINTEXT_MODULUS, Q_INVERSE_MOD_T);
