// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity ^0.8.27;

import {IRiscZeroVerifier, Receipt} from "@risc0/ethereum/contracts/src/IRiscZeroVerifier.sol";

contract MockRISC0Verifier is IRiscZeroVerifier {
    function verify(
        bytes calldata seal,
        bytes32 imageId,
        bytes32 journalDigest
    ) public view override {}

    function verifyIntegrity(Receipt calldata receipt) external view override {}
}
