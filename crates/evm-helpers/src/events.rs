// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::sol;

// TODO: extract these from that actual contract

sol! {
    #[derive(Debug)]
    event E3Activated(uint256 e3Id, uint256 expiration, bytes committeePublicKey);

    #[derive(Debug)]
    event E3Requested(uint256 e3Id, E3 e3, address filter, IE3Program indexed e3Program);

    #[derive(Debug)]
    interface IE3Program {
        function e3Program() external view returns (address);
    }

    #[derive(Debug)]
    interface IInputValidator {
        function validateInput(bytes data) external view returns (bool);
    }

    #[derive(Debug)]
    interface IDecryptionVerifier {
        function verifyDecryption(bytes data) external view returns (bool);
    }

    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256 requestBlock;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        bytes32 encryptionSchemeId;
        IE3Program e3Program;
        bytes e3ProgramParams;
        bytes customParams;
        IInputValidator inputValidator;
        IDecryptionVerifier decryptionVerifier;
        bytes32 committeePublicKey;
        bytes32 ciphertextOutput;
        bytes plaintextOutput;
    }

    #[derive(Debug)]
    event InputPublished(uint256 indexed e3Id, bytes data, uint256 inputHash, uint256 index);

    #[derive(Debug)]
    event CiphertextOutputPublished(uint256 indexed e3Id, bytes ciphertextOutput);

    #[derive(Debug)]
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);

    #[derive(Debug)]
    event CommitteePublished(uint256 indexed e3Id, bytes publicKey);
}
