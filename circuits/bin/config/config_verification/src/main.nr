// This circuit verifies all the configuration parameters used in the secure configs
// Only re-run if parameters change (new deployment)

use lib::configs::secure::dkg::{
    // DKG structural parameters
    N as DKG_N, L as DKG_L,
    // DKG moduli
    QIS as DKG_QIS,
    // DKG derived values
    PLAINTEXT_MODULUS as DKG_PLAINTEXT_MODULUS, Q_MOD_T as DKG_Q_MOD_T,
    // Threshold moduli reference
    L_THRESHOLD, QIS_THRESHOLD,
    // Share encryption bounds
    SHARE_ENCRYPTION_T, SHARE_ENCRYPTION_K0IS, SHARE_ENCRYPTION_PK_BOUNDS,
    SHARE_ENCRYPTION_E0_BOUND, SHARE_ENCRYPTION_E1_BOUND, SHARE_ENCRYPTION_U_BOUND,
    SHARE_ENCRYPTION_R1_LOW_BOUNDS, SHARE_ENCRYPTION_R1_UP_BOUNDS, SHARE_ENCRYPTION_R2_BOUNDS,
    SHARE_ENCRYPTION_P1_BOUNDS, SHARE_ENCRYPTION_P2_BOUNDS, SHARE_ENCRYPTION_MSG_BOUND,
};

use lib::configs::secure::threshold::{
    // Structural parameters
    N as THRESHOLD_N, L as THRESHOLD_L, N_PARTIES, N_CIPHERTEXTS,
    // Moduli
    QIS as THRESHOLD_QIS,
    // Derived values
    PLAINTEXT_MODULUS as THRESHOLD_PLAINTEXT_MODULUS, Q_MOD_T as THRESHOLD_Q_MOD_T,
    Q_INVERSE_MOD_T as THRESHOLD_Q_INVERSE_MOD_T,
    // pk_generation bounds
    PK_GENERATION_EEK_BOUND, PK_GENERATION_SK_BOUND, PK_GENERATION_E_SM_BOUND,
    PK_GENERATION_R1_BOUNDS, PK_GENERATION_R2_BOUNDS, PK_GENERATION_B_ENC,
    // share_decryption bounds
    SHARE_DECRYPTION_R1_BOUNDS, SHARE_DECRYPTION_R2_BOUNDS,
    // user_data_encryption (Greco) bounds
    USER_DATA_ENCRYPTION_K0IS, USER_DATA_ENCRYPTION_PK_BOUNDS, USER_DATA_ENCRYPTION_E0_BOUND,
    USER_DATA_ENCRYPTION_E1_BOUND, USER_DATA_ENCRYPTION_U_BOUND, USER_DATA_ENCRYPTION_K1_LOW_BOUND,
    USER_DATA_ENCRYPTION_K1_UP_BOUND, USER_DATA_ENCRYPTION_R1_LOW_BOUNDS,
    USER_DATA_ENCRYPTION_R1_UP_BOUNDS, USER_DATA_ENCRYPTION_R2_BOUNDS,
    USER_DATA_ENCRYPTION_P1_BOUNDS, USER_DATA_ENCRYPTION_P2_BOUNDS,
};

use lib::math::modulo::U128::ModU128;

fn main() {
    // DKG (BFV) Verification
    verify_dkg_derived_values();
    verify_dkg_bounds();

    // Threshold (TRBFV) Verification
    verify_threshold_derived_values();
    verify_threshold_bounds();

    // user_data_encryption Verification
    verify_greco_bounds();

    // Cross-Config Consistency
    verify_cross_config_consistency();
}

//DKG Derived Values

fn verify_dkg_derived_values() {
    verify_dkg_q_mod_t();
}

