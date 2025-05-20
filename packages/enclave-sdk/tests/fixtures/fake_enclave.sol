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
}
