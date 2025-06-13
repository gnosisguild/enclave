pragma solidity >=0.4.24;

contract FakeEnclave {
    event E3Activated(
        uint256 e3Id,
        uint256 expiration,
        bytes committeePublicKey
    );
    event InputPublished(
        uint256 indexed e3Id,
        bytes data,
        uint256 inputHash,
        uint256 index
    );
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);
    event CommitteePublished(uint256 indexed e3Id, bytes publicKey);

    // Emit E3Activated event with passed test data
    function emitE3Activated(
        uint256 e3Id,
        uint256 expiration,
        bytes memory committeePublicKey
    ) public {
        emit E3Activated(e3Id, expiration, committeePublicKey);
    }

    // Emit InputPublished event with passed test data
    function emitInputPublished(
        uint256 e3Id,
        bytes memory data,
        uint256 inputHash,
        uint256 index
    ) public {
        emit InputPublished(e3Id, data, inputHash, index);
    }

    // Emit CiphertextOutputPublished event with passed test data
    function emitCiphertextOutputPublished(
        uint256 e3Id,
        bytes memory ciphertextOutput
    ) public {
        emit CiphertextOutputPublished(e3Id, ciphertextOutput);
    }

    // Emit PlaintextOutputPublished event with passed test data
    function emitPlaintextOutputPublished(
        uint256 e3Id,
        bytes memory plaintextOutput
    ) public {
        emit PlaintextOutputPublished(e3Id, plaintextOutput);
    }

    // Emit CommitteePublished event with passed test data
    function emitCommitteePublished(
        uint256 e3Id,
        bytes memory publicKey
    ) public {
        emit CommitteePublished(e3Id, publicKey);
    }

    function getE3(uint256 _e3Id) external view returns (E3 memory e3) {
        e3 = E3({
            seed: 123456789012,
            threshold: [uint32(2), uint32(3)],
            requestBlock: 18750000,
            startWindow: [uint256(18750100), uint256(18750200)],
            duration: 100,
            expiration: block.timestamp + 1 days,
            encryptionSchemeId: bytes32(keccak256("AES-256-GCM")),
            e3Program: 0x7F3E4df648B8Cb96C1D343be976b91B97CaD5c21,
            inputValidator: 0xA51D5E87c0C82dDEBfa4E7E515B2D8Eea8f3e4f2,
            decryptionVerifier: 0x4B0D8c2E5f7a6c832f8b16d3aB0e7F5d9E9B24b1,
            e3ProgramParams: abi.encode(42, "testParams"),
            committeePublicKey: bytes32(keccak256("committee_public_key")),
            ciphertextOutput: bytes32(keccak256("encrypted_data")),
            plaintextOutput: abi.encode("decrypted_result")
        });
    }
}

struct E3 {
    uint256 seed;
    uint32[2] threshold;
    uint256 requestBlock;
    uint256[2] startWindow;
    uint256 duration;
    uint256 expiration;
    bytes32 encryptionSchemeId;
    address e3Program;
    bytes e3ProgramParams;
    address inputValidator;
    address decryptionVerifier;
    bytes32 committeePublicKey;
    bytes32 ciphertextOutput;
    bytes plaintextOutput;
}
