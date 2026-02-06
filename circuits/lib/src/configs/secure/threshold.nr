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

// Global configs for threshold secure preset
pub global N: u32 = 8192;
pub global L: u32 = 4;
pub global PLAINTEXT_MODULUS: Field = 100;
pub global QIS: [Field; L] =
    [2251799822204929, 4503599627763713, 4503599631433729, 4503599634579457];
pub global Q_MOD_T_MOD_P: Field =
    21888242871839275222246405745257275088548364400416034343698204186575808495598;
pub global Q_MOD_T: Field = 81;
pub global Q_INVERSE_MOD_T: Field = 21;
pub global T_INV_MOD_Q: Field = 162493576071160894262400971579825215135378801169890023914222388;

/************************************
-------------------------------------
pk_generation (CIRCUIT 1)
-------------------------------------
************************************/

pub global PK_GENERATION_BIT_EEK: u32 = 5;
pub global PK_GENERATION_BIT_SK: u32 = 1;
pub global PK_GENERATION_BIT_E_SM: u32 = 192;
pub global PK_GENERATION_BIT_R1: u32 = 17;
pub global PK_GENERATION_BIT_R2: u32 = 52;
pub global PK_GENERATION_BIT_PK: u32 = 52;

pub global PK_GENERATION_EEK_BOUND: Field = 20;
pub global PK_GENERATION_SK_BOUND: Field = 1;
pub global PK_GENERATION_E_SM_BOUND: Field =
    4789048565205902682369834578696752269343944643454802329600;
pub global PK_GENERATION_R1_BOUNDS: [Field; L] = [81920, 81920, 81920, 81920];
pub global PK_GENERATION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

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

pub global PK_AGGREGATION_BIT_PK: u32 = 52;

pub global PK_AGGREGATION_CONFIGS: PkAggregationConfigs<L> = PkAggregationConfigs::new(QIS);

/************************************
-------------------------------------
user_data_encryption (GRECO)
-------------------------------------
************************************/

// user_data_encryption - bit parameters
pub global USER_DATA_ENCRYPTION_BIT_PK: u32 = 52;
pub global USER_DATA_ENCRYPTION_BIT_CT: u32 = 52;
pub global USER_DATA_ENCRYPTION_BIT_U: u32 = 2;
pub global USER_DATA_ENCRYPTION_BIT_E0: u32 = 107;
pub global USER_DATA_ENCRYPTION_BIT_E1: u32 = 6;
pub global USER_DATA_ENCRYPTION_BIT_K: u32 = 7;
pub global USER_DATA_ENCRYPTION_BIT_R1: u32 = 14;
pub global USER_DATA_ENCRYPTION_BIT_R2: u32 = 53;
pub global USER_DATA_ENCRYPTION_BIT_P1: u32 = 14;
pub global USER_DATA_ENCRYPTION_BIT_P2: u32 = 53;

// user_data_encryption - bounds
pub global USER_DATA_ENCRYPTION_K0IS: [Field; L] =
    [1553741877321401, 3467771713378059, 3107483745689273, 4188347660158895];
pub global USER_DATA_ENCRYPTION_PK_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global USER_DATA_ENCRYPTION_E0_BOUND: Field = 54890881146011919040994244755389;
pub global USER_DATA_ENCRYPTION_E1_BOUND: Field = 20;
pub global USER_DATA_ENCRYPTION_U_BOUND: Field = 1;
pub global USER_DATA_ENCRYPTION_K1_LOW_BOUND: Field = 50;
pub global USER_DATA_ENCRYPTION_K1_UP_BOUND: Field = 49;
pub global USER_DATA_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [4132, 4136, 4131, 4143];
pub global USER_DATA_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [4131, 4135, 4131, 4142];
pub global USER_DATA_ENCRYPTION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global USER_DATA_ENCRYPTION_P1_BOUNDS: [Field; L] = [4096, 4096, 4096, 4096];
pub global USER_DATA_ENCRYPTION_P2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

// greco - configs
pub global USER_DATA_ENCRYPTION_CONFIGS: UserDataEncryptionConfigs<N, L> = UserDataEncryptionConfigs::new(
    Q_MOD_T_MOD_P,
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
share_decryption (CIRCUIT 6)
-------------------------------------
************************************/

// share_decryption - bit parameters
pub global SHARE_DECRYPTION_BIT_CT: u32 = 53;
pub global SHARE_DECRYPTION_BIT_SK: u32 = 53;
pub global SHARE_DECRYPTION_BIT_E_SM: u32 = 53;
pub global SHARE_DECRYPTION_BIT_R1: u32 = 65;
pub global SHARE_DECRYPTION_BIT_R2: u32 = 53;
pub global SHARE_DECRYPTION_BIT_D: u32 = 53;

// share_decryption - bounds
pub global SHARE_DECRYPTION_R1_BOUNDS: [Field; L] =
    [4611686035875690497, 9223372037660080129, 9223372045176272897, 9223372051618723841];
pub global SHARE_DECRYPTION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

// share_decryption - configs
pub global SHARE_DECRYPTION_CONFIGS: ShareDecryptionConfigs<L> =
    ShareDecryptionConfigs::new(QIS, SHARE_DECRYPTION_R1_BOUNDS, SHARE_DECRYPTION_R2_BOUNDS);

/************************************
-------------------------------------
decrypted_shares_aggregation (CIRCUIT 7)
-------------------------------------
************************************/

// decrypted_shares_aggregation - bit parameters
pub global DECRYPTED_SHARES_AGGREGATION_BIT_NOISE: u32 = 201;

// decrypted_shares_aggregation - configs
pub global DECRYPTED_SHARES_AGGREGATION_CONFIGS: DecryptedSharesAggregationConfigs<L> =
    DecryptedSharesAggregationConfigs::new(QIS, PLAINTEXT_MODULUS, Q_INVERSE_MOD_T);
