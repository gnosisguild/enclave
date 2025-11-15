// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {RiscZeroGroth16Verifier as RiscZero} from "risc0/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "risc0/groth16/ControlID.sol";

contract RiscZeroGroth16Verifier is RiscZero {
    constructor() RiscZero(ControlID.CONTROL_ROOT, ControlID.BN254_CONTROL_ID) {}
}