// Verifies DKG: Q_MOD_T = (product of QIS) mod t
fn verify_dkg_q_mod_t() {
    let t = DKG_PLAINTEXT_MODULUS;
    let m = ModU128::new(t);

    let mut product: Field = 1;
    for i in 0..DKG_L {
        product = m.mul_mod(product, DKG_QIS[i]);
    }

    assert(product == DKG_Q_MOD_T, "DKG Q_MOD_T verification failed");
}

//DKG Bounds

// Verifies share_encryption bounds (Circuit 3)
fn verify_dkg_bounds() {
    let n: u128 = DKG_N as u128;
    let t: u128 = SHARE_ENCRYPTION_T as u128;
    let u_bound: u128 = SHARE_ENCRYPTION_U_BOUND as u128;
    let e0_bound: u128 = SHARE_ENCRYPTION_E0_BOUND as u128;
    let e1_bound: u128 = SHARE_ENCRYPTION_E1_BOUND as u128;

    // u_bound = 1 (ternary)
    assert(SHARE_ENCRYPTION_U_BOUND == 1, "SHARE_ENCRYPTION_U_BOUND should be 1 for ternary");

    // e0_bound = 20
    assert(SHARE_ENCRYPTION_E0_BOUND == 20, "SHARE_ENCRYPTION_E0_BOUND should be 20");

    // e1_bound = 20
    assert(SHARE_ENCRYPTION_E1_BOUND == 20, "SHARE_ENCRYPTION_E1_BOUND should be 20");

    // msg_bound = t - 1
    assert(SHARE_ENCRYPTION_MSG_BOUND as u128 == t - 1, "SHARE_ENCRYPTION_MSG_BOUND mismatch");

    // ptxt bounds for r1 calculation
    let ptxt_up_bound: u128 = (t - 1) / 2;

    // Check if t is even or odd for ptxt_low calculation
    let t_is_even: bool = (t % 2) == 0;

    for i in 0..DKG_L {
        let qi: u128 = DKG_QIS[i] as u128;
        let qi_bound: u128 = (qi - 1) / 2;
        let k0i: u128 = SHARE_ENCRYPTION_K0IS[i] as u128;

        // pk_bounds[i] = (q_i - 1) / 2
        assert(
            SHARE_ENCRYPTION_PK_BOUNDS[i] as u128 == qi_bound,
            "SHARE_ENCRYPTION_PK_BOUNDS mismatch",
        );

        // r2_bounds[i] = (q_i - 1) / 2
        assert(
            SHARE_ENCRYPTION_R2_BOUNDS[i] as u128 == qi_bound,
            "SHARE_ENCRYPTION_R2_BOUNDS mismatch",
        );

        // p2_bounds[i] = (q_i - 1) / 2
        assert(
            SHARE_ENCRYPTION_P2_BOUNDS[i] as u128 == qi_bound,
            "SHARE_ENCRYPTION_P2_BOUNDS mismatch",
        );

        // p1_bounds[i] = ((n * u_bound + 2) * qi_bound + e1_bound) / q_i
        let expected_p1: u128 = ((n * u_bound + 2) * qi_bound + e1_bound) / qi;
        assert(
            SHARE_ENCRYPTION_P1_BOUNDS[i] as u128 == expected_p1,
            "SHARE_ENCRYPTION_P1_BOUNDS mismatch",
        );

        // k0is[i] verification: k0i * t = -1 mod q_i
        let m = ModU128::new(DKG_QIS[i]);
        let product = m.mul_mod(SHARE_ENCRYPTION_K0IS[i], SHARE_ENCRYPTION_T);
        let neg_one = DKG_QIS[i] - 1;
        assert(product == neg_one, "SHARE_ENCRYPTION_K0IS mismatch");

        // e0_bound mod qi for r1 calculation
        let e0_bound_i: u128 = e0_bound % qi;

        // r1_up_bounds[i] = (ptxt_up * k0i + ((n * u + 2) * qi_bound + e0_i)) / q_i
        let expected_r1_up: u128 =
            (ptxt_up_bound * k0i + ((n * u_bound + 2) * qi_bound + e0_bound_i)) / qi;
        assert(
            SHARE_ENCRYPTION_R1_UP_BOUNDS[i] as u128 == expected_r1_up,
            "SHARE_ENCRYPTION_R1_UP_BOUNDS mismatch",
        );

        // r1_low_bounds[i] depends on whether t is even or odd
        let expected_r1_low: u128 = if t_is_even {
            ((ptxt_up_bound + 1) * k0i + ((n * u_bound + 2) * qi_bound + e0_bound_i)) / qi
        } else {
            expected_r1_up
        };
        assert(
            SHARE_ENCRYPTION_R1_LOW_BOUNDS[i] as u128 == expected_r1_low,
            "SHARE_ENCRYPTION_R1_LOW_BOUNDS mismatch",
        );
    }
}

