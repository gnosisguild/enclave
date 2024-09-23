use alloy::sol;

// Manage an internal web3 instance and express protocol specific behaviour through the events it
// accepts and emits to the EventBus
// Monitor contract events using `contract.events().create_filter()` and rebroadcast to eventbus by
// creating `EnclaveEvent` events
// Delegate signing to a separate actor responsible for managing Eth keys
// Accept eventbus events and forward as appropriate contract calls as required

sol! {
    #[derive(Debug)]
    struct E3 {
        uint256 seed;
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
        uint256 expiration;
        address e3Program;
        bytes e3ProgramParams;
        address inputValidator;
        address decryptionVerifier;
        bytes committeePublicKey;
        bytes ciphertextOutput;
        bytes plaintextOutput;
    }

    #[derive(Debug)]
    event CommitteeRequested(
        uint256 indexed e3Id,
        address filter,
        uint32[2] threshold
    );

    #[derive(Debug)]
    event CiphernodeAdded(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphernodeRemoved(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );

    #[derive(Debug)]
    event E3Requested(
        uint256 e3Id,
        E3 e3,
        address filter,
        address indexed e3Program
    );
}
