// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { IE3Lifecycle } from "./interfaces/IE3Lifecycle.sol";

/**
 * @title E3Lifecycle
 * @notice Manages E3 lifecycle state machine with timeout enforcement
 * @dev Tracks E3 progress through defined stages and enables failure detection
 */
contract E3Lifecycle is IE3Lifecycle, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Authorized caller (typically Enclave contract)
    address public enclave;
    /// @notice Maps E3 ID to its current stage
    mapping(uint256 e3Id => E3Stage) internal _e3Stages;
    /// @notice Maps E3 ID to its deadlines
    mapping(uint256 e3Id => E3Deadlines) internal _e3Deadlines;
    /// @notice Maps E3 ID to failure reason (if failed)
    mapping(uint256 e3Id => FailureReason) internal _e3FailureReasons;
    /// @notice Maps E3 ID to requester address
    mapping(uint256 e3Id => address) internal _e3Requesters;
    /// @notice Global timeout configuration
    E3TimeoutConfig internal _timeoutConfig;
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                       Modifiers                        //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Restricts function to Enclave contract only
    modifier onlyEnclave() {
        if (msg.sender != enclave) revert Unauthorized();
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Constructor that disables initializers
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the E3Lifecycle contract
    /// @param _owner The owner address
    /// @param _enclave The Enclave contract address
    /// @param _config Initial timeout configuration
    function initialize(
        address _owner,
        address _enclave,
        E3TimeoutConfig calldata _config
    ) public initializer {
        __Ownable_init(msg.sender);

        require(_enclave != address(0), "Invalid enclave address");
        enclave = _enclave;

        _setTimeoutConfig(_config);

        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Stage Transitions                     //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3Lifecycle
    function initializeE3(
        uint256 e3Id,
        address requester
    ) external onlyEnclave {
        require(_e3Stages[e3Id] == E3Stage.None, "E3 already exists");

        _e3Stages[e3Id] = E3Stage.Requested;
        _e3Requesters[e3Id] = requester;

        _e3Deadlines[e3Id].committeeDeadline =
            block.timestamp +
            _timeoutConfig.committeeFormationWindow;

        emit E3StageChanged(e3Id, E3Stage.None, E3Stage.Requested);
    }

    /// @inheritdoc IE3Lifecycle
    function onCommitteeFinalized(uint256 e3Id) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.Requested) {
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        }

        _e3Stages[e3Id] = E3Stage.CommitteeFinalized;

        // DKG deadline - committee must complete DKG and publish key by this time
        _e3Deadlines[e3Id].dkgDeadline =
            block.timestamp +
            _timeoutConfig.dkgWindow;

        emit E3StageChanged(
            e3Id,
            E3Stage.Requested,
            E3Stage.CommitteeFinalized
        );
    }

    /// @inheritdoc IE3Lifecycle
    function onKeyPublished(
        uint256 e3Id,
        uint256 activationDeadline
    ) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.CommitteeFinalized) {
            revert InvalidStage(e3Id, E3Stage.CommitteeFinalized, current);
        }

        _e3Stages[e3Id] = E3Stage.KeyPublished;

        // Activation deadline (from Enclave's startWindow[1])
        _e3Deadlines[e3Id].activationDeadline = activationDeadline;

        emit E3StageChanged(
            e3Id,
            E3Stage.CommitteeFinalized,
            E3Stage.KeyPublished
        );
    }

    /// @inheritdoc IE3Lifecycle
    function onActivated(
        uint256 e3Id,
        uint256 expiration
    ) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.KeyPublished) {
            revert InvalidStage(e3Id, E3Stage.KeyPublished, current);
        }

        _e3Stages[e3Id] = E3Stage.Activated;

        // Set compute deadline (expiration + computeWindow)
        // expiration = when inputs close, computeWindow = time for compute provider to finish
        _e3Deadlines[e3Id].computeDeadline =
            expiration +
            _timeoutConfig.computeWindow;

        emit E3StageChanged(e3Id, E3Stage.KeyPublished, E3Stage.Activated);
    }

    /// @inheritdoc IE3Lifecycle
    function onCiphertextPublished(uint256 e3Id) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];
        // Transition from Activated (inputs closed is implicit - time-based)
        if (current != E3Stage.Activated) {
            revert InvalidStage(e3Id, E3Stage.Activated, current);
        }

        _e3Stages[e3Id] = E3Stage.CiphertextReady;

        // Set decryption deadline
        _e3Deadlines[e3Id].decryptionDeadline =
            block.timestamp +
            _timeoutConfig.decryptionWindow;

        emit E3StageChanged(e3Id, E3Stage.Activated, E3Stage.CiphertextReady);
    }

    /// @inheritdoc IE3Lifecycle
    function onComplete(uint256 e3Id) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.CiphertextReady) {
            revert InvalidStage(e3Id, E3Stage.CiphertextReady, current);
        }

        _e3Stages[e3Id] = E3Stage.Complete;

        emit E3StageChanged(e3Id, E3Stage.CiphertextReady, E3Stage.Complete);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Failure Detection                    //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3Lifecycle
    function markE3Failed(
        uint256 e3Id
    ) external returns (FailureReason reason) {
        E3Stage current = _e3Stages[e3Id];

        if (current == E3Stage.None)
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        if (current == E3Stage.Complete) revert E3AlreadyComplete(e3Id);
        if (current == E3Stage.Failed) revert E3AlreadyFailed(e3Id);

        (bool canFail, FailureReason detectedReason) = _checkFailureCondition(
            e3Id,
            current
        );
        if (!canFail) revert FailureConditionNotMet(e3Id);

        _e3Stages[e3Id] = E3Stage.Failed;
        _e3FailureReasons[e3Id] = detectedReason;

        emit E3Failed(e3Id, current, detectedReason);

        return detectedReason;
    }

    /// @inheritdoc IE3Lifecycle
    function markE3FailedWithReason(
        uint256 e3Id,
        FailureReason reason
    ) external onlyEnclave {
        E3Stage current = _e3Stages[e3Id];

        if (current == E3Stage.None)
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        if (current == E3Stage.Complete) revert E3AlreadyComplete(e3Id);
        if (current == E3Stage.Failed) revert E3AlreadyFailed(e3Id);

        _e3Stages[e3Id] = E3Stage.Failed;
        _e3FailureReasons[e3Id] = reason;

        emit E3Failed(e3Id, current, reason);
    }

    /// @inheritdoc IE3Lifecycle
    function checkFailureCondition(
        uint256 e3Id
    ) external view returns (bool canFail, FailureReason reason) {
        E3Stage current = _e3Stages[e3Id];
        return _checkFailureCondition(e3Id, current);
    }

    /// @notice Internal function to check failure conditions
    function _checkFailureCondition(
        uint256 e3Id,
        E3Stage stage
    ) internal view returns (bool canFail, FailureReason reason) {
        E3Deadlines storage d = _e3Deadlines[e3Id];

        if (
            stage == E3Stage.Requested && block.timestamp > d.committeeDeadline
        ) {
            return (true, FailureReason.CommitteeFormationTimeout);
        }
        if (
            stage == E3Stage.CommitteeFinalized &&
            block.timestamp > d.dkgDeadline
        ) {
            return (true, FailureReason.DKGTimeout);
        }
        if (
            stage == E3Stage.KeyPublished &&
            d.activationDeadline > 0 &&
            block.timestamp > d.activationDeadline
        ) {
            return (true, FailureReason.ActivationWindowExpired);
        }
        if (stage == E3Stage.Activated && block.timestamp > d.computeDeadline) {
            return (true, FailureReason.ComputeTimeout);
        }
        if (
            stage == E3Stage.CiphertextReady &&
            block.timestamp > d.decryptionDeadline
        ) {
            return (true, FailureReason.DecryptionTimeout);
        }

        return (false, FailureReason.None);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                    View Functions                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3Lifecycle
    function getE3Stage(uint256 e3Id) external view returns (E3Stage) {
        return _e3Stages[e3Id];
    }

    /// @inheritdoc IE3Lifecycle
    function getFailureReason(
        uint256 e3Id
    ) external view returns (FailureReason) {
        return _e3FailureReasons[e3Id];
    }

    /// @inheritdoc IE3Lifecycle
    function getRequester(uint256 e3Id) external view returns (address) {
        return _e3Requesters[e3Id];
    }

    /// @inheritdoc IE3Lifecycle
    function getDeadlines(
        uint256 e3Id
    ) external view returns (E3Deadlines memory) {
        return _e3Deadlines[e3Id];
    }

    /// @inheritdoc IE3Lifecycle
    function getTimeoutConfig() external view returns (E3TimeoutConfig memory) {
        return _timeoutConfig;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Admin Functions                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3Lifecycle
    function setTimeoutConfig(
        E3TimeoutConfig calldata config
    ) external onlyOwner {
        _setTimeoutConfig(config);
    }

    /// @notice Internal function to set timeout config
    function _setTimeoutConfig(E3TimeoutConfig calldata config) internal {
        require(
            config.committeeFormationWindow > 0,
            "Invalid committee window"
        );
        require(config.dkgWindow > 0, "Invalid DKG window");
        require(config.computeWindow > 0, "Invalid compute window");
        require(config.decryptionWindow > 0, "Invalid decryption window");

        _timeoutConfig = config;

        emit TimeoutConfigUpdated(config);
    }

    /// @notice Set the Enclave contract address
    /// @param _enclave New Enclave address
    function setEnclave(address _enclave) external onlyOwner {
        require(_enclave != address(0), "Invalid enclave address");
        enclave = _enclave;
    }
}
