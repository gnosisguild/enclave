// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::core::threshold::decrypted_shares_aggregation::Configs as DecryptedSharesAggregationConfigs;
use crate::core::threshold::pk_aggregation::Configs as PkAggregationConfigs;
use crate::core::threshold::pk_generation::Configs as PkGenerationConfigs;
use crate::core::threshold::share_decryption::Configs as ShareDecryptionConfigs;
use crate::core::threshold::user_data_encryption::Configs as UserDataEncryptionConfigs;

// Global configs for threshold insecure preset
pub global N: u32 = 512;
pub global L: u32 = 2;
pub global PLAINTEXT_MODULUS: Field = 100;
pub global QIS: [Field; L] = [68719403009, 68719230977];
pub global Q_MOD_T: Field = 3;
pub global Q_INVERSE_MOD_T: Field = 7;
pub global T_INV_MOD_Q: Field = 1416703358393105942938;

/************************************
-------------------------------------
pk_generation (CIRCUIT 1)
-------------------------------------
************************************/

// pk_generation - bit parameters
pub global PK_GENERATION_BIT_EEK: u32 = 6;
pub global PK_GENERATION_BIT_SK: u32 = 2;
pub global PK_GENERATION_BIT_E_SM: u32 = 18;
pub global PK_GENERATION_BIT_R1: u32 = 14;
pub global PK_GENERATION_BIT_R2: u32 = 36;
pub global PK_GENERATION_BIT_PK: u32 = 36;

// pk_generation - bounds
pub global PK_GENERATION_EEK_BOUND: Field = 20;
pub global PK_GENERATION_SK_BOUND: Field = 1;
pub global PK_GENERATION_E_SM_BOUND: Field = 122892;
pub global PK_GENERATION_R1_BOUNDS: [Field; L] = [5120, 5120];
pub global PK_GENERATION_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];

// pk_generation - configs
pub global PK_GENERATION_CONFIGS: PkGenerationConfigs<N, L> = PkGenerationConfigs::new(
    QIS,
    PK_GENERATION_EEK_BOUND,
    PK_GENERATION_SK_BOUND,
    PK_GENERATION_E_SM_BOUND,
    PK_GENERATION_R1_BOUNDS,
    PK_GENERATION_R2_BOUNDS,
);

/************************************
-------------------------------------
pk_aggregation (CIRCUIT 5)
-------------------------------------
************************************/

// pk_aggregation - bit parameters
pub global PK_AGGREGATION_BIT_PK: u32 = 36;

// pk_aggregation - configs
pub global PK_AGGREGATION_CONFIGS: PkAggregationConfigs<L> = PkAggregationConfigs::new(QIS);

/************************************
-------------------------------------
user_data_encryption (CIRCUIT 6)
-------------------------------------
************************************/

pub global USER_DATA_ENCRYPTION_BIT_PK: u32 = 35;
pub global USER_DATA_ENCRYPTION_BIT_CT: u32 = 35;
pub global USER_DATA_ENCRYPTION_BIT_U: u32 = 1;
pub global USER_DATA_ENCRYPTION_BIT_E0: u32 = 5;
pub global USER_DATA_ENCRYPTION_BIT_E1: u32 = 5;
pub global USER_DATA_ENCRYPTION_BIT_K: u32 = 6;
pub global USER_DATA_ENCRYPTION_BIT_R1: u32 = 9;
pub global USER_DATA_ENCRYPTION_BIT_R2: u32 = 35;
pub global USER_DATA_ENCRYPTION_BIT_P1: u32 = 9;
pub global USER_DATA_ENCRYPTION_BIT_P2: u32 = 35;

pub global USER_DATA_ENCRYPTION_Q_MOD_T_MOD_P: Field =
    21888242871839275222246405745257275088548364400416034343698204186575808495610;
pub global USER_DATA_ENCRYPTION_K0IS: [Field; L] = [61160268678, 8933500027];
pub global USER_DATA_ENCRYPTION_PK_BOUNDS: [Field; L] = [34359701504, 34359615488];
pub global USER_DATA_ENCRYPTION_E0_BOUND: Field = 20;
pub global USER_DATA_ENCRYPTION_E1_BOUND: Field = 20;
pub global USER_DATA_ENCRYPTION_U_BOUND: Field = 1;
pub global USER_DATA_ENCRYPTION_K1_LOW_BOUND: Field = 50;
pub global USER_DATA_ENCRYPTION_K1_UP_BOUND: Field = 49;
pub global USER_DATA_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [301, 263];
pub global USER_DATA_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [300, 263];
pub global USER_DATA_ENCRYPTION_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];
pub global USER_DATA_ENCRYPTION_P1_BOUNDS: [Field; L] = [256, 256];
pub global USER_DATA_ENCRYPTION_P2_BOUNDS: [Field; L] = [34359701504, 34359615488];

pub global USER_DATA_ENCRYPTION_CONFIGS: UserDataEncryptionConfigs<N, L> = UserDataEncryptionConfigs::new(
    USER_DATA_ENCRYPTION_Q_MOD_T_MOD_P,
    QIS,
    USER_DATA_ENCRYPTION_K0IS,
    USER_DATA_ENCRYPTION_PK_BOUNDS,
    USER_DATA_ENCRYPTION_E0_BOUND,
    USER_DATA_ENCRYPTION_E1_BOUND,
    USER_DATA_ENCRYPTION_U_BOUND,
    USER_DATA_ENCRYPTION_R1_LOW_BOUNDS,
    USER_DATA_ENCRYPTION_R1_UP_BOUNDS,
    USER_DATA_ENCRYPTION_R2_BOUNDS,
    USER_DATA_ENCRYPTION_P1_BOUNDS,
    USER_DATA_ENCRYPTION_P2_BOUNDS,
    USER_DATA_ENCRYPTION_K1_LOW_BOUND,
    USER_DATA_ENCRYPTION_K1_UP_BOUND,
);

/************************************
-------------------------------------
share_decryption (GRECO)
-------------------------------------
************************************/

// share_decryption - bit parameters
pub global SHARE_DECRYPTION_BIT_CT: u32 = 36;
pub global SHARE_DECRYPTION_BIT_SK: u32 = 36;
pub global SHARE_DECRYPTION_BIT_E_SM: u32 = 36;
pub global SHARE_DECRYPTION_BIT_R1: u32 = 44;
pub global SHARE_DECRYPTION_BIT_R2: u32 = 36;
pub global SHARE_DECRYPTION_BIT_D: u32 = 36;

// share_decryption - bounds
pub global SHARE_DECRYPTION_R1_BOUNDS: [Field; L] = [8796083584897, 8796061564801];
pub global SHARE_DECRYPTION_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];

// share_decryption - configs
pub global SHARE_DECRYPTION_CONFIGS: ShareDecryptionConfigs<L> =
    ShareDecryptionConfigs::new(QIS, SHARE_DECRYPTION_R1_BOUNDS, SHARE_DECRYPTION_R2_BOUNDS);

/************************************
-------------------------------------
decrypted_shares_aggregation (CIRCUIT 7)
-------------------------------------
************************************/

// decrypted_shares_aggregation - bit parameters
pub global DECRYPTED_SHARES_AGGREGATION_BIT_NOISE: u32 = 69;

// decrypted_shares_aggregation - configs
pub global DECRYPTED_SHARES_AGGREGATION_CONFIGS: DecryptedSharesAggregationConfigs<L> =
    DecryptedSharesAggregationConfigs::new(QIS, PLAINTEXT_MODULUS, Q_INVERSE_MOD_T);
