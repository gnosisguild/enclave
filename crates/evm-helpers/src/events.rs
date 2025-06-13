use alloy::sol;

// TODO: extract these from that actual contract

sol! {
    #[derive(Debug)]
    event E3Activated(uint256 e3Id, uint256 expiration, bytes committeePublicKey);

    #[derive(Debug)]
    event InputPublished(uint256 indexed e3Id, bytes data, uint256 inputHash, uint256 index);

    #[derive(Debug)]
    event CiphertextOutputPublished(uint256 indexed e3Id, bytes ciphertextOutput);

    #[derive(Debug)]
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);

    #[derive(Debug)]
    event CommitteePublished(uint256 indexed e3Id, bytes publicKey);
}