//Threshold Derived Values

fn verify_threshold_derived_values() {
    verify_threshold_q_mod_t();
    verify_threshold_q_inverse_mod_t();
}

// Verifies Threshold: Q_MOD_T = (product of QIS) mod t
fn verify_threshold_q_mod_t() {
    let t = THRESHOLD_PLAINTEXT_MODULUS;
    let m = ModU128::new(t);

    let mut product: Field = 1;
    for i in 0..THRESHOLD_L {
        product = m.mul_mod(product, THRESHOLD_QIS[i]);
    }

    assert(product == THRESHOLD_Q_MOD_T, "Threshold Q_MOD_T verification failed");
}

// Verifies Threshold: Q * Q_INVERSE_MOD_T = 1 mod t
fn verify_threshold_q_inverse_mod_t() {
    let t = THRESHOLD_PLAINTEXT_MODULUS;
    let m = ModU128::new(t);

    let product = m.mul_mod(THRESHOLD_Q_MOD_T, THRESHOLD_Q_INVERSE_MOD_T);

    assert(product == 1, "Threshold Q_INVERSE_MOD_T verification failed");
}

//Threshold Bounds

fn verify_threshold_bounds() {
    verify_pk_generation_bounds();
    verify_share_decryption_bounds();
}

// Verifies pk_generation bounds (Circuit 1)
fn verify_pk_generation_bounds() {
    let n: u128 = THRESHOLD_N as u128;
    let eek: u128 = PK_GENERATION_EEK_BOUND as u128;

    // sk_bound = 1 (ternary)
    assert(PK_GENERATION_SK_BOUND == 1, "PK_GENERATION_SK_BOUND should be 1 for ternary");

    // eek_bound = 20
    assert(PK_GENERATION_EEK_BOUND == 20, "PK_GENERATION_EEK_BOUND should be 20");

    // r2_bounds and r1_bounds for each modulus
    for i in 0..THRESHOLD_L {
        let qi: u128 = THRESHOLD_QIS[i] as u128;
        let qi_bound: u128 = (qi - 1) / 2;

        // r2_bounds[i] = (q_i - 1) / 2
        assert(PK_GENERATION_R2_BOUNDS[i] as u128 == qi_bound, "PK_GENERATION_R2_BOUNDS mismatch");

        // r1_bounds[i] = ((n * eek_bound + 2) * qi_bound + eek_bound) / q_i
        let expected_r1: u128 = ((n * eek + 2) * qi_bound + eek) / qi;
        assert(
            PK_GENERATION_R1_BOUNDS[i] as u128 == expected_r1,
            "PK_GENERATION_R1_BOUNDS mismatch",
        );
    }

    verify_e_sm_bound();
}

