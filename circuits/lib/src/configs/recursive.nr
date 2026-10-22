// SPDX-License-Identifier: LGPL-3.0-only

//! Shared public-field layouts for the synchronized-decryption trust tree.

use super::default::dkg::L_THRESHOLD;
use super::default::{H, MAX_MSG_NON_ZERO_COEFFS, N_PARTIES, T};

pub global C3_SLOTS: u32 = N_PARTIES * L_THRESHOLD;
pub global C3_FOLD_PREFIX_LEN: u32 = 5;
pub global C3_FOLD_PUBLIC_LEN: u32 = C3_FOLD_PREFIX_LEN + 4 * C3_SLOTS;
pub global C2_PUBLIC_LEN: u32 = 1 + N_PARTIES + N_PARTIES * L_THRESHOLD;
pub global C4_PUBLIC_LEN: u32 = H * L_THRESHOLD + H + 2;

pub global NODE_FOLD_PARAM_PUBLIC_LEN: u32 = 6;
pub global NF_PARTY_ID_IDX: u32 = 0;
pub global NF_C0_PK_IDX: u32 = 7;
pub global NF_C1_PK_IDX: u32 = 8;
pub global NF_C3_PK_BASE_IDX: u32 = 9;
pub global NF_C2A_BASE_IDX: u32 = NF_C3_PK_BASE_IDX + N_PARTIES;
pub global NF_C4A_BASE_IDX: u32 = NF_C2A_BASE_IDX + N_PARTIES * L_THRESHOLD;
pub global NF_SK_AGG_IDX: u32 = NF_C4A_BASE_IDX + H * L_THRESHOLD;
pub global NF_C2_KEY_BASE_IDX: u32 = NF_SK_AGG_IDX + 1;
pub global NF_C4_KEY_BASE_IDX: u32 = NF_C2_KEY_BASE_IDX + N_PARTIES;
pub global NODE_FOLD_PUBLIC_LEN: u32 = NF_C4_KEY_BASE_IDX + H;
pub global NODES_FOLD_PARAM_PUBLIC_LEN: u32 = 5;
pub global NODES_FOLD_PUBLIC_LEN: u32 = NODES_FOLD_PARAM_PUBLIC_LEN + H * NODE_FOLD_PUBLIC_LEN;

pub global DECRYPTOR_COUNT: u32 = T + 1;
pub global C6_INNER_PUBLIC_LEN: u32 = 7 + DECRYPTOR_COUNT + 2 * N_PARTIES;
pub global C6_FOLD_PREFIX_LEN: u32 = 7;
pub global C6_KEY_SLOTS: u32 = DECRYPTOR_COUNT * N_PARTIES;
pub global C6_FOLD_PUBLIC_LEN: u32 = C6_FOLD_PREFIX_LEN + 5 * DECRYPTOR_COUNT + 2 * C6_KEY_SLOTS;
pub global C7_PUBLIC_LEN: u32 = 2 * DECRYPTOR_COUNT + MAX_MSG_NON_ZERO_COEFFS + 1;
