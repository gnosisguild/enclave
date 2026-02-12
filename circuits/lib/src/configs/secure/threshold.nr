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

/// Threshold BFV parameter set search defaults configurations.
/// These are for the SecureThreshold8192 preset.
/// The InsecureThreshold512 preset has been generated manually.
pub global PARAMS_SEARCH_N: Field = 100;
pub global PARAMS_SEARCH_Z: Field = 100;

// Global configs for threshold secure preset
pub global N: u32 = 8192;
pub global L: u32 = 4;
pub global QIS: [Field; L] =
    [2251799822204929, 4503599627763713, 4503599631433729, 4503599634579457];
pub global Q_MOD_T: Field = -19;
pub global PLAINTEXT_MODULUS: Field = 100;
pub global Q_INVERSE_MOD_T: Field = 21;
pub global Q_MOD_T: Field = -19;

/************************************
-------------------------------------
pk_generation (CIRCUIT 1 - PUBLIC KEY THRESHOLD BFV)
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
    4789048565205902682369836460365611983121962490123622809600;
pub global PK_GENERATION_R1_BOUNDS: [Field; L] = [81920, 81920, 81920, 81920];
pub global PK_GENERATION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

pub global PK_GENERATION_B_ENC: Field = 39614081257132168796771975168000;

pub global PK_GENERATION_B_ENC: Field = 39614081257132168796771975168000;

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
user_data_encryption (USED FOR DATA ENCRYPTION)
-------------------------------------
************************************/

pub global USER_DATA_ENCRYPTION_BIT_PK: u32 = 52;
pub global USER_DATA_ENCRYPTION_BIT_CT: u32 = 52;
pub global USER_DATA_ENCRYPTION_BIT_U: u32 = 1;
pub global USER_DATA_ENCRYPTION_BIT_E0: u32 = 105;
pub global USER_DATA_ENCRYPTION_BIT_E1: u32 = 5;
pub global USER_DATA_ENCRYPTION_BIT_K: u32 = 6;
pub global USER_DATA_ENCRYPTION_BIT_R1: u32 = 13;
pub global USER_DATA_ENCRYPTION_BIT_R2: u32 = 52;
pub global USER_DATA_ENCRYPTION_BIT_P1: u32 = 13;
pub global USER_DATA_ENCRYPTION_BIT_P2: u32 = 52;

pub global USER_DATA_ENCRYPTION_K0IS: [Field; L] =
    [1553741877321401, 3467771713378059, 3107483745689273, 4188347660158895];
pub global USER_DATA_ENCRYPTION_PK_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global USER_DATA_ENCRYPTION_E0_BOUND: Field = 39614081257132168796771975168000;
pub global USER_DATA_ENCRYPTION_E1_BOUND: Field = 20;
pub global USER_DATA_ENCRYPTION_U_BOUND: Field = 1;
pub global USER_DATA_ENCRYPTION_K1_LOW_BOUND: Field = 50;
pub global USER_DATA_ENCRYPTION_K1_UP_BOUND: Field = 49;
pub global USER_DATA_ENCRYPTION_R1_LOW_BOUNDS: [Field; L] = [4131, 4135, 4131, 4143];
pub global USER_DATA_ENCRYPTION_R1_UP_BOUNDS: [Field; L] = [4131, 4134, 4130, 4142];
pub global USER_DATA_ENCRYPTION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global USER_DATA_ENCRYPTION_P1_BOUNDS: [Field; L] = [4096, 4096, 4096, 4096];
pub global USER_DATA_ENCRYPTION_P2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

pub global USER_DATA_ENCRYPTION_CONFIGS: UserDataEncryptionConfigs<N, L> = UserDataEncryptionConfigs::new(
    Q_MOD_T,
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
share_decryption (CIRCUIT 6 - THRESHOLD BFV SHARE DECRYPTION)
-------------------------------------
************************************/

pub global THRESHOLD_SHARE_DECRYPTION_BIT_CT: u32 = 52;
pub global THRESHOLD_SHARE_DECRYPTION_BIT_SK: u32 = 52;
pub global THRESHOLD_SHARE_DECRYPTION_BIT_E_SM: u32 = 52;
pub global THRESHOLD_SHARE_DECRYPTION_BIT_R1: u32 = 64;
pub global THRESHOLD_SHARE_DECRYPTION_BIT_R2: u32 = 52;
pub global THRESHOLD_SHARE_DECRYPTION_BIT_D: u32 = 52;

pub global THRESHOLD_SHARE_DECRYPTION_R1_BOUNDS: [Field; L] =
    [4611686035875690497, 9223372037660080129, 9223372045176272897, 9223372051618723841];
pub global THRESHOLD_SHARE_DECRYPTION_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

pub global THRESHOLD_SHARE_DECRYPTION_CONFIGS: ShareDecryptionConfigs<L> = ShareDecryptionConfigs::new(
    QIS,
    THRESHOLD_SHARE_DECRYPTION_R1_BOUNDS,
    THRESHOLD_SHARE_DECRYPTION_R2_BOUNDS,
);

/************************************
-------------------------------------
decrypted_shares_aggregation (CIRCUIT 7)
-------------------------------------
************************************/

pub global DECRYPTED_SHARES_AGGREGATION_BIT_NOISE: u32 = 200;

pub global DECRYPTED_SHARES_AGGREGATION_CONFIGS: DecryptedSharesAggregationConfigs<L> =
    DecryptedSharesAggregationConfigs::new(QIS, PLAINTEXT_MODULUS, Q_INVERSE_MOD_T);