// Verifies e_sm_bound (smudging noise bound)
fn verify_e_sm_bound() {
    let n: Field = THRESHOLD_N as Field;
    let e_norm: Field = 20;
    let b_e: Field = 20;
    let sk_norm: Field = N_PARTIES as Field;
    let num_ciphertexts: Field = N_CIPHERTEXTS as Field;

    // b_fresh = N * e_norm + b_enc + N * b_e * sk_norm
    let b_fresh = n * e_norm + PK_GENERATION_B_ENC + n * b_e * sk_norm;

    // b_c = N_CIPHERTEXTS * (b_fresh + Q_MOD_T)
    let b_c = num_ciphertexts * (b_fresh + THRESHOLD_Q_MOD_T);

    // 2^80
    let two_pow_80: Field = 0x100000000000000000000;

    // e_sm_bound = 2^80 * b_c
    let expected_e_sm_bound = two_pow_80 * b_c;

    assert(expected_e_sm_bound == PK_GENERATION_E_SM_BOUND, "PK_GENERATION_E_SM_BOUND mismatch");
}

// Verifies share_decryption bounds (Circuit 6)
fn verify_share_decryption_bounds() {
    let n: u128 = THRESHOLD_N as u128;

    for i in 0..THRESHOLD_L {
        let qi: u128 = THRESHOLD_QIS[i] as u128;
        let qi_bound: u128 = (qi - 1) / 2;

        // r2_bounds[i] = (q_i - 1) / 2
        assert(
            SHARE_DECRYPTION_R2_BOUNDS[i] as u128 == qi_bound,
            "SHARE_DECRYPTION_R2_BOUNDS mismatch",
        );

        // r1_bounds[i] = (qi_bound * (qi_bound * n + 3) - qi_bound) / q_i
        let expected_r1: u128 = (qi_bound * (qi_bound * n + 3) - qi_bound) / qi;
        assert(
            SHARE_DECRYPTION_R1_BOUNDS[i] as u128 == expected_r1,
            "SHARE_DECRYPTION_R1_BOUNDS mismatch",
        );
    }
}

//user_data_encryption Bounds

