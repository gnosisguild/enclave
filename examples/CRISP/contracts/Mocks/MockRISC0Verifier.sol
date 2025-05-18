// SPDX-License-Identifier: MIT
pragma solidity ^0.8.27;

import {IRiscZeroVerifier, Receipt} from "risc0/IRiscZeroVerifier.sol";

contract MockRISC0Verifier is IRiscZeroVerifier {
    function verify(
        bytes calldata seal,
        bytes32 imageId,
        bytes32 journalDigest
    ) public view override {}

    function verifyIntegrity(Receipt calldata receipt) external view override {}
}
