// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {IInputValidator} from "@enclave-e3/contracts/contracts/interfaces/IInputValidator.sol";
import {Clone} from "@excubiae/contracts/proxy/Clone.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {IVerifier} from "./CRISPVerifier.sol";

/// @title CRISPInputValidator.
/// @notice Enclave Input Validator
contract CRISPInputValidator is IInputValidator, Clone, Ownable(msg.sender) {
    /// @notice The verifier that will be used to validate the input.
    IVerifier internal noirVerifier;

    /// @notice The governance token address.
    address public token;
    /// @notice The minimum balance required to pass the validation.
    uint256 public balanceThreshold;
    /// @notice The block number at which the balance will be checked.
    uint256 public snapshotBlock;
    /// @notice The Merkle root of the census.
    uint256 public censusMerkleRoot;

    /// @notice Indicates if the the round data has been set.
    bool public isDataSet;

    /// @notice The error emitted when the input data is empty.
    error EmptyInputData();
    /// @notice The error emitted when the input data is invalid.
    error InvalidInputData(bytes reason);
    /// @notice The error emitted when the Noir proof is invalid.
    error InvalidNoirProof();
    /// @notice The error emitted when the round data is not set.
    error RounDataNotSet();
    /// @notice The error emitted when trying to set the round data more than once.
    error RoundDataAlreadySet();

    /// @notice Initializes the contract with appended bytes data for configuration.
    function _initialize() internal virtual override(Clone) {
        super._initialize();

        (address _verifierAddr, address _owner) = abi.decode(
            _getAppendedBytes(),
            (address, address)
        );

        noirVerifier = IVerifier(_verifierAddr);
        _transferOwnership(_owner);
    }

    /// @notice Sets the Merkle root of the census. Can only be set once.
    /// @param _root The Merkle root to set.
    function setRoundData(uint256 _root, address _token, uint256 _balanceThreshold, uint256 _snapshotBlock) external onlyOwner {
        if (isDataSet) revert RoundDataAlreadySet();

        isDataSet = true;
        token = _token;
        balanceThreshold = _balanceThreshold;
        snapshotBlock = _snapshotBlock;
        censusMerkleRoot = _root;
    }

    /// @notice Validates input
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(
        address,
        bytes memory data
    ) external returns (bytes memory input) {
        // we need to ensure that the CRISP admin set the merkle root of the census
        // @todo update this once we have all components working
        // if (!isDataSet) revert RounDataNotSet();

        if (data.length == 0) revert EmptyInputData();

        (
            bytes memory noirProof,
            bytes32[] memory noirPublicInputs,
            bytes memory vote
        ) = abi.decode(data, (bytes, bytes32[], bytes));

        // Check if the ciphertext was encrypted correctly
        if (!noirVerifier.verify(noirProof, noirPublicInputs))
            revert InvalidNoirProof();

        input = vote;
    }
}