// Verifies user_data_encryption (Greco) bounds
// Uses THRESHOLD parameters (L=4 moduli, t=100)
fn verify_greco_bounds() {
    let n: u128 = THRESHOLD_N as u128;
    let t: u128 = THRESHOLD_PLAINTEXT_MODULUS as u128;
    let u_bound: u128 = USER_DATA_ENCRYPTION_U_BOUND as u128;
    let e1_bound: u128 = USER_DATA_ENCRYPTION_E1_BOUND as u128;

    // u_bound = 1 (ternary)
    assert(USER_DATA_ENCRYPTION_U_BOUND == 1, "USER_DATA_ENCRYPTION_U_BOUND should be 1");

    // e1_bound = 20 (variance * 2)
    assert(USER_DATA_ENCRYPTION_E1_BOUND == 20, "USER_DATA_ENCRYPTION_E1_BOUND should be 20");

    // e0_bound = sqrt(3 * error1_variance) = PK_GENERATION_B_ENC (same parameter set)
    assert(
        USER_DATA_ENCRYPTION_E0_BOUND == PK_GENERATION_B_ENC,
        "USER_DATA_ENCRYPTION_E0_BOUND should equal PK_GENERATION_B_ENC",
    );

    // k1_up_bound = (t - 1) / 2 = 49
    let expected_k1_up: u128 = (t - 1) / 2;
    assert(
        USER_DATA_ENCRYPTION_K1_UP_BOUND as u128 == expected_k1_up,
        "USER_DATA_ENCRYPTION_K1_UP_BOUND mismatch",
    );

    // k1_low_bound = (t - 1) / 2 + 1 = 50 (for even t)
    let t_is_even: bool = (t % 2) == 0;
    let expected_k1_low: u128 = if t_is_even {
        expected_k1_up + 1
    } else {
        expected_k1_up
    };
    assert(
        USER_DATA_ENCRYPTION_K1_LOW_BOUND as u128 == expected_k1_low,
        "USER_DATA_ENCRYPTION_K1_LOW_BOUND mismatch",
    );

    // ptxt_up_bound for r1 calculations
    let ptxt_up_bound: u128 = (t - 1) / 2;

    for i in 0..THRESHOLD_L {
        let qi: u128 = THRESHOLD_QIS[i] as u128;
        let qi_bound: u128 = (qi - 1) / 2;
        let k0i: u128 = USER_DATA_ENCRYPTION_K0IS[i] as u128;

        // pk_bounds[i] = (q_i - 1) / 2
        assert(
            USER_DATA_ENCRYPTION_PK_BOUNDS[i] as u128 == qi_bound,
            "USER_DATA_ENCRYPTION_PK_BOUNDS mismatch",
        );

        // r2_bounds[i] = (q_i - 1) / 2
        assert(
            USER_DATA_ENCRYPTION_R2_BOUNDS[i] as u128 == qi_bound,
            "USER_DATA_ENCRYPTION_R2_BOUNDS mismatch",
        );

        // p2_bounds[i] = (q_i - 1) / 2
        assert(
            USER_DATA_ENCRYPTION_P2_BOUNDS[i] as u128 == qi_bound,
            "USER_DATA_ENCRYPTION_P2_BOUNDS mismatch",
        );

        // p1_bounds[i] = ((n * u_bound + 2) * qi_bound + e1_bound) / q_i
        let expected_p1: u128 = ((n * u_bound + 2) * qi_bound + e1_bound) / qi;
        assert(
            USER_DATA_ENCRYPTION_P1_BOUNDS[i] as u128 == expected_p1,
            "USER_DATA_ENCRYPTION_P1_BOUNDS mismatch",
        );

        // k0is[i] verification: k0i * t = -1 mod q_i
        let m = ModU128::new(THRESHOLD_QIS[i]);
        let product = m.mul_mod(USER_DATA_ENCRYPTION_K0IS[i], THRESHOLD_PLAINTEXT_MODULUS);
        let neg_one = THRESHOLD_QIS[i] - 1;
        assert(product == neg_one, "USER_DATA_ENCRYPTION_K0IS mismatch");

        // e0_bound mod qi for r1 calculation
        // Use Field arithmetic since e0_bound is ~107 bits
        let e0_bound_i: u128 = (USER_DATA_ENCRYPTION_E0_BOUND as u128) % qi;

        // r1_up_bounds[i] = (ptxt_up * k0i + ((n * u + 2) * qi_bound + e0_i)) / q_i
        let expected_r1_up: u128 =
            (ptxt_up_bound * k0i + ((n * u_bound + 2) * qi_bound + e0_bound_i)) / qi;
        assert(
            USER_DATA_ENCRYPTION_R1_UP_BOUNDS[i] as u128 == expected_r1_up,
            "USER_DATA_ENCRYPTION_R1_UP_BOUNDS mismatch",
        );

        // r1_low_bounds[i] for even t
        let expected_r1_low: u128 = if t_is_even {
            ((ptxt_up_bound + 1) * k0i + ((n * u_bound + 2) * qi_bound + e0_bound_i)) / qi
        } else {
            expected_r1_up
        };
        assert(
            USER_DATA_ENCRYPTION_R1_LOW_BOUNDS[i] as u128 == expected_r1_low,
            "USER_DATA_ENCRYPTION_R1_LOW_BOUNDS mismatch",
        );
    }
}

//Cross-Config Consistency

fn verify_cross_config_consistency() {
    // N is consistent
    assert(DKG_N == THRESHOLD_N, "N mismatch between DKG and Threshold configs");

    // L_THRESHOLD in dkg.nr matches L in threshold.nr
    assert(L_THRESHOLD == THRESHOLD_L, "L_THRESHOLD mismatch between config files");

    // QIS_THRESHOLD in dkg.nr matches QIS in threshold.nr
    for i in 0..THRESHOLD_L {
        assert(
            QIS_THRESHOLD[i] == THRESHOLD_QIS[i],
            "QIS_THRESHOLD mismatch between dkg.nr and threshold.nr",
        );
    }
}
