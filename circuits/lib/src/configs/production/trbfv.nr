use crate::core::greco::Configs as GrecoConfigs;
use crate::core::trbfv_dec_share::Configs as DecShareTrBfvConfigs;
use crate::core::trbfv_dec_shares_agg::Configs as DecShareAggTrBfvConfigs;
use crate::core::trbfv_pk::Configs as TrbfvPkConfigs;
use crate::core::trbfv_pk_agg::Configs as PkAggTrBfvConfigs;
use crate::core::trbfv_verify_shares::Configs as VerifySharesConfigs;

// Global configs for BFV parameter set
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
pk_trbfv (CIRCUIT 1 - PUBLIC KEY THRESHOLD BFV)
-------------------------------------
************************************/

// pk_trbfv - bit parameters
pub global PK_TRBFV_BIT_EEK: u32 = 6;
pub global PK_TRBFV_BIT_SK: u32 = 2;
pub global PK_TRBFV_BIT_E_SM: u32 = 187;
pub global PK_TRBFV_BIT_R1: u32 = 18;
pub global PK_TRBFV_BIT_R2: u32 = 53;
pub global PK_TRBFV_BIT_PK: u32 = 53;

// pk_trbfv - bounds
pub global PK_TRBFV_EEK_BOUND: Field = 20;
pub global PK_TRBFV_SK_BOUND: Field = 1;
pub global PK_TRBFV_E_SM_BOUND: Field = 66359003478811654788063765182202739927396573769378037760;
pub global PK_TRBFV_R1_BOUNDS: [Field; L] = [81920, 81920, 81920, 81920];
pub global PK_TRBFV_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

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
pub global VERIFY_SHARES_BIT_SHARE: u32 = 54;
pub global VERIFY_SHARES_BIT_SECRET_SK: u32 = 2;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_SK: VerifySharesConfigs<L> = VerifySharesConfigs::new(QIS);

/************************************
-------------------------------------
verify_shares (CIRCUIT 2b - VERIFY SHARES E_SM)
-------------------------------------
************************************/

// verify_shares - bit parameters
pub global VERIFY_SHARES_BIT_SECRET_E_SM: u32 = 187;

// verify_shares - configs
pub global VERIFY_SHARES_CONFIGS_E_SM: VerifySharesConfigs<L> = VerifySharesConfigs::new(QIS);

/************************************
-------------------------------------
pk_agg_trbfv (CIRCUIT 5 - PUBLIC KEY AGGREGATION TRBFV)
-------------------------------------
************************************/

// pk_agg_trbfv - bit parameters
pub global PK_AGG_TRBFV_BIT_PK: u32 = 53;

// pk_agg_trbfv - configs
pub global PK_AGG_TRBFV_CONFIGS: PkAggTrBfvConfigs<L> = PkAggTrBfvConfigs::new(QIS);

/************************************
-------------------------------------
greco (USED FOR ENCRYPTION TRBFV x PVSS)
-------------------------------------
************************************/

// greco - bit parameters
pub global GRECO_BIT_PK: u32 = 52;
pub global GRECO_BIT_CT: u32 = 52;
pub global GRECO_BIT_U: u32 = 2;
pub global GRECO_BIT_E0: u32 = 107;
pub global GRECO_BIT_E1: u32 = 6;
pub global GRECO_BIT_K: u32 = 7;
pub global GRECO_BIT_R1: u32 = 14;
pub global GRECO_BIT_R2: u32 = 53;
pub global GRECO_BIT_P1: u32 = 14;
pub global GRECO_BIT_P2: u32 = 53;

// greco - bounds
pub global GRECO_Q_MOD_T: Field =
    21888242871839275222246405745257275088548364400416034343698204186575808495598;
pub global GRECO_K0IS: [Field; L] =
    [1553741877321401, 3467771713378059, 3107483745689273, 4188347660158895];
pub global GRECO_PK_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global GRECO_E0_BOUND: Field = 54890881146011919040994244755389;
pub global GRECO_E1_BOUND: Field = 20;
pub global GRECO_U_BOUND: Field = 1;
pub global GRECO_K1_LOW_BOUND: Field = 50;
pub global GRECO_K1_UP_BOUND: Field = 49;
pub global GRECO_R1_LOW_BOUNDS: [Field; L] = [4132, 4136, 4131, 4143];
pub global GRECO_R1_UP_BOUNDS: [Field; L] = [4131, 4135, 4131, 4142];
pub global GRECO_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];
pub global GRECO_P1_BOUNDS: [Field; L] = [4096, 4096, 4096, 4096];
pub global GRECO_P2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

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
pub global DEC_SHARES_BIT_CT: u32 = 53;
pub global DEC_SHARES_BIT_S: u32 = 53;
pub global DEC_SHARES_BIT_E: u32 = 53;
pub global DEC_SHARES_BIT_R1: u32 = 65;
pub global DEC_SHARES_BIT_R2: u32 = 53;
pub global DEC_SHARES_BIT_D: u32 = 53;

// dec_share_trbfv - bounds
pub global DEC_SHARES_DECRYPTION_SHARE_BOUND: Field = 2251799817289728;
pub global DEC_SHARES_R1_BOUNDS: [Field; L] =
    [4611686035875690497, 9223372037660080129, 9223372045176272897, 9223372051618723841];
pub global DEC_SHARES_R2_BOUNDS: [Field; L] =
    [1125899911102464, 2251799813881856, 2251799815716864, 2251799817289728];

// dec_share_trbfv - configs
pub global DEC_SHARES_CONFIGS: DecShareTrBfvConfigs<L> = DecShareTrBfvConfigs::new(
    QIS,
    DEC_SHARES_DECRYPTION_SHARE_BOUND,
    DEC_SHARES_R1_BOUNDS,
    DEC_SHARES_R2_BOUNDS,
);

/************************************
-------------------------------------
dec_share_agg_trbfv (CIRCUIT 7 - DECRYPTION SHARE AGGREGATION TRBFV)
-------------------------------------
************************************/

// dec_share_agg_trbfv - bit parameters
pub global DEC_SHARES_AGG_BIT_NOISE: u32 = 201;

// dec_share_agg_trbfv - configs
pub global DEC_SHARES_AGG_CONFIGS: DecShareAggTrBfvConfigs<L> =
    DecShareAggTrBfvConfigs::new(QIS, PLAINTEXT_MODULUS, Q_INVERSE_MOD_T);
