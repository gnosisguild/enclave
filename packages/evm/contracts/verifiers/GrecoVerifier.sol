// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

contract GrecoVerifier {
    uint256 internal constant PROOF_LEN_CPTR = 0x44;
    uint256 internal constant PROOF_CPTR = 0x64;
    uint256 internal constant NUM_INSTANCE_CPTR = 0x1c04;
    uint256 internal constant INSTANCE_CPTR = 0x1c24;

    uint256 internal constant FIRST_QUOTIENT_X_CPTR = 0x09a4;
    uint256 internal constant LAST_QUOTIENT_X_CPTR = 0x0a24;

    uint256 internal constant VK_MPTR = 0x08a0;
    uint256 internal constant VK_DIGEST_MPTR = 0x08a0;
    uint256 internal constant NUM_INSTANCES_MPTR = 0x08c0;
    uint256 internal constant K_MPTR = 0x08e0;
    uint256 internal constant N_INV_MPTR = 0x0900;
    uint256 internal constant OMEGA_MPTR = 0x0920;
    uint256 internal constant OMEGA_INV_MPTR = 0x0940;
    uint256 internal constant OMEGA_INV_TO_L_MPTR = 0x0960;
    uint256 internal constant HAS_ACCUMULATOR_MPTR = 0x0980;
    uint256 internal constant ACC_OFFSET_MPTR = 0x09a0;
    uint256 internal constant NUM_ACC_LIMBS_MPTR = 0x09c0;
    uint256 internal constant NUM_ACC_LIMB_BITS_MPTR = 0x09e0;
    uint256 internal constant G1_X_MPTR = 0x0a00;
    uint256 internal constant G1_Y_MPTR = 0x0a20;
    uint256 internal constant G2_X_1_MPTR = 0x0a40;
    uint256 internal constant G2_X_2_MPTR = 0x0a60;
    uint256 internal constant G2_Y_1_MPTR = 0x0a80;
    uint256 internal constant G2_Y_2_MPTR = 0x0aa0;
    uint256 internal constant NEG_S_G2_X_1_MPTR = 0x0ac0;
    uint256 internal constant NEG_S_G2_X_2_MPTR = 0x0ae0;
    uint256 internal constant NEG_S_G2_Y_1_MPTR = 0x0b00;
    uint256 internal constant NEG_S_G2_Y_2_MPTR = 0x0b20;

    uint256 internal constant CHALLENGE_MPTR = 0x1400;

    uint256 internal constant THETA_MPTR = 0x1420;
    uint256 internal constant BETA_MPTR = 0x1440;
    uint256 internal constant GAMMA_MPTR = 0x1460;
    uint256 internal constant Y_MPTR = 0x1480;
    uint256 internal constant X_MPTR = 0x14a0;
    uint256 internal constant ZETA_MPTR = 0x14c0;
    uint256 internal constant NU_MPTR = 0x14e0;
    uint256 internal constant MU_MPTR = 0x1500;

    uint256 internal constant ACC_LHS_X_MPTR = 0x1520;
    uint256 internal constant ACC_LHS_Y_MPTR = 0x1540;
    uint256 internal constant ACC_RHS_X_MPTR = 0x1560;
    uint256 internal constant ACC_RHS_Y_MPTR = 0x1580;
    uint256 internal constant X_N_MPTR = 0x15a0;
    uint256 internal constant X_N_MINUS_1_INV_MPTR = 0x15c0;
    uint256 internal constant L_LAST_MPTR = 0x15e0;
    uint256 internal constant L_BLIND_MPTR = 0x1600;
    uint256 internal constant L_0_MPTR = 0x1620;
    uint256 internal constant INSTANCE_EVAL_MPTR = 0x1640;
    uint256 internal constant QUOTIENT_EVAL_MPTR = 0x1660;
    uint256 internal constant QUOTIENT_X_MPTR = 0x1680;
    uint256 internal constant QUOTIENT_Y_MPTR = 0x16a0;
    uint256 internal constant G1_SCALAR_MPTR = 0x16c0;
    uint256 internal constant PAIRING_LHS_X_MPTR = 0x16e0;
    uint256 internal constant PAIRING_LHS_Y_MPTR = 0x1700;
    uint256 internal constant PAIRING_RHS_X_MPTR = 0x1720;
    uint256 internal constant PAIRING_RHS_Y_MPTR = 0x1740;

    function verifyProof(
        bytes calldata proof,
        uint256[] calldata instances
    ) public view returns (bool) {
        assembly ("memory-safe") {
            // Read EC point (x, y) at (proof_cptr, proof_cptr + 0x20),
            // and check if the point is on affine plane,
            // and store them in (hash_mptr, hash_mptr + 0x20).
            // Return updated (success, proof_cptr, hash_mptr).
            function read_ec_point(success, proof_cptr, hash_mptr, q)
                -> ret0, ret1, ret2
            {
                let x := calldataload(proof_cptr)
                let y := calldataload(add(proof_cptr, 0x20))
                ret0 := and(success, lt(x, q))
                ret0 := and(ret0, lt(y, q))
                ret0 := and(
                    ret0,
                    eq(
                        mulmod(y, y, q),
                        addmod(mulmod(x, mulmod(x, x, q), q), 3, q)
                    )
                )
                mstore(hash_mptr, x)
                mstore(add(hash_mptr, 0x20), y)
                ret1 := add(proof_cptr, 0x40)
                ret2 := add(hash_mptr, 0x40)
            }

            // Squeeze challenge by keccak256(memory[0..hash_mptr]),
            // and store hash mod r as challenge in challenge_mptr,
            // and push back hash in 0x00 as the first input for next squeeze.
            // Return updated (challenge_mptr, hash_mptr).
            function squeeze_challenge(challenge_mptr, hash_mptr, r)
                -> ret0, ret1
            {
                let hash := keccak256(0x00, hash_mptr)
                mstore(challenge_mptr, mod(hash, r))
                mstore(0x00, hash)
                ret0 := add(challenge_mptr, 0x20)
                ret1 := 0x20
            }

            // Squeeze challenge without absorbing new input from calldata,
            // by putting an extra 0x01 in memory[0x20] and squeeze by keccak256(memory[0..21]),
            // and store hash mod r as challenge in challenge_mptr,
            // and push back hash in 0x00 as the first input for next squeeze.
            // Return updated (challenge_mptr).
            function squeeze_challenge_cont(challenge_mptr, r) -> ret {
                mstore8(0x20, 0x01)
                let hash := keccak256(0x00, 0x21)
                mstore(challenge_mptr, mod(hash, r))
                mstore(0x00, hash)
                ret := add(challenge_mptr, 0x20)
            }

            // Batch invert values in memory[mptr_start..mptr_end] in place.
            // Return updated (success).
            function batch_invert(success, mptr_start, mptr_end, r) -> ret {
                let gp_mptr := mptr_end
                let gp := mload(mptr_start)
                let mptr := add(mptr_start, 0x20)
                for {

                } lt(mptr, sub(mptr_end, 0x20)) {

                } {
                    gp := mulmod(gp, mload(mptr), r)
                    mstore(gp_mptr, gp)
                    mptr := add(mptr, 0x20)
                    gp_mptr := add(gp_mptr, 0x20)
                }
                gp := mulmod(gp, mload(mptr), r)

                mstore(gp_mptr, 0x20)
                mstore(add(gp_mptr, 0x20), 0x20)
                mstore(add(gp_mptr, 0x40), 0x20)
                mstore(add(gp_mptr, 0x60), gp)
                mstore(add(gp_mptr, 0x80), sub(r, 2))
                mstore(add(gp_mptr, 0xa0), r)
                ret := and(
                    success,
                    staticcall(gas(), 0x05, gp_mptr, 0xc0, gp_mptr, 0x20)
                )
                let all_inv := mload(gp_mptr)

                let first_mptr := mptr_start
                let second_mptr := add(first_mptr, 0x20)
                gp_mptr := sub(gp_mptr, 0x20)
                for {

                } lt(second_mptr, mptr) {

                } {
                    let inv := mulmod(all_inv, mload(gp_mptr), r)
                    all_inv := mulmod(all_inv, mload(mptr), r)
                    mstore(mptr, inv)
                    mptr := sub(mptr, 0x20)
                    gp_mptr := sub(gp_mptr, 0x20)
                }
                let inv_first := mulmod(all_inv, mload(second_mptr), r)
                let inv_second := mulmod(all_inv, mload(first_mptr), r)
                mstore(first_mptr, inv_first)
                mstore(second_mptr, inv_second)
            }

            // Add (x, y) into point at (0x00, 0x20).
            // Return updated (success).
            function ec_add_acc(success, x, y) -> ret {
                mstore(0x40, x)
                mstore(0x60, y)
                ret := and(
                    success,
                    staticcall(gas(), 0x06, 0x00, 0x80, 0x00, 0x40)
                )
            }

            // Scale point at (0x00, 0x20) by scalar.
            function ec_mul_acc(success, scalar) -> ret {
                mstore(0x40, scalar)
                ret := and(
                    success,
                    staticcall(gas(), 0x07, 0x00, 0x60, 0x00, 0x40)
                )
            }

            // Add (x, y) into point at (0x80, 0xa0).
            // Return updated (success).
            function ec_add_tmp(success, x, y) -> ret {
                mstore(0xc0, x)
                mstore(0xe0, y)
                ret := and(
                    success,
                    staticcall(gas(), 0x06, 0x80, 0x80, 0x80, 0x40)
                )
            }

            // Scale point at (0x80, 0xa0) by scalar.
            // Return updated (success).
            function ec_mul_tmp(success, scalar) -> ret {
                mstore(0xc0, scalar)
                ret := and(
                    success,
                    staticcall(gas(), 0x07, 0x80, 0x60, 0x80, 0x40)
                )
            }

            // Perform pairing check.
            // Return updated (success).
            function ec_pairing(success, lhs_x, lhs_y, rhs_x, rhs_y) -> ret {
                mstore(0x00, lhs_x)
                mstore(0x20, lhs_y)
                mstore(0x40, mload(G2_X_1_MPTR))
                mstore(0x60, mload(G2_X_2_MPTR))
                mstore(0x80, mload(G2_Y_1_MPTR))
                mstore(0xa0, mload(G2_Y_2_MPTR))
                mstore(0xc0, rhs_x)
                mstore(0xe0, rhs_y)
                mstore(0x100, mload(NEG_S_G2_X_1_MPTR))
                mstore(0x120, mload(NEG_S_G2_X_2_MPTR))
                mstore(0x140, mload(NEG_S_G2_Y_1_MPTR))
                mstore(0x160, mload(NEG_S_G2_Y_2_MPTR))
                ret := and(
                    success,
                    staticcall(gas(), 0x08, 0x00, 0x180, 0x00, 0x20)
                )
                ret := and(ret, mload(0x00))
            }

            // Modulus
            let
                q
            := 21888242871839275222246405745257275088696311157297823662689037894645226208583 // BN254 base field
            let
                r
            := 21888242871839275222246405745257275088548364400416034343698204186575808495617 // BN254 scalar field

            // Initialize success as true
            let success := true

            {
                // Load vk_digest and num_instances of vk into memory
                mstore(
                    0x08a0,
                    0x16b9101a2bd75ecdfb9942f961a6d86036fd8b0ad0d5128d350357d94b916d47
                ) // vk_digest
                mstore(
                    0x08c0,
                    0x0000000000000000000000000000000000000000000000000000000000000004
                ) // num_instances

                // Check valid length of proof
                success := and(
                    success,
                    eq(0x1ba0, calldataload(PROOF_LEN_CPTR))
                )

                // Check valid length of instances
                let num_instances := mload(NUM_INSTANCES_MPTR)
                success := and(
                    success,
                    eq(num_instances, calldataload(NUM_INSTANCE_CPTR))
                )

                // Absorb vk diegst
                mstore(0x00, mload(VK_DIGEST_MPTR))

                // Read instances and witness commitments and generate challenges
                let hash_mptr := 0x20
                let instance_cptr := INSTANCE_CPTR
                for {
                    let instance_cptr_end := add(
                        instance_cptr,
                        mul(0x20, num_instances)
                    )
                } lt(instance_cptr, instance_cptr_end) {

                } {
                    let instance := calldataload(instance_cptr)
                    success := and(success, lt(instance, r))
                    mstore(hash_mptr, instance)
                    instance_cptr := add(instance_cptr, 0x20)
                    hash_mptr := add(hash_mptr, 0x20)
                }

                let proof_cptr := PROOF_CPTR
                let challenge_mptr := CHALLENGE_MPTR

                // Phase 1
                for {
                    let proof_cptr_end := add(proof_cptr, 0x40)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    success, proof_cptr, hash_mptr := read_ec_point(
                        success,
                        proof_cptr,
                        hash_mptr,
                        q
                    )
                }

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                )

                // Phase 2
                for {
                    let proof_cptr_end := add(proof_cptr, 0x0400)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    success, proof_cptr, hash_mptr := read_ec_point(
                        success,
                        proof_cptr,
                        hash_mptr,
                        q
                    )
                }

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                )

                // Phase 3
                for {
                    let proof_cptr_end := add(proof_cptr, 0x0180)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    success, proof_cptr, hash_mptr := read_ec_point(
                        success,
                        proof_cptr,
                        hash_mptr,
                        q
                    )
                }

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                )
                challenge_mptr := squeeze_challenge_cont(challenge_mptr, r)

                // Phase 4
                for {
                    let proof_cptr_end := add(proof_cptr, 0x0380)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    success, proof_cptr, hash_mptr := read_ec_point(
                        success,
                        proof_cptr,
                        hash_mptr,
                        q
                    )
                }

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                )

                // Phase 5
                for {
                    let proof_cptr_end := add(proof_cptr, 0xc0)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    success, proof_cptr, hash_mptr := read_ec_point(
                        success,
                        proof_cptr,
                        hash_mptr,
                        q
                    )
                }

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                )

                // Read evaluations
                for {
                    let proof_cptr_end := add(proof_cptr, 0x1120)
                } lt(proof_cptr, proof_cptr_end) {

                } {
                    let eval := calldataload(proof_cptr)
                    success := and(success, lt(eval, r))
                    mstore(hash_mptr, eval)
                    proof_cptr := add(proof_cptr, 0x20)
                    hash_mptr := add(hash_mptr, 0x20)
                }

                // Read batch opening proof and generate challenges
                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                ) // zeta
                challenge_mptr := squeeze_challenge_cont(challenge_mptr, r) // nu

                success, proof_cptr, hash_mptr := read_ec_point(
                    success,
                    proof_cptr,
                    hash_mptr,
                    q
                ) // W

                challenge_mptr, hash_mptr := squeeze_challenge(
                    challenge_mptr,
                    hash_mptr,
                    r
                ) // mu

                success, proof_cptr, hash_mptr := read_ec_point(
                    success,
                    proof_cptr,
                    hash_mptr,
                    q
                ) // W'

                // Load full vk into memory
                mstore(
                    0x08a0,
                    0x16b9101a2bd75ecdfb9942f961a6d86036fd8b0ad0d5128d350357d94b916d47
                ) // vk_digest
                mstore(
                    0x08c0,
                    0x0000000000000000000000000000000000000000000000000000000000000004
                ) // num_instances
                mstore(
                    0x08e0,
                    0x000000000000000000000000000000000000000000000000000000000000000f
                ) // k
                mstore(
                    0x0900,
                    0x3063edaa444bddc677fcd515f614555a777997e0a9287d1e62bf6dd004d82001
                ) // n_inv
                mstore(
                    0x0920,
                    0x2b7ddfe4383c8d806530b94d3120ce6fcb511871e4d44a65f0acd0b96a8a942e
                ) // omega
                mstore(
                    0x0940,
                    0x1f67bc4574eaef5e630a13c710221a3e3d491e59fddabaf321e56f3ca8d91624
                ) // omega_inv
                mstore(
                    0x0960,
                    0x19351bb40ad3ea92ee3c154c83b173944b455dbebea9370acd919c3ba19d6546
                ) // omega_inv_to_l
                mstore(
                    0x0980,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // has_accumulator
                mstore(
                    0x09a0,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // acc_offset
                mstore(
                    0x09c0,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // num_acc_limbs
                mstore(
                    0x09e0,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // num_acc_limb_bits
                mstore(
                    0x0a00,
                    0x0000000000000000000000000000000000000000000000000000000000000001
                ) // g1_x
                mstore(
                    0x0a20,
                    0x0000000000000000000000000000000000000000000000000000000000000002
                ) // g1_y
                mstore(
                    0x0a40,
                    0x198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2
                ) // g2_x_1
                mstore(
                    0x0a60,
                    0x1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed
                ) // g2_x_2
                mstore(
                    0x0a80,
                    0x090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b
                ) // g2_y_1
                mstore(
                    0x0aa0,
                    0x12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa
                ) // g2_y_2
                mstore(
                    0x0ac0,
                    0x0181624e80f3d6ae28df7e01eaeab1c0e919877a3b8a6b7fbc69a6817d596ea2
                ) // neg_s_g2_x_1
                mstore(
                    0x0ae0,
                    0x1783d30dcb12d259bb89098addf6280fa4b653be7a152542a28f7b926e27e648
                ) // neg_s_g2_x_2
                mstore(
                    0x0b00,
                    0x00ae44489d41a0d179e2dfdc03bddd883b7109f8b6ae316a59e815c1a6b35304
                ) // neg_s_g2_y_1
                mstore(
                    0x0b20,
                    0x0b2147ab62a386bd63e6de1522109b8c9588ab466f5aadfde8c41ca3749423ee
                ) // neg_s_g2_y_2
                mstore(
                    0x0b40,
                    0x1a01290b3442ca346a9f4fc35ab974ae7e7bcd0e7f5b047c693c09847201c538
                ) // fixed_comms[0].x
                mstore(
                    0x0b60,
                    0x1fad77828508076665623e3bcf79b31f883fa9bb14cfc8f28eb600bb6f7109c8
                ) // fixed_comms[0].y
                mstore(
                    0x0b80,
                    0x1b38c810b84828707ecf383612a0bbd12c707e3c4daffb0bb12cef8e0e1e4bf4
                ) // fixed_comms[1].x
                mstore(
                    0x0ba0,
                    0x1e3bbe6ce15073b1a30b7ed979b96029218bcb560683faed916615ee2f83ffe6
                ) // fixed_comms[1].y
                mstore(
                    0x0bc0,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // fixed_comms[2].x
                mstore(
                    0x0be0,
                    0x0000000000000000000000000000000000000000000000000000000000000000
                ) // fixed_comms[2].y
                mstore(
                    0x0c00,
                    0x1a4ba068a404a507e9a23b893fb4ff4bfffbcf34977f2b6c5b945077a653a0d8
                ) // fixed_comms[3].x
                mstore(
                    0x0c20,
                    0x083f81534756d2c55e3325c9fd888da2338025e692bce486efe72fe47857a24d
                ) // fixed_comms[3].y
                mstore(
                    0x0c40,
                    0x1eaf495f4b5707f1741c01bd07f31687f8a34d3c0f88d1e550328bd83d08adf7
                ) // fixed_comms[4].x
                mstore(
                    0x0c60,
                    0x218c5cab9a35dcc22dd433e73ccb3d467ceab37d22551ccd7ccdad9283b0d903
                ) // fixed_comms[4].y
                mstore(
                    0x0c80,
                    0x0d30908653f4158b576fc43745bbd6b9dd65670611e99eb8b739aec264a3d69d
                ) // fixed_comms[5].x
                mstore(
                    0x0ca0,
                    0x1b75cc2a05a6650b1dadd556e96bee1211eb79014679a9a75da0437769689b3d
                ) // fixed_comms[5].y
                mstore(
                    0x0cc0,
                    0x12cfac57acf3fec6562d565f562097336778abafe4afe167dad0debf0e0e7560
                ) // fixed_comms[6].x
                mstore(
                    0x0ce0,
                    0x0e57a11bd02f858acc8721d768c9793594810a16c6141beab9380d2a39fe66b1
                ) // fixed_comms[6].y
                mstore(
                    0x0d00,
                    0x0811b77872fa36129fa64c790e67f7b320266172c17f359dae9f47b57a5b9c36
                ) // fixed_comms[7].x
                mstore(
                    0x0d20,
                    0x0fa0cc146c7b55a28891cdfde8d280830c71a78ee074c71d4131fcbc24e97f2a
                ) // fixed_comms[7].y
                mstore(
                    0x0d40,
                    0x173ec3a1ed294039a515cfcbdd37ec7ec164c388d229c00390bfb03ae08372f0
                ) // fixed_comms[8].x
                mstore(
                    0x0d60,
                    0x29df13a51d2379b9199edadb33bcda1fd2383ff71e3ed439b8a49840f7fe141d
                ) // fixed_comms[8].y
                mstore(
                    0x0d80,
                    0x0fd46ca0b20bbdf47ef2166b65f8c08e2ac26acf4fdebffd9d509f17e0e91117
                ) // fixed_comms[9].x
                mstore(
                    0x0da0,
                    0x0cbc98a79279fd75ca43a72de207b52bfc7f5dc5b361c7894ebde411e21cac6d
                ) // fixed_comms[9].y
                mstore(
                    0x0dc0,
                    0x0beaecc2365e102086443ee6251e2e1d99d725e5ba4025ca828c00aa259b501c
                ) // fixed_comms[10].x
                mstore(
                    0x0de0,
                    0x059245bba8759de018413af3bfb869744855f5c3c4450887de635a216bea2451
                ) // fixed_comms[10].y
                mstore(
                    0x0e00,
                    0x1a4e05b053665a994035a4b20cb6e4547a0c5556fb81e7659989cd7f738bbff9
                ) // fixed_comms[11].x
                mstore(
                    0x0e20,
                    0x0fa2a0b43ce11a9a8f0aafd525990babe6e764c30711a9231f3008e7a0aa49cc
                ) // fixed_comms[11].y
                mstore(
                    0x0e40,
                    0x09ba2d97d6e17455a7b9f2b54db3b02f466ecdb85e2812de16f20a5f5d191136
                ) // fixed_comms[12].x
                mstore(
                    0x0e60,
                    0x2313da7d18cba08f03aa7f4ea923fe876abc7d3f7ab702552c93b863f08106f7
                ) // fixed_comms[12].y
                mstore(
                    0x0e80,
                    0x0cb2855d0a7301777accc775c735f81f0d14ef6db568669ea0f551ea55aca6ae
                ) // fixed_comms[13].x
                mstore(
                    0x0ea0,
                    0x20f5514019f450db8887e9f4c1ea44681a7936620e00facdfc8a5a9c774eaeaf
                ) // fixed_comms[13].y
                mstore(
                    0x0ec0,
                    0x065575608c3b66cc8b00cc955dc62da524bda7ce48676432a6ef883bb57c5271
                ) // fixed_comms[14].x
                mstore(
                    0x0ee0,
                    0x2566d460f6e3a527010a0c8a9eb78ef0bd4a3657ab569d06d62147afd3e3265d
                ) // fixed_comms[14].y
                mstore(
                    0x0f00,
                    0x1425c6babc964d851a996cd1d570fad9d6e21bc7356a0a06553ea3694d9b696d
                ) // fixed_comms[15].x
                mstore(
                    0x0f20,
                    0x097da64aaa6b4e8c5368b2630be79e0f0b6d08e0e80a5840b24d5cda039f3281
                ) // fixed_comms[15].y
                mstore(
                    0x0f40,
                    0x04b531449d7327048164dfd2af23004d96add0aa9b77d690f61953aa3d6aeda3
                ) // permutation_comms[0].x
                mstore(
                    0x0f60,
                    0x2b903afe676ea26fc6694c0764070482d72de4fa78489dcf0b86dfbc01738fb6
                ) // permutation_comms[0].y
                mstore(
                    0x0f80,
                    0x02dcedaf9ab056b2c0933df9fe87d835d968353a635118ac8365f04e999d9db0
                ) // permutation_comms[1].x
                mstore(
                    0x0fa0,
                    0x0d0c5e181062dac9db91d0f5348a801b655de7a23dc6175755d7ee3b2c791aa2
                ) // permutation_comms[1].y
                mstore(
                    0x0fc0,
                    0x2f3331370b89391d028114667da7f002903dde46e28d3ddadac921c26b340f89
                ) // permutation_comms[2].x
                mstore(
                    0x0fe0,
                    0x141a9d20c6153b4c6ff19fd30163bba08cc180fa25452fedf5cff64d6c643b9e
                ) // permutation_comms[2].y
                mstore(
                    0x1000,
                    0x2aa78a1d933e24ec624ec008cc3a3865a8fe78f05341c9ff29924fd1c045c45e
                ) // permutation_comms[3].x
                mstore(
                    0x1020,
                    0x1ead0473575f90ee4a0486c6f4e06bab23f98f5d0127d820d897e7d06822a893
                ) // permutation_comms[3].y
                mstore(
                    0x1040,
                    0x0847f490d1860700802ad8c69ad158554001655de96f0997690f19308f8f7ae1
                ) // permutation_comms[4].x
                mstore(
                    0x1060,
                    0x1a74d977f48ca6b4bf1c8e8f9b78889ece8e44dae053b3c96591474cd622688e
                ) // permutation_comms[4].y
                mstore(
                    0x1080,
                    0x1f935f34df7162f58ee0338726d64081ff9a6c5c86b0e79e25b9cae7e934b313
                ) // permutation_comms[5].x
                mstore(
                    0x10a0,
                    0x0ea1bd950ce6934f6734b7399746304cfb22b40375e5c7030d1bb0eedbc795d7
                ) // permutation_comms[5].y
                mstore(
                    0x10c0,
                    0x0b90427085312fd4e17d481a08f0c33fcd59852e6bee7d9cd2941ae870bde00e
                ) // permutation_comms[6].x
                mstore(
                    0x10e0,
                    0x0600d425363347bf60d302b56aaba8ea45ff4f13d2f67811709de9cd322b8403
                ) // permutation_comms[6].y
                mstore(
                    0x1100,
                    0x2d4dbb2380f33ed016bf355f2fc7ca83291cf1c0ab94eaa4fba0d2055e13a00f
                ) // permutation_comms[7].x
                mstore(
                    0x1120,
                    0x0bc7628f773e07f63f7542e9dc26aa6428aa51a34c403d4eff6a984932062e57
                ) // permutation_comms[7].y
                mstore(
                    0x1140,
                    0x13f0d19da2d582fb858ba269733800b7125edbfaee7d1e9d1dd6ff1155690056
                ) // permutation_comms[8].x
                mstore(
                    0x1160,
                    0x2c5cefe62ed151c153c5c154d9398a8421fcfbfbddb902f69ddb0d50c1572901
                ) // permutation_comms[8].y
                mstore(
                    0x1180,
                    0x2e3d1aee4680523d37e73c139efb21559dda4734f0476e2f5c3d847abc5f76d7
                ) // permutation_comms[9].x
                mstore(
                    0x11a0,
                    0x06844c948aefd2249652d9103186e0d3ef53accec7840bd1580eeb36c8ec87d9
                ) // permutation_comms[9].y
                mstore(
                    0x11c0,
                    0x126f8a145739b462a96aba864e0264339d714d55aca6973175dc7fdf7aa34f96
                ) // permutation_comms[10].x
                mstore(
                    0x11e0,
                    0x0585976e21d090af363430dff686e94744222e08af213014b2f8c80fb4732219
                ) // permutation_comms[10].y
                mstore(
                    0x1200,
                    0x0153d1e93a37a22035b68a7e75ed999f1a74eed8928cac787a75454be4a9a2c4
                ) // permutation_comms[11].x
                mstore(
                    0x1220,
                    0x22e119742836ee29d9e47d84fc75d0e28585c98d8d0200c5c31a1886faa141d8
                ) // permutation_comms[11].y
                mstore(
                    0x1240,
                    0x29d044d3d2d9197c64513f419407fa584295edf323d13a8dfe29d925fbb1097e
                ) // permutation_comms[12].x
                mstore(
                    0x1260,
                    0x1c9e43b7516d48f13a68b9a71638d99d4ee6f8fb8e410cc62cc978fc1b79572a
                ) // permutation_comms[12].y
                mstore(
                    0x1280,
                    0x0cfe26eaadb39ef1a7d4de9ab0b09eb49c0d7f6e01e3c11a3799bc577d24d361
                ) // permutation_comms[13].x
                mstore(
                    0x12a0,
                    0x0d2c0ff1eb5d54b0cdf5b89820d13188d2e7aee877eb85c4ba773e1b7d0c880e
                ) // permutation_comms[13].y
                mstore(
                    0x12c0,
                    0x0f74079674a69306a459c59e4a101162d51c88cf0d0405035e3712d8213893b3
                ) // permutation_comms[14].x
                mstore(
                    0x12e0,
                    0x01888d436cdf8ff16a13b5eb58d6a6e5f3e995ebeea55131b6c03c311333f0c7
                ) // permutation_comms[14].y
                mstore(
                    0x1300,
                    0x2a0e3125594384c85a3d00be841199729d69bec6d2c00e4fc08cec027b714083
                ) // permutation_comms[15].x
                mstore(
                    0x1320,
                    0x0af66a07a81840c123d487f7d08706de281913fd0587c93306b58ecabb652db1
                ) // permutation_comms[15].y
                mstore(
                    0x1340,
                    0x220342bc2fd78c5f00f5f1a26db348371d35abf58254462ebe9986bed261d5a2
                ) // permutation_comms[16].x
                mstore(
                    0x1360,
                    0x000654957fa760f8409e658ce4bab13a95bccf1a11323df7ce2a29252f913e8d
                ) // permutation_comms[16].y
                mstore(
                    0x1380,
                    0x0f71c7b93a6db5ad321eecd0f7bcff0fc2c66d835d9a877a708bb6bd5204e6a5
                ) // permutation_comms[17].x
                mstore(
                    0x13a0,
                    0x2c4746e7db1ae04dcfa090aa6a9d1f4727fff6e7dfce588c7d7ed2caad85f69d
                ) // permutation_comms[17].y
                mstore(
                    0x13c0,
                    0x26b98bfb8647817bc8a1e35d0076310637c4394ed62d93e1a2abf29d0ebc7d48
                ) // permutation_comms[18].x
                mstore(
                    0x13e0,
                    0x1eb44011587ac9b74ded024895759af8c883b6f0452a63d1ca084e5f3dfe5f1b
                ) // permutation_comms[18].y

                // Read accumulator from instances
                if mload(HAS_ACCUMULATOR_MPTR) {
                    let num_limbs := mload(NUM_ACC_LIMBS_MPTR)
                    let num_limb_bits := mload(NUM_ACC_LIMB_BITS_MPTR)

                    let cptr := add(
                        INSTANCE_CPTR,
                        mul(mload(ACC_OFFSET_MPTR), 0x20)
                    )
                    let lhs_y_off := mul(num_limbs, 0x20)
                    let rhs_x_off := mul(lhs_y_off, 2)
                    let rhs_y_off := mul(lhs_y_off, 3)
                    let lhs_x := calldataload(cptr)
                    let lhs_y := calldataload(add(cptr, lhs_y_off))
                    let rhs_x := calldataload(add(cptr, rhs_x_off))
                    let rhs_y := calldataload(add(cptr, rhs_y_off))
                    for {
                        let cptr_end := add(cptr, mul(0x20, num_limbs))
                        let shift := num_limb_bits
                    } lt(cptr, cptr_end) {

                    } {
                        cptr := add(cptr, 0x20)
                        lhs_x := add(lhs_x, shl(shift, calldataload(cptr)))
                        lhs_y := add(
                            lhs_y,
                            shl(shift, calldataload(add(cptr, lhs_y_off)))
                        )
                        rhs_x := add(
                            rhs_x,
                            shl(shift, calldataload(add(cptr, rhs_x_off)))
                        )
                        rhs_y := add(
                            rhs_y,
                            shl(shift, calldataload(add(cptr, rhs_y_off)))
                        )
                        shift := add(shift, num_limb_bits)
                    }

                    success := and(success, and(lt(lhs_x, q), lt(lhs_y, q)))
                    success := and(
                        success,
                        eq(
                            mulmod(lhs_y, lhs_y, q),
                            addmod(
                                mulmod(lhs_x, mulmod(lhs_x, lhs_x, q), q),
                                3,
                                q
                            )
                        )
                    )
                    success := and(success, and(lt(rhs_x, q), lt(rhs_y, q)))
                    success := and(
                        success,
                        eq(
                            mulmod(rhs_y, rhs_y, q),
                            addmod(
                                mulmod(rhs_x, mulmod(rhs_x, rhs_x, q), q),
                                3,
                                q
                            )
                        )
                    )

                    mstore(ACC_LHS_X_MPTR, lhs_x)
                    mstore(ACC_LHS_Y_MPTR, lhs_y)
                    mstore(ACC_RHS_X_MPTR, rhs_x)
                    mstore(ACC_RHS_Y_MPTR, rhs_y)
                }

                pop(q)
            }

            // Revert earlier if anything from calldata is invalid
            if iszero(success) {
                revert(0, 0)
            }

            // Compute lagrange evaluations and instance evaluation
            {
                let k := mload(K_MPTR)
                let x := mload(X_MPTR)
                let x_n := x
                for {
                    let idx := 0
                } lt(idx, k) {
                    idx := add(idx, 1)
                } {
                    x_n := mulmod(x_n, x_n, r)
                }

                let omega := mload(OMEGA_MPTR)

                let mptr := X_N_MPTR
                let mptr_end := add(
                    mptr,
                    mul(0x20, add(mload(NUM_INSTANCES_MPTR), 7))
                )
                if iszero(mload(NUM_INSTANCES_MPTR)) {
                    mptr_end := add(mptr_end, 0x20)
                }
                for {
                    let pow_of_omega := mload(OMEGA_INV_TO_L_MPTR)
                } lt(mptr, mptr_end) {
                    mptr := add(mptr, 0x20)
                } {
                    mstore(mptr, addmod(x, sub(r, pow_of_omega), r))
                    pow_of_omega := mulmod(pow_of_omega, omega, r)
                }
                let x_n_minus_1 := addmod(x_n, sub(r, 1), r)
                mstore(mptr_end, x_n_minus_1)
                success := batch_invert(
                    success,
                    X_N_MPTR,
                    add(mptr_end, 0x20),
                    r
                )

                mptr := X_N_MPTR
                let l_i_common := mulmod(x_n_minus_1, mload(N_INV_MPTR), r)
                for {
                    let pow_of_omega := mload(OMEGA_INV_TO_L_MPTR)
                } lt(mptr, mptr_end) {
                    mptr := add(mptr, 0x20)
                } {
                    mstore(
                        mptr,
                        mulmod(
                            l_i_common,
                            mulmod(mload(mptr), pow_of_omega, r),
                            r
                        )
                    )
                    pow_of_omega := mulmod(pow_of_omega, omega, r)
                }

                let l_blind := mload(add(X_N_MPTR, 0x20))
                let l_i_cptr := add(X_N_MPTR, 0x40)
                for {
                    let l_i_cptr_end := add(X_N_MPTR, 0xe0)
                } lt(l_i_cptr, l_i_cptr_end) {
                    l_i_cptr := add(l_i_cptr, 0x20)
                } {
                    l_blind := addmod(l_blind, mload(l_i_cptr), r)
                }

                let instance_eval := 0
                for {
                    let instance_cptr := INSTANCE_CPTR
                    let instance_cptr_end := add(
                        instance_cptr,
                        mul(0x20, mload(NUM_INSTANCES_MPTR))
                    )
                } lt(instance_cptr, instance_cptr_end) {
                    instance_cptr := add(instance_cptr, 0x20)
                    l_i_cptr := add(l_i_cptr, 0x20)
                } {
                    instance_eval := addmod(
                        instance_eval,
                        mulmod(mload(l_i_cptr), calldataload(instance_cptr), r),
                        r
                    )
                }

                let x_n_minus_1_inv := mload(mptr_end)
                let l_last := mload(X_N_MPTR)
                let l_0 := mload(add(X_N_MPTR, 0xe0))

                mstore(X_N_MPTR, x_n)
                mstore(X_N_MINUS_1_INV_MPTR, x_n_minus_1_inv)
                mstore(L_LAST_MPTR, l_last)
                mstore(L_BLIND_MPTR, l_blind)
                mstore(L_0_MPTR, l_0)
                mstore(INSTANCE_EVAL_MPTR, instance_eval)
            }

            // Compute quotient evavluation
            {
                let quotient_eval_numer
                let
                    delta
                := 4131629893567559867359510883348571134090853742863529169391034518566172092834
                let y := mload(Y_MPTR)
                {
                    let f_2 := calldataload(0x11c4)
                    let a_0 := calldataload(0x0a64)
                    let a_0_next_1 := calldataload(0x0a84)
                    let a_0_next_2 := calldataload(0x0aa4)
                    let var0 := mulmod(a_0_next_1, a_0_next_2, r)
                    let var1 := addmod(a_0, var0, r)
                    let a_0_next_3 := calldataload(0x0ac4)
                    let var2 := sub(r, a_0_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_2, var3, r)
                    quotient_eval_numer := var4
                }
                {
                    let f_3 := calldataload(0x11e4)
                    let a_1 := calldataload(0x0ae4)
                    let a_1_next_1 := calldataload(0x0b04)
                    let a_1_next_2 := calldataload(0x0b24)
                    let var0 := mulmod(a_1_next_1, a_1_next_2, r)
                    let var1 := addmod(a_1, var0, r)
                    let a_1_next_3 := calldataload(0x0b44)
                    let var2 := sub(r, a_1_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_3, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_4 := calldataload(0x1204)
                    let a_2 := calldataload(0x0b64)
                    let a_2_next_1 := calldataload(0x0b84)
                    let a_2_next_2 := calldataload(0x0ba4)
                    let var0 := mulmod(a_2_next_1, a_2_next_2, r)
                    let var1 := addmod(a_2, var0, r)
                    let a_2_next_3 := calldataload(0x0bc4)
                    let var2 := sub(r, a_2_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_4, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_5 := calldataload(0x1224)
                    let a_3 := calldataload(0x0be4)
                    let a_3_next_1 := calldataload(0x0c04)
                    let a_3_next_2 := calldataload(0x0c24)
                    let var0 := mulmod(a_3_next_1, a_3_next_2, r)
                    let var1 := addmod(a_3, var0, r)
                    let a_3_next_3 := calldataload(0x0c44)
                    let var2 := sub(r, a_3_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_5, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_6 := calldataload(0x1244)
                    let a_4 := calldataload(0x0c64)
                    let a_4_next_1 := calldataload(0x0c84)
                    let a_4_next_2 := calldataload(0x0ca4)
                    let var0 := mulmod(a_4_next_1, a_4_next_2, r)
                    let var1 := addmod(a_4, var0, r)
                    let a_4_next_3 := calldataload(0x0cc4)
                    let var2 := sub(r, a_4_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_6, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_7 := calldataload(0x1264)
                    let a_5 := calldataload(0x0ce4)
                    let a_5_next_1 := calldataload(0x0d04)
                    let a_5_next_2 := calldataload(0x0d24)
                    let var0 := mulmod(a_5_next_1, a_5_next_2, r)
                    let var1 := addmod(a_5, var0, r)
                    let a_5_next_3 := calldataload(0x0d44)
                    let var2 := sub(r, a_5_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_7, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_8 := calldataload(0x1284)
                    let a_6 := calldataload(0x0d64)
                    let a_6_next_1 := calldataload(0x0d84)
                    let a_6_next_2 := calldataload(0x0da4)
                    let var0 := mulmod(a_6_next_1, a_6_next_2, r)
                    let var1 := addmod(a_6, var0, r)
                    let a_6_next_3 := calldataload(0x0dc4)
                    let var2 := sub(r, a_6_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_8, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_9 := calldataload(0x12a4)
                    let a_7 := calldataload(0x0de4)
                    let a_7_next_1 := calldataload(0x0e04)
                    let a_7_next_2 := calldataload(0x0e24)
                    let var0 := mulmod(a_7_next_1, a_7_next_2, r)
                    let var1 := addmod(a_7, var0, r)
                    let a_7_next_3 := calldataload(0x0e44)
                    let var2 := sub(r, a_7_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_9, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_10 := calldataload(0x12c4)
                    let a_8 := calldataload(0x0e64)
                    let a_8_next_1 := calldataload(0x0e84)
                    let a_8_next_2 := calldataload(0x0ea4)
                    let var0 := mulmod(a_8_next_1, a_8_next_2, r)
                    let var1 := addmod(a_8, var0, r)
                    let a_8_next_3 := calldataload(0x0ec4)
                    let var2 := sub(r, a_8_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_10, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_11 := calldataload(0x12e4)
                    let a_9 := calldataload(0x0ee4)
                    let a_9_next_1 := calldataload(0x0f04)
                    let a_9_next_2 := calldataload(0x0f24)
                    let var0 := mulmod(a_9_next_1, a_9_next_2, r)
                    let var1 := addmod(a_9, var0, r)
                    let a_9_next_3 := calldataload(0x0f44)
                    let var2 := sub(r, a_9_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_11, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_12 := calldataload(0x1304)
                    let a_10 := calldataload(0x0f64)
                    let a_10_next_1 := calldataload(0x0f84)
                    let a_10_next_2 := calldataload(0x0fa4)
                    let var0 := mulmod(a_10_next_1, a_10_next_2, r)
                    let var1 := addmod(a_10, var0, r)
                    let a_10_next_3 := calldataload(0x0fc4)
                    let var2 := sub(r, a_10_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_12, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_13 := calldataload(0x1324)
                    let a_11 := calldataload(0x0fe4)
                    let a_11_next_1 := calldataload(0x1004)
                    let a_11_next_2 := calldataload(0x1024)
                    let var0 := mulmod(a_11_next_1, a_11_next_2, r)
                    let var1 := addmod(a_11, var0, r)
                    let a_11_next_3 := calldataload(0x1044)
                    let var2 := sub(r, a_11_next_3)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_13, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_14 := calldataload(0x1344)
                    let a_15 := calldataload(0x10c4)
                    let c_0 := mload(0x1400)
                    let var0 := mulmod(a_15, c_0, r)
                    let a_15_next_1 := calldataload(0x1104)
                    let var1 := addmod(var0, a_15_next_1, r)
                    let a_15_next_2 := calldataload(0x1124)
                    let var2 := sub(r, a_15_next_2)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_14, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let f_15 := calldataload(0x1364)
                    let a_16 := calldataload(0x10e4)
                    let c_0 := mload(0x1400)
                    let var0 := mulmod(a_16, c_0, r)
                    let a_16_next_1 := calldataload(0x1144)
                    let var1 := addmod(var0, a_16_next_1, r)
                    let a_16_next_2 := calldataload(0x1164)
                    let var2 := sub(r, a_16_next_2)
                    let var3 := addmod(var1, var2, r)
                    let var4 := mulmod(f_15, var3, r)
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        var4,
                        r
                    )
                }
                {
                    let l_0 := mload(L_0_MPTR)
                    let eval := addmod(
                        l_0,
                        sub(r, mulmod(l_0, calldataload(0x1604), r)),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let perm_z_last := calldataload(0x1964)
                    let eval := mulmod(
                        mload(L_LAST_MPTR),
                        addmod(
                            mulmod(perm_z_last, perm_z_last, r),
                            sub(r, perm_z_last),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1664),
                            sub(r, calldataload(0x1644)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x16c4),
                            sub(r, calldataload(0x16a4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1724),
                            sub(r, calldataload(0x1704)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1784),
                            sub(r, calldataload(0x1764)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x17e4),
                            sub(r, calldataload(0x17c4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1844),
                            sub(r, calldataload(0x1824)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x18a4),
                            sub(r, calldataload(0x1884)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1904),
                            sub(r, calldataload(0x18e4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1964),
                            sub(r, calldataload(0x1944)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1624)
                    let rhs := calldataload(0x1604)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x1184),
                                mulmod(beta, calldataload(0x13a4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0a64),
                                mulmod(beta, calldataload(0x13c4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(beta, mload(X_MPTR), r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x1184), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0a64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1684)
                    let rhs := calldataload(0x1664)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0ae4),
                                mulmod(beta, calldataload(0x13e4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0b64),
                                mulmod(beta, calldataload(0x1404), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0ae4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0b64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x16e4)
                    let rhs := calldataload(0x16c4)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0be4),
                                mulmod(beta, calldataload(0x1424), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0c64),
                                mulmod(beta, calldataload(0x1444), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0be4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0c64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1744)
                    let rhs := calldataload(0x1724)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0ce4),
                                mulmod(beta, calldataload(0x1464), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0d64),
                                mulmod(beta, calldataload(0x1484), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0ce4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0d64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x17a4)
                    let rhs := calldataload(0x1784)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0de4),
                                mulmod(beta, calldataload(0x14a4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0e64),
                                mulmod(beta, calldataload(0x14c4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0de4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0e64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1804)
                    let rhs := calldataload(0x17e4)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0ee4),
                                mulmod(beta, calldataload(0x14e4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0f64),
                                mulmod(beta, calldataload(0x1504), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0ee4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0f64), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1864)
                    let rhs := calldataload(0x1844)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x0fe4),
                                mulmod(beta, calldataload(0x1524), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x1064),
                                mulmod(beta, calldataload(0x1544), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x0fe4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x1064), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x18c4)
                    let rhs := calldataload(0x18a4)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x1084),
                                mulmod(beta, calldataload(0x1564), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x10a4),
                                mulmod(beta, calldataload(0x1584), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x1084), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x10a4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1924)
                    let rhs := calldataload(0x1904)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                mload(INSTANCE_EVAL_MPTR),
                                mulmod(beta, calldataload(0x15a4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x10c4),
                                mulmod(beta, calldataload(0x15c4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(mload(INSTANCE_EVAL_MPTR), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x10c4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    mstore(0x00, mulmod(mload(0x00), delta, r))
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let gamma := mload(GAMMA_MPTR)
                    let beta := mload(BETA_MPTR)
                    let lhs := calldataload(0x1984)
                    let rhs := calldataload(0x1964)
                    lhs := mulmod(
                        lhs,
                        addmod(
                            addmod(
                                calldataload(0x10e4),
                                mulmod(beta, calldataload(0x15e4), r),
                                r
                            ),
                            gamma,
                            r
                        ),
                        r
                    )
                    rhs := mulmod(
                        rhs,
                        addmod(
                            addmod(calldataload(0x10e4), mload(0x00), r),
                            gamma,
                            r
                        ),
                        r
                    )
                    let left_sub_right := addmod(lhs, sub(r, rhs), r)
                    let eval := addmod(
                        left_sub_right,
                        sub(
                            r,
                            mulmod(
                                left_sub_right,
                                addmod(
                                    mload(L_LAST_MPTR),
                                    mload(L_BLIND_MPTR),
                                    r
                                ),
                                r
                            )
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_0 := mload(L_0_MPTR)
                    let eval := addmod(
                        l_0,
                        mulmod(l_0, sub(r, calldataload(0x19a4)), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_last := mload(L_LAST_MPTR)
                    let eval := mulmod(
                        l_last,
                        addmod(
                            mulmod(
                                calldataload(0x19a4),
                                calldataload(0x19a4),
                                r
                            ),
                            sub(r, calldataload(0x19a4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let theta := mload(THETA_MPTR)
                    let input
                    {
                        let a_12 := calldataload(0x1064)
                        input := a_12
                    }
                    let table
                    {
                        let f_0 := calldataload(0x11a4)
                        table := f_0
                    }
                    let beta := mload(BETA_MPTR)
                    let gamma := mload(GAMMA_MPTR)
                    let lhs := mulmod(
                        calldataload(0x19c4),
                        mulmod(
                            addmod(calldataload(0x19e4), beta, r),
                            addmod(calldataload(0x1a24), gamma, r),
                            r
                        ),
                        r
                    )
                    let rhs := mulmod(
                        calldataload(0x19a4),
                        mulmod(
                            addmod(input, beta, r),
                            addmod(table, gamma, r),
                            r
                        ),
                        r
                    )
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        addmod(lhs, sub(r, rhs), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x19e4),
                            sub(r, calldataload(0x1a24)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        mulmod(
                            addmod(
                                calldataload(0x19e4),
                                sub(r, calldataload(0x1a24)),
                                r
                            ),
                            addmod(
                                calldataload(0x19e4),
                                sub(r, calldataload(0x1a04)),
                                r
                            ),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_0 := mload(L_0_MPTR)
                    let eval := addmod(
                        l_0,
                        mulmod(l_0, sub(r, calldataload(0x1a44)), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_last := mload(L_LAST_MPTR)
                    let eval := mulmod(
                        l_last,
                        addmod(
                            mulmod(
                                calldataload(0x1a44),
                                calldataload(0x1a44),
                                r
                            ),
                            sub(r, calldataload(0x1a44)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let theta := mload(THETA_MPTR)
                    let input
                    {
                        let a_13 := calldataload(0x1084)
                        input := a_13
                    }
                    let table
                    {
                        let f_0 := calldataload(0x11a4)
                        table := f_0
                    }
                    let beta := mload(BETA_MPTR)
                    let gamma := mload(GAMMA_MPTR)
                    let lhs := mulmod(
                        calldataload(0x1a64),
                        mulmod(
                            addmod(calldataload(0x1a84), beta, r),
                            addmod(calldataload(0x1ac4), gamma, r),
                            r
                        ),
                        r
                    )
                    let rhs := mulmod(
                        calldataload(0x1a44),
                        mulmod(
                            addmod(input, beta, r),
                            addmod(table, gamma, r),
                            r
                        ),
                        r
                    )
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        addmod(lhs, sub(r, rhs), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1a84),
                            sub(r, calldataload(0x1ac4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        mulmod(
                            addmod(
                                calldataload(0x1a84),
                                sub(r, calldataload(0x1ac4)),
                                r
                            ),
                            addmod(
                                calldataload(0x1a84),
                                sub(r, calldataload(0x1aa4)),
                                r
                            ),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_0 := mload(L_0_MPTR)
                    let eval := addmod(
                        l_0,
                        mulmod(l_0, sub(r, calldataload(0x1ae4)), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let l_last := mload(L_LAST_MPTR)
                    let eval := mulmod(
                        l_last,
                        addmod(
                            mulmod(
                                calldataload(0x1ae4),
                                calldataload(0x1ae4),
                                r
                            ),
                            sub(r, calldataload(0x1ae4)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let theta := mload(THETA_MPTR)
                    let input
                    {
                        let a_14 := calldataload(0x10a4)
                        input := a_14
                    }
                    let table
                    {
                        let f_0 := calldataload(0x11a4)
                        table := f_0
                    }
                    let beta := mload(BETA_MPTR)
                    let gamma := mload(GAMMA_MPTR)
                    let lhs := mulmod(
                        calldataload(0x1b04),
                        mulmod(
                            addmod(calldataload(0x1b24), beta, r),
                            addmod(calldataload(0x1b64), gamma, r),
                            r
                        ),
                        r
                    )
                    let rhs := mulmod(
                        calldataload(0x1ae4),
                        mulmod(
                            addmod(input, beta, r),
                            addmod(table, gamma, r),
                            r
                        ),
                        r
                    )
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        addmod(lhs, sub(r, rhs), r),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        mload(L_0_MPTR),
                        addmod(
                            calldataload(0x1b24),
                            sub(r, calldataload(0x1b64)),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }
                {
                    let eval := mulmod(
                        addmod(
                            1,
                            sub(
                                r,
                                addmod(
                                    mload(L_BLIND_MPTR),
                                    mload(L_LAST_MPTR),
                                    r
                                )
                            ),
                            r
                        ),
                        mulmod(
                            addmod(
                                calldataload(0x1b24),
                                sub(r, calldataload(0x1b64)),
                                r
                            ),
                            addmod(
                                calldataload(0x1b24),
                                sub(r, calldataload(0x1b44)),
                                r
                            ),
                            r
                        ),
                        r
                    )
                    quotient_eval_numer := addmod(
                        mulmod(quotient_eval_numer, y, r),
                        eval,
                        r
                    )
                }

                pop(y)
                pop(delta)

                let quotient_eval := mulmod(
                    quotient_eval_numer,
                    mload(X_N_MINUS_1_INV_MPTR),
                    r
                )
                mstore(QUOTIENT_EVAL_MPTR, quotient_eval)
            }

            // Compute quotient commitment
            {
                mstore(0x00, calldataload(LAST_QUOTIENT_X_CPTR))
                mstore(0x20, calldataload(add(LAST_QUOTIENT_X_CPTR, 0x20)))
                let x_n := mload(X_N_MPTR)
                for {
                    let cptr := sub(LAST_QUOTIENT_X_CPTR, 0x40)
                    let cptr_end := sub(FIRST_QUOTIENT_X_CPTR, 0x40)
                } lt(cptr_end, cptr) {

                } {
                    success := ec_mul_acc(success, x_n)
                    success := ec_add_acc(
                        success,
                        calldataload(cptr),
                        calldataload(add(cptr, 0x20))
                    )
                    cptr := sub(cptr, 0x40)
                }
                mstore(QUOTIENT_X_MPTR, mload(0x00))
                mstore(QUOTIENT_Y_MPTR, mload(0x20))
            }

            // Compute pairing lhs and rhs
            {
                {
                    let x := mload(X_MPTR)
                    let omega := mload(OMEGA_MPTR)
                    let omega_inv := mload(OMEGA_INV_MPTR)
                    let x_pow_of_omega := mulmod(x, omega, r)
                    mstore(0x0520, x_pow_of_omega)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega, r)
                    mstore(0x0540, x_pow_of_omega)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega, r)
                    mstore(0x0560, x_pow_of_omega)
                    mstore(0x0500, x)
                    x_pow_of_omega := mulmod(x, omega_inv, r)
                    mstore(0x04e0, x_pow_of_omega)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    x_pow_of_omega := mulmod(x_pow_of_omega, omega_inv, r)
                    mstore(0x04c0, x_pow_of_omega)
                }
                {
                    let mu := mload(MU_MPTR)
                    for {
                        let mptr := 0x0580
                        let mptr_end := 0x0640
                        let point_mptr := 0x04c0
                    } lt(mptr, mptr_end) {
                        mptr := add(mptr, 0x20)
                        point_mptr := add(point_mptr, 0x20)
                    } {
                        mstore(mptr, addmod(mu, sub(r, mload(point_mptr)), r))
                    }
                    let s
                    s := mload(0x05c0)
                    s := mulmod(s, mload(0x05e0), r)
                    s := mulmod(s, mload(0x0600), r)
                    s := mulmod(s, mload(0x0620), r)
                    mstore(0x0640, s)
                    let diff
                    diff := mload(0x0580)
                    diff := mulmod(diff, mload(0x05a0), r)
                    mstore(0x0660, diff)
                    mstore(0x00, diff)
                    diff := mload(0x0580)
                    diff := mulmod(diff, mload(0x05a0), r)
                    diff := mulmod(diff, mload(0x05e0), r)
                    diff := mulmod(diff, mload(0x0600), r)
                    diff := mulmod(diff, mload(0x0620), r)
                    mstore(0x0680, diff)
                    diff := mload(0x0580)
                    diff := mulmod(diff, mload(0x05a0), r)
                    diff := mulmod(diff, mload(0x0620), r)
                    mstore(0x06a0, diff)
                    diff := mload(0x05a0)
                    diff := mulmod(diff, mload(0x0600), r)
                    diff := mulmod(diff, mload(0x0620), r)
                    mstore(0x06c0, diff)
                    diff := mload(0x0580)
                    diff := mulmod(diff, mload(0x05a0), r)
                    diff := mulmod(diff, mload(0x0600), r)
                    diff := mulmod(diff, mload(0x0620), r)
                    mstore(0x06e0, diff)
                    diff := mload(0x0580)
                    diff := mulmod(diff, mload(0x05e0), r)
                    diff := mulmod(diff, mload(0x0600), r)
                    diff := mulmod(diff, mload(0x0620), r)
                    mstore(0x0700, diff)
                }
                {
                    let point_2 := mload(0x0500)
                    let point_3 := mload(0x0520)
                    let point_4 := mload(0x0540)
                    let point_5 := mload(0x0560)
                    let coeff
                    coeff := addmod(point_2, sub(r, point_3), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_2, sub(r, point_4), r),
                        r
                    )
                    coeff := mulmod(
                        coeff,
                        addmod(point_2, sub(r, point_5), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0x20, coeff)
                    coeff := addmod(point_3, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_3, sub(r, point_4), r),
                        r
                    )
                    coeff := mulmod(
                        coeff,
                        addmod(point_3, sub(r, point_5), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05e0), r)
                    mstore(0x40, coeff)
                    coeff := addmod(point_4, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_4, sub(r, point_3), r),
                        r
                    )
                    coeff := mulmod(
                        coeff,
                        addmod(point_4, sub(r, point_5), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x0600), r)
                    mstore(0x60, coeff)
                    coeff := addmod(point_5, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_5, sub(r, point_3), r),
                        r
                    )
                    coeff := mulmod(
                        coeff,
                        addmod(point_5, sub(r, point_4), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x0620), r)
                    mstore(0x80, coeff)
                }
                {
                    let point_2 := mload(0x0500)
                    let coeff
                    coeff := 1
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0xa0, coeff)
                }
                {
                    let point_2 := mload(0x0500)
                    let point_3 := mload(0x0520)
                    let point_4 := mload(0x0540)
                    let coeff
                    coeff := addmod(point_2, sub(r, point_3), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_2, sub(r, point_4), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0xc0, coeff)
                    coeff := addmod(point_3, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_3, sub(r, point_4), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05e0), r)
                    mstore(0xe0, coeff)
                    coeff := addmod(point_4, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_4, sub(r, point_3), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x0600), r)
                    mstore(0x0100, coeff)
                }
                {
                    let point_0 := mload(0x04c0)
                    let point_2 := mload(0x0500)
                    let point_3 := mload(0x0520)
                    let coeff
                    coeff := addmod(point_0, sub(r, point_2), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_0, sub(r, point_3), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x0580), r)
                    mstore(0x0120, coeff)
                    coeff := addmod(point_2, sub(r, point_0), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_2, sub(r, point_3), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0x0140, coeff)
                    coeff := addmod(point_3, sub(r, point_0), r)
                    coeff := mulmod(
                        coeff,
                        addmod(point_3, sub(r, point_2), r),
                        r
                    )
                    coeff := mulmod(coeff, mload(0x05e0), r)
                    mstore(0x0160, coeff)
                }
                {
                    let point_2 := mload(0x0500)
                    let point_3 := mload(0x0520)
                    let coeff
                    coeff := addmod(point_2, sub(r, point_3), r)
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0x0180, coeff)
                    coeff := addmod(point_3, sub(r, point_2), r)
                    coeff := mulmod(coeff, mload(0x05e0), r)
                    mstore(0x01a0, coeff)
                }
                {
                    let point_1 := mload(0x04e0)
                    let point_2 := mload(0x0500)
                    let coeff
                    coeff := addmod(point_1, sub(r, point_2), r)
                    coeff := mulmod(coeff, mload(0x05a0), r)
                    mstore(0x01c0, coeff)
                    coeff := addmod(point_2, sub(r, point_1), r)
                    coeff := mulmod(coeff, mload(0x05c0), r)
                    mstore(0x01e0, coeff)
                }
                {
                    success := batch_invert(success, 0, 0x0200, r)
                    let diff_0_inv := mload(0x00)
                    mstore(0x0660, diff_0_inv)
                    for {
                        let mptr := 0x0680
                        let mptr_end := 0x0720
                    } lt(mptr, mptr_end) {
                        mptr := add(mptr, 0x20)
                    } {
                        mstore(mptr, mulmod(mload(mptr), diff_0_inv, r))
                    }
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0fe4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x1004), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x1024), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x1044), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0f64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0f84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0fa4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0fc4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0ee4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0f04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0f24), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0f44), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0e64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0e84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0ea4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0ec4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0de4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0e04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0e24), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0e44), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0d64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0d84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0da4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0dc4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0ce4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0d04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0d24), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0d44), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0c64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0c84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0ca4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0cc4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0be4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0c04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0c24), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0c44), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0b64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0b84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0ba4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0bc4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0ae4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0b04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0b24), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0b44), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x20), calldataload(0x0a64), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x40), calldataload(0x0a84), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x60), calldataload(0x0aa4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x80), calldataload(0x0ac4), r),
                        r
                    )
                    mstore(0x0720, r_eval)
                }
                {
                    let coeff := mload(0xa0)
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := mulmod(coeff, calldataload(0x1384), r)
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(coeff, mload(QUOTIENT_EVAL_MPTR), r),
                        r
                    )
                    for {
                        let cptr := 0x15e4
                        let cptr_end := 0x1384
                    } lt(cptr_end, cptr) {
                        cptr := sub(cptr, 0x20)
                    } {
                        r_eval := addmod(
                            mulmod(r_eval, zeta, r),
                            mulmod(coeff, calldataload(cptr), r),
                            r
                        )
                    }
                    for {
                        let cptr := 0x1364
                        let cptr_end := 0x1164
                    } lt(cptr_end, cptr) {
                        cptr := sub(cptr, 0x20)
                    } {
                        r_eval := addmod(
                            mulmod(r_eval, zeta, r),
                            mulmod(coeff, calldataload(cptr), r),
                            r
                        )
                    }
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(coeff, calldataload(0x1b64), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(coeff, calldataload(0x1ac4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(coeff, calldataload(0x1a24), r),
                        r
                    )
                    for {
                        let cptr := 0x10a4
                        let cptr_end := 0x1044
                    } lt(cptr_end, cptr) {
                        cptr := sub(cptr, 0x20)
                    } {
                        r_eval := addmod(
                            mulmod(r_eval, zeta, r),
                            mulmod(coeff, calldataload(cptr), r),
                            r
                        )
                    }
                    r_eval := mulmod(r_eval, mload(0x0680), r)
                    mstore(0x0740, r_eval)
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0xc0), calldataload(0x10e4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0xe0), calldataload(0x1144), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0100), calldataload(0x1164), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0xc0), calldataload(0x10c4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0xe0), calldataload(0x1104), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0100), calldataload(0x1124), r),
                        r
                    )
                    r_eval := mulmod(r_eval, mload(0x06a0), r)
                    mstore(0x0760, r_eval)
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1944), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1904), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1924), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x18e4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x18a4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x18c4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1884), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1844), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1864), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1824), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x17e4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1804), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x17c4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1784), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x17a4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1764), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1724), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1744), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1704), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x16c4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x16e4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x16a4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1664), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1684), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0120), calldataload(0x1644), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0140), calldataload(0x1604), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0160), calldataload(0x1624), r),
                        r
                    )
                    r_eval := mulmod(r_eval, mload(0x06c0), r)
                    mstore(0x0780, r_eval)
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0180), calldataload(0x1ae4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01a0), calldataload(0x1b04), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0180), calldataload(0x1a44), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01a0), calldataload(0x1a64), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0180), calldataload(0x19a4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01a0), calldataload(0x19c4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x0180), calldataload(0x1964), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01a0), calldataload(0x1984), r),
                        r
                    )
                    r_eval := mulmod(r_eval, mload(0x06e0), r)
                    mstore(0x07a0, r_eval)
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let r_eval
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01c0), calldataload(0x1b44), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01e0), calldataload(0x1b24), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01c0), calldataload(0x1aa4), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01e0), calldataload(0x1a84), r),
                        r
                    )
                    r_eval := mulmod(r_eval, zeta, r)
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01c0), calldataload(0x1a04), r),
                        r
                    )
                    r_eval := addmod(
                        r_eval,
                        mulmod(mload(0x01e0), calldataload(0x19e4), r),
                        r
                    )
                    r_eval := mulmod(r_eval, mload(0x0700), r)
                    mstore(0x07c0, r_eval)
                }
                {
                    let sum := mload(0x20)
                    sum := addmod(sum, mload(0x40), r)
                    sum := addmod(sum, mload(0x60), r)
                    sum := addmod(sum, mload(0x80), r)
                    mstore(0x07e0, sum)
                }
                {
                    let sum := mload(0xa0)
                    mstore(0x0800, sum)
                }
                {
                    let sum := mload(0xc0)
                    sum := addmod(sum, mload(0xe0), r)
                    sum := addmod(sum, mload(0x0100), r)
                    mstore(0x0820, sum)
                }
                {
                    let sum := mload(0x0120)
                    sum := addmod(sum, mload(0x0140), r)
                    sum := addmod(sum, mload(0x0160), r)
                    mstore(0x0840, sum)
                }
                {
                    let sum := mload(0x0180)
                    sum := addmod(sum, mload(0x01a0), r)
                    mstore(0x0860, sum)
                }
                {
                    let sum := mload(0x01c0)
                    sum := addmod(sum, mload(0x01e0), r)
                    mstore(0x0880, sum)
                }
                {
                    for {
                        let mptr := 0x00
                        let mptr_end := 0xc0
                        let sum_mptr := 0x07e0
                    } lt(mptr, mptr_end) {
                        mptr := add(mptr, 0x20)
                        sum_mptr := add(sum_mptr, 0x20)
                    } {
                        mstore(mptr, mload(sum_mptr))
                    }
                    success := batch_invert(success, 0, 0xc0, r)
                    let r_eval := mulmod(mload(0xa0), mload(0x07c0), r)
                    for {
                        let sum_inv_mptr := 0x80
                        let sum_inv_mptr_end := 0xc0
                        let r_eval_mptr := 0x07a0
                    } lt(sum_inv_mptr, sum_inv_mptr_end) {
                        sum_inv_mptr := sub(sum_inv_mptr, 0x20)
                        r_eval_mptr := sub(r_eval_mptr, 0x20)
                    } {
                        r_eval := mulmod(r_eval, mload(NU_MPTR), r)
                        r_eval := addmod(
                            r_eval,
                            mulmod(mload(sum_inv_mptr), mload(r_eval_mptr), r),
                            r
                        )
                    }
                    mstore(G1_SCALAR_MPTR, sub(r, r_eval))
                }
                {
                    let zeta := mload(ZETA_MPTR)
                    let nu := mload(NU_MPTR)
                    mstore(0x00, calldataload(0x0324))
                    mstore(0x20, calldataload(0x0344))
                    for {
                        let ptr := 0x02e4
                        let ptr_end := 0x24
                    } lt(ptr_end, ptr) {
                        ptr := sub(ptr, 0x40)
                    } {
                        success := ec_mul_acc(success, zeta)
                        success := ec_add_acc(
                            success,
                            calldataload(ptr),
                            calldataload(add(ptr, 0x20))
                        )
                    }
                    mstore(0x80, calldataload(0x0964))
                    mstore(0xa0, calldataload(0x0984))
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        mload(QUOTIENT_X_MPTR),
                        mload(QUOTIENT_Y_MPTR)
                    )
                    for {
                        let ptr := 0x13c0
                        let ptr_end := 0x0b80
                    } lt(ptr_end, ptr) {
                        ptr := sub(ptr, 0x40)
                    } {
                        success := ec_mul_tmp(success, zeta)
                        success := ec_add_tmp(
                            success,
                            mload(ptr),
                            mload(add(ptr, 0x20))
                        )
                    }
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(success, mload(0x0b40), mload(0x0b60))
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(success, mload(0x0b80), mload(0x0ba0))
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x05e4),
                        calldataload(0x0604)
                    )
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x0564),
                        calldataload(0x0584)
                    )
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x04e4),
                        calldataload(0x0504)
                    )
                    for {
                        let ptr := 0x03e4
                        let ptr_end := 0x0324
                    } lt(ptr_end, ptr) {
                        ptr := sub(ptr, 0x40)
                    } {
                        success := ec_mul_tmp(success, zeta)
                        success := ec_add_tmp(
                            success,
                            calldataload(ptr),
                            calldataload(add(ptr, 0x20))
                        )
                    }
                    success := ec_mul_tmp(success, mulmod(nu, mload(0x0680), r))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    nu := mulmod(nu, mload(NU_MPTR), r)
                    mstore(0x80, calldataload(0x0464))
                    mstore(0xa0, calldataload(0x0484))
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x0424),
                        calldataload(0x0444)
                    )
                    success := ec_mul_tmp(success, mulmod(nu, mload(0x06a0), r))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    nu := mulmod(nu, mload(NU_MPTR), r)
                    mstore(0x80, calldataload(0x0824))
                    mstore(0xa0, calldataload(0x0844))
                    for {
                        let ptr := 0x07e4
                        let ptr_end := 0x05e4
                    } lt(ptr_end, ptr) {
                        ptr := sub(ptr, 0x40)
                    } {
                        success := ec_mul_tmp(success, zeta)
                        success := ec_add_tmp(
                            success,
                            calldataload(ptr),
                            calldataload(add(ptr, 0x20))
                        )
                    }
                    success := ec_mul_tmp(success, mulmod(nu, mload(0x06c0), r))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    nu := mulmod(nu, mload(NU_MPTR), r)
                    mstore(0x80, calldataload(0x0924))
                    mstore(0xa0, calldataload(0x0944))
                    for {
                        let ptr := 0x08e4
                        let ptr_end := 0x0824
                    } lt(ptr_end, ptr) {
                        ptr := sub(ptr, 0x40)
                    } {
                        success := ec_mul_tmp(success, zeta)
                        success := ec_add_tmp(
                            success,
                            calldataload(ptr),
                            calldataload(add(ptr, 0x20))
                        )
                    }
                    success := ec_mul_tmp(success, mulmod(nu, mload(0x06e0), r))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    nu := mulmod(nu, mload(NU_MPTR), r)
                    mstore(0x80, calldataload(0x05a4))
                    mstore(0xa0, calldataload(0x05c4))
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x0524),
                        calldataload(0x0544)
                    )
                    success := ec_mul_tmp(success, zeta)
                    success := ec_add_tmp(
                        success,
                        calldataload(0x04a4),
                        calldataload(0x04c4)
                    )
                    success := ec_mul_tmp(success, mulmod(nu, mload(0x0700), r))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    mstore(0x80, mload(G1_X_MPTR))
                    mstore(0xa0, mload(G1_Y_MPTR))
                    success := ec_mul_tmp(success, mload(G1_SCALAR_MPTR))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    mstore(0x80, calldataload(0x1b84))
                    mstore(0xa0, calldataload(0x1ba4))
                    success := ec_mul_tmp(success, sub(r, mload(0x0640)))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    mstore(0x80, calldataload(0x1bc4))
                    mstore(0xa0, calldataload(0x1be4))
                    success := ec_mul_tmp(success, mload(MU_MPTR))
                    success := ec_add_acc(success, mload(0x80), mload(0xa0))
                    mstore(PAIRING_LHS_X_MPTR, mload(0x00))
                    mstore(PAIRING_LHS_Y_MPTR, mload(0x20))
                    mstore(PAIRING_RHS_X_MPTR, calldataload(0x1bc4))
                    mstore(PAIRING_RHS_Y_MPTR, calldataload(0x1be4))
                }
            }

            // Random linear combine with accumulator
            if mload(HAS_ACCUMULATOR_MPTR) {
                mstore(0x00, mload(ACC_LHS_X_MPTR))
                mstore(0x20, mload(ACC_LHS_Y_MPTR))
                mstore(0x40, mload(ACC_RHS_X_MPTR))
                mstore(0x60, mload(ACC_RHS_Y_MPTR))
                mstore(0x80, mload(PAIRING_LHS_X_MPTR))
                mstore(0xa0, mload(PAIRING_LHS_Y_MPTR))
                mstore(0xc0, mload(PAIRING_RHS_X_MPTR))
                mstore(0xe0, mload(PAIRING_RHS_Y_MPTR))
                let challenge := mod(keccak256(0x00, 0x100), r)

                // [pairing_lhs] += challenge * [acc_lhs]
                success := ec_mul_acc(success, challenge)
                success := ec_add_acc(
                    success,
                    mload(PAIRING_LHS_X_MPTR),
                    mload(PAIRING_LHS_Y_MPTR)
                )
                mstore(PAIRING_LHS_X_MPTR, mload(0x00))
                mstore(PAIRING_LHS_Y_MPTR, mload(0x20))

                // [pairing_rhs] += challenge * [acc_rhs]
                mstore(0x00, mload(ACC_RHS_X_MPTR))
                mstore(0x20, mload(ACC_RHS_Y_MPTR))
                success := ec_mul_acc(success, challenge)
                success := ec_add_acc(
                    success,
                    mload(PAIRING_RHS_X_MPTR),
                    mload(PAIRING_RHS_Y_MPTR)
                )
                mstore(PAIRING_RHS_X_MPTR, mload(0x00))
                mstore(PAIRING_RHS_Y_MPTR, mload(0x20))
            }

            // Perform pairing
            success := ec_pairing(
                success,
                mload(PAIRING_LHS_X_MPTR),
                mload(PAIRING_LHS_Y_MPTR),
                mload(PAIRING_RHS_X_MPTR),
                mload(PAIRING_RHS_Y_MPTR)
            )

            // Revert if anything fails
            if iszero(success) {
                revert(0x00, 0x00)
            }

            // Return 1 as result if everything succeeds
            mstore(0x00, 1)
            return(0x00, 0x20)
        }
    }
}
