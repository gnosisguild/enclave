// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::core::threshold::pk_generation::Configs as PkGenerationConfigs;

// Global configs for Threshold Public Key Generation circuit
pub global N: u32 = 512;
pub global L: u32 = 2;
pub global QIS: [Field; L] = [68719403009, 68719230977];

/************************************
-------------------------------------
pk_generation (CIRCUIT 1 - PUBLIC KEY THRESHOLD BFV)
-------------------------------------
************************************/

pub global PK_GENERATION_BIT_EEK: u32 = 5;
pub global PK_GENERATION_BIT_SK: u32 = 1;
pub global PK_GENERATION_BIT_E_SM: u32 = 17;
pub global PK_GENERATION_BIT_R1: u32 = 13;
pub global PK_GENERATION_BIT_R2: u32 = 35;
pub global PK_GENERATION_BIT_PK: u32 = 35;

pub global PK_GENERATION_EEK_BOUND: Field = 20;
pub global PK_GENERATION_SK_BOUND: Field = 1;
pub global PK_GENERATION_E_SM_BOUND: Field = 123072;
pub global PK_GENERATION_R1_BOUNDS: [Field; L] = [5120, 5120];
pub global PK_GENERATION_R2_BOUNDS: [Field; L] = [34359701504, 34359615488];

pub global PK_GENERATION_CONFIGS: PkGenerationConfigs<N, L> = PkGenerationConfigs::new(
QIS,
PK_GENERATION_EEK_BOUND,
PK_GENERATION_SK_BOUND,
PK_GENERATION_E_SM_BOUND,
PK_GENERATION_R1_BOUNDS,
PK_GENERATION_R2_BOUNDS,
);
