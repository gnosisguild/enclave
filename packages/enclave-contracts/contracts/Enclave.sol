// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave, E3, IE3Program } from "./interfaces/IEnclave.sol";
import { ICiphernodeRegistry } from "./interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "./interfaces/IBondingRegistry.sol";
import { ISlashingManager } from "./interfaces/ISlashingManager.sol";
import { IE3RefundManager } from "./interfaces/IE3RefundManager.sol";
import { IDecryptionVerifier } from "./interfaces/IDecryptionVerifier.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title Enclave
 * @notice Main contract for managing Encrypted Execution Environments (E3)
 * @dev Coordinates E3 lifecycle including request, activation, input publishing, and output verification
 */
contract Enclave is IEnclave, OwnableUpgradeable {
    using SafeERC20 for IERC20;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Address of the Ciphernode Registry contract.
    /// @dev Manages the pool of ciphernodes and committee selection.
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice Address of the Bonding Registry contract.
    /// @dev Handles staking and reward distribution for ciphernodes.
    IBondingRegistry public bondingRegistry;

    /// @notice E3 Refund Manager contract for handling failed E3 refunds.
    /// @dev Manages refund calculation and claiming for failed E3s.
    IE3RefundManager public e3RefundManager;

    /// @notice Slashing Manager contract for fault attribution.
    /// @dev Used to check which operators have been slashed for E3s.
    ISlashingManager public slashingManager;

    /// @notice Address of the ERC20 token used for E3 fees.
    /// @dev All E3 request fees must be paid in this token.
    IERC20 public feeToken;

    /// @notice Maximum allowed duration for an E3 computation in seconds.
    /// @dev Requests with duration exceeding this value will be rejected.
    uint256 public maxDuration;

    /// @notice ID counter for the next E3 to be created.
    /// @dev Incremented after each successful E3 request.
    uint256 public nexte3Id;

    /// @notice Mapping of allowed E3 Programs.
    /// @dev Only enabled E3 Programs can be used in computation requests.
    mapping(IE3Program e3Program => bool allowed) public e3Programs;

    /// @notice Mapping storing all E3 instances by their ID.
    /// @dev Contains the full state and configuration of each E3.
    mapping(uint256 e3Id => E3 e3) public e3s;

    /// @notice Mapping of enabled encryption schemes to their decryption verifiers.
    /// @dev Each encryption scheme ID maps to a contract that can verify decrypted outputs.
    mapping(bytes32 encryptionSchemeId => IDecryptionVerifier decryptionVerifier)
        public decryptionVerifiers;

    /// @notice Mapping storing valid E3 program ABI encoded parameter sets.
    /// @dev Stores allowed encryption scheme parameters (e.g., BFV parameters).
    mapping(bytes e3ProgramParams => bool allowed) public e3ProgramsParams;

    /// @notice Mapping tracking fee payments for each E3.
    /// @dev Stores the amount paid for an E3, distributed to committee upon completion.
    mapping(uint256 e3Id => uint256 e3Payment) public e3Payments;

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
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Thrown when committee selection fails during E3 request or activation.
    error CommitteeSelectionFailed();

    /// @notice Thrown when an E3 request uses a program that is not enabled.
    /// @param e3Program The E3 program address that is not allowed.
    error E3ProgramNotAllowed(IE3Program e3Program);

    /// @notice Thrown when attempting to access an E3 that does not exist.
    /// @param e3Id The ID of the non-existent E3.
    error E3DoesNotExist(uint256 e3Id);

    /// @notice Thrown when attempting to enable a module or program that is already enabled.
    /// @param module The address of the module that is already enabled.
    error ModuleAlreadyEnabled(address module);

    /// @notice Thrown when attempting to disable a module or program that is not enabled.
    /// @param module The address of the module that is not enabled.
    error ModuleNotEnabled(address module);

    /// @notice Thrown when an invalid or disabled encryption scheme is used.
    /// @param encryptionSchemeId The ID of the invalid encryption scheme.
    error InvalidEncryptionScheme(bytes32 encryptionSchemeId);

    /// @notice Thrown when attempting to set an invalid ciphernode registry address.
    /// @param ciphernodeRegistry The invalid ciphernode registry address.
    error InvalidCiphernodeRegistry(ICiphernodeRegistry ciphernodeRegistry);

    /// @notice Thrown when the requested duration exceeds maxDuration or is zero.
    /// @param duration The invalid duration value.
    error InvalidDuration(uint256 duration);

    /// @notice Thrown when output verification fails.
    /// @param output The invalid output data.
    error InvalidOutput(bytes output);

    /// @notice Thrown when the threshold parameters are invalid (e.g., M > N or M = 0).
    /// @param threshold The invalid threshold array [M, N].
    error InvalidThreshold(uint32[2] threshold);

    /// @notice Thrown when attempting to publish ciphertext output that has already been published.
    /// @param e3Id The ID of the E3.
    error CiphertextOutputAlreadyPublished(uint256 e3Id);

    /// @notice Thrown when attempting to publish plaintext output before ciphertext output.
    /// @param e3Id The ID of the E3.
    error CiphertextOutputNotPublished(uint256 e3Id);

    /// @notice Thrown when payment is required but not provided or insufficient.
    /// @param value The required payment amount.
    error PaymentRequired(uint256 value);

    /// @notice Thrown when attempting to publish plaintext output that has already been published.
    /// @param e3Id The ID of the E3.
    error PlaintextOutputAlreadyPublished(uint256 e3Id);

    /// @notice Thrown when attempting to set an invalid bonding registry address.
    /// @param bondingRegistry The invalid bonding registry address.
    error InvalidBondingRegistry(IBondingRegistry bondingRegistry);

    /// @notice Thrown when attempting to set an invalid fee token address.
    /// @param feeToken The invalid fee token address.
    error InvalidFeeToken(IERC20 feeToken);

    /// @notice E3 is not in expected stage
    error InvalidStage(uint256 e3Id, E3Stage expected, E3Stage actual);

    /// @notice E3 has already been marked as failed
    error E3AlreadyFailed(uint256 e3Id);

    /// @notice E3 has already completed
    error E3AlreadyComplete(uint256 e3Id);

    /// @notice Failure condition not yet met
    error FailureConditionNotMet(uint256 e3Id);

    /// @notice The Input deadline is invalid
    error InvalidInputDeadline(uint256 deadline);

    /// @notice The input deadline start is in the past
    error InvalidInputDeadlineStart(uint256 start);
    /// @notice The input deadline end is before the start
    error InvalidInputDeadlineEnd(uint256 end);

    /// @notice The duties are completed, and ciphernodes are not required to act anymore for this E3
    /// @param e3Id The ID of the E3
    /// @param expiration The expiration timestamp of the E3
    error CommitteeDutiesCompleted(uint256 e3Id, uint256 expiration);

    /// @notice The input deadline has not yet been reached
    /// @param e3Id The ID of the E3
    /// @param inputDeadline The input deadline timestamp of the E3
    error InputDeadlineNotReached(uint256 e3Id, uint256 inputDeadline);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                       Modifiers                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Restricts function to CiphernodeRegistry contract only
    modifier onlyCiphernodeRegistry() {
        require(
            msg.sender == address(ciphernodeRegistry),
            "Only CiphernodeRegistry"
        );
        _;
    }

    /// @notice Restricts function to CiphernodeRegistry or SlashingManager
    modifier onlyCiphernodeRegistryOrSlashingManager() {
        require(
            msg.sender == address(ciphernodeRegistry) ||
                msg.sender == address(slashingManager),
            "Only CiphernodeRegistry or SlashingManager"
        );
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    ////////////////////////////////////////////////////////////

    /// @notice Constructor that disables initializers.
    /// @dev Prevents the implementation contract from being initialized. Initialization is performed
    /// via the initialize() function when deployed behind a proxy.
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the Enclave contract with initial configuration.
    /// @dev This function can only be called once due to the initializer modifier. Sets up core dependencies.
    /// @param _owner The owner address of this contract.
    /// @param _ciphernodeRegistry The address of the Ciphernode Registry contract.
    /// @param _bondingRegistry The address of the Bonding Registry contract.
    /// @param _e3RefundManager The address of the E3 Refund Manager contract.
    /// @param _feeToken The address of the ERC20 token used for E3 fees.
    /// @param _maxDuration The maximum duration of a computation in seconds.
    /// @param config Initial timeout configuration for E3 lifecycle stages.
    /// @param _e3ProgramsParams Array of ABI encoded E3 encryption scheme parameters sets (e.g., for BFV).
    function initialize(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        IBondingRegistry _bondingRegistry,
        IE3RefundManager _e3RefundManager,
        IERC20 _feeToken,
        uint256 _maxDuration,
        E3TimeoutConfig calldata config,
        bytes[] memory _e3ProgramsParams
    ) public initializer {
        __Ownable_init(msg.sender);
        setMaxDuration(_maxDuration);
        setCiphernodeRegistry(_ciphernodeRegistry);
        setBondingRegistry(_bondingRegistry);
        setE3RefundManager(_e3RefundManager);
        setFeeToken(_feeToken);
        _setTimeoutConfig(config);
        setE3ProgramsParams(_e3ProgramsParams);
        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc IEnclave
    function request(
        E3RequestParams calldata requestParams
    ) external returns (uint256 e3Id, E3 memory e3) {
        // check whether the threshold config is valid
        require(
            requestParams.threshold[1] >= requestParams.threshold[0] &&
                requestParams.threshold[0] > 0,
            InvalidThreshold(requestParams.threshold)
        );

        // input start date should be in the future
        require(
            requestParams.inputWindow[0] >= block.timestamp,
            // &&
            // requestParams.inputWindow[0] >= block.timestamp +
            //     _timeoutConfig.dkgWindow,
            InvalidInputDeadlineStart(requestParams.inputWindow[0])
        );
        // the end of the input window should be after the start
        require(
            requestParams.inputWindow[1] >= requestParams.inputWindow[0],
            InvalidInputDeadlineEnd(requestParams.inputWindow[1])
        );

        // The total duration cannot be > maxDuration
        uint256 totalDuration = requestParams.inputWindow[1] -
            block.timestamp +
            _timeoutConfig.computeWindow +
            _timeoutConfig.decryptionWindow;
        // TODO do we actually need a max duration?
        require(totalDuration < maxDuration, InvalidDuration(totalDuration));

        require(
            e3Programs[requestParams.e3Program],
            E3ProgramNotAllowed(requestParams.e3Program)
        );

        uint256 e3Fee = getE3Quote(requestParams);

        e3Id = nexte3Id;
        nexte3Id++;
        uint256 seed = uint256(keccak256(abi.encode(block.prevrandao, e3Id)));

        e3.seed = seed;
        e3.threshold = requestParams.threshold;
        e3.requestBlock = block.number;
        e3.inputWindow = requestParams.inputWindow;
        e3.e3Program = requestParams.e3Program;
        e3.e3ProgramParams = requestParams.e3ProgramParams;
        e3.customParams = requestParams.customParams;
        e3.committeePublicKey = hex"";
        e3.ciphertextOutput = hex"";
        e3.plaintextOutput = hex"";
        e3.requester = msg.sender;

        bytes32 encryptionSchemeId = requestParams.e3Program.validate(
            e3Id,
            seed,
            requestParams.e3ProgramParams,
            requestParams.computeProviderParams,
            requestParams.customParams
        );
        IDecryptionVerifier decryptionVerifier = decryptionVerifiers[
            encryptionSchemeId
        ];

        require(
            decryptionVerifiers[encryptionSchemeId] !=
                IDecryptionVerifier(address(0)),
            InvalidEncryptionScheme(encryptionSchemeId)
        );

        e3.encryptionSchemeId = encryptionSchemeId;
        e3.decryptionVerifier = decryptionVerifier;

        e3s[e3Id] = e3;
        e3Payments[e3Id] = e3Fee;

        feeToken.safeTransferFrom(msg.sender, address(this), e3Fee);

        require(
            ciphernodeRegistry.requestCommittee(
                e3Id,
                seed,
                requestParams.threshold
            ),
            CommitteeSelectionFailed()
        );

        // Initialize E3 lifecycle
        _e3Stages[e3Id] = E3Stage.Requested;
        _e3Requesters[e3Id] = msg.sender;

        // the compute deadline is end of input window + compute window
        _e3Deadlines[e3Id].computeDeadline =
            e3.inputWindow[1] +
            _timeoutConfig.computeWindow;

        emit E3Requested(e3Id, e3, requestParams.e3Program);
        emit E3StageChanged(e3Id, E3Stage.None, E3Stage.Requested);
    }

    /// @inheritdoc IEnclave
    function publishCiphertextOutput(
        uint256 e3Id,
        bytes calldata ciphertextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        E3Deadlines memory deadlines = _e3Deadlines[e3Id];

        // You cannot post outputs after the compute deadline
        require(
            deadlines.computeDeadline >= block.timestamp,
            CommitteeDutiesCompleted(e3Id, deadlines.computeDeadline)
        );

        // The program need to have stopped accepting inputs
        require(
            block.timestamp >= e3.inputWindow[1],
            InputDeadlineNotReached(e3Id, e3.inputWindow[1])
        );

        // For now we only accept one output
        require(
            e3.ciphertextOutput == bytes32(0),
            CiphertextOutputAlreadyPublished(e3Id)
        );

        bytes32 ciphertextOutputHash = keccak256(ciphertextOutput);
        e3s[e3Id].ciphertextOutput = ciphertextOutputHash;

        (success) = e3.e3Program.verify(e3Id, ciphertextOutputHash, proof);
        require(success, InvalidOutput(ciphertextOutput));

        // Update lifecycle stage
        _e3Stages[e3Id] = E3Stage.CiphertextReady;
        _e3Deadlines[e3Id].decryptionDeadline =
            block.timestamp +
            _timeoutConfig.decryptionWindow;

        emit CiphertextOutputPublished(e3Id, ciphertextOutput);
        emit E3StageChanged(
            e3Id,
            E3Stage.KeyPublished,
            E3Stage.CiphertextReady
        );
    }

    /// @inheritdoc IEnclave
    function publishPlaintextOutput(
        uint256 e3Id,
        bytes calldata plaintextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Check we are in the right stage
        // no need to check if there's a ciphertext as we would not
        // be in this stage otherwise
        E3Stage current = _e3Stages[e3Id];
        require(
            current == E3Stage.CiphertextReady,
            InvalidStage(e3Id, E3Stage.CiphertextReady, current)
        );

        // you cannot post a decryption after the decryption deadline
        E3Deadlines memory deadlines = _e3Deadlines[e3Id];
        require(
            deadlines.decryptionDeadline >= block.timestamp,
            CommitteeDutiesCompleted(e3Id, deadlines.decryptionDeadline)
        );

        e3s[e3Id].plaintextOutput = plaintextOutput;

        (success) = e3.decryptionVerifier.verify(
            e3Id,
            keccak256(plaintextOutput),
            proof
        );
        require(success, InvalidOutput(plaintextOutput));

        // Update lifecycle stage to Complete
        _e3Stages[e3Id] = E3Stage.Complete;

        _distributeRewards(e3Id);

        emit PlaintextOutputPublished(e3Id, plaintextOutput);
        emit E3StageChanged(e3Id, E3Stage.CiphertextReady, E3Stage.Complete);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Internal Functions                   //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Distributes rewards to active committee members after successful E3 completion.
    /// @dev Uses active committee nodes (excluding expelled members).
    ///      Divides the E3 payment equally among active members and transfers via bonding registry.
    ///      If no active members remain (e.g., all expelled), refunds the requester to prevent fund lockup.
    /// @param e3Id The ID of the E3 for which to distribute rewards.
    function _distributeRewards(uint256 e3Id) internal {
        address[] memory activeNodes = ciphernodeRegistry
            .getActiveCommitteeNodes(e3Id);
        uint256 activeLength = activeNodes.length;

        uint256 totalAmount = e3Payments[e3Id];
        e3Payments[e3Id] = 0;
        if (totalAmount == 0) return;

        if (activeLength == 0) {
            address requester = _e3Requesters[e3Id];
            if (requester != address(0)) {
                feeToken.safeTransfer(requester, totalAmount);
            }
            return;
        }

        uint256[] memory amounts = new uint256[](activeLength);

        // Distribute equally among active (non-expelled) committee members
        uint256 amount = totalAmount / activeLength;
        for (uint256 i = 0; i < activeLength; i++) {
            amounts[i] = amount;
        }

        feeToken.approve(address(bondingRegistry), totalAmount);

        bondingRegistry.distributeRewards(feeToken, activeNodes, amounts);

        // Dust goes to treasury (implicit via remaining approval)
        feeToken.approve(address(bondingRegistry), 0);

        emit RewardsDistributed(e3Id, activeNodes, amounts);
    }

    /// @notice Retrieves the honest committee nodes for a given E3.
    /// @dev Uses active committee view from the registry (which excludes expelled/slashed members).
    /// @param e3Id The ID of the E3.
    /// @return honestNodes An array of addresses of honest committee nodes.
    function _getHonestNodes(
        uint256 e3Id
    ) private view returns (address[] memory) {
        FailureReason reason = _e3FailureReasons[e3Id];

        // Early failures have no committee
        if (
            reason == FailureReason.CommitteeFormationTimeout ||
            reason == FailureReason.InsufficientCommitteeMembers
        ) {
            return new address[](0);
        }

        // Use active committee nodes (already filtered by expulsion)
        try ciphernodeRegistry.getActiveCommitteeNodes(e3Id) returns (
            address[] memory nodes
        ) {
            return nodes;
        } catch {
            return new address[](0); // Committee not published (DKG failed)
        }
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc IEnclave
    function setMaxDuration(uint256 _maxDuration) public onlyOwner {
        maxDuration = _maxDuration;
        emit MaxDurationSet(_maxDuration);
    }

    /// @inheritdoc IEnclave
    function setCiphernodeRegistry(
        ICiphernodeRegistry _ciphernodeRegistry
    ) public onlyOwner {
        require(
            address(_ciphernodeRegistry) != address(0) &&
                _ciphernodeRegistry != ciphernodeRegistry,
            InvalidCiphernodeRegistry(_ciphernodeRegistry)
        );
        ciphernodeRegistry = _ciphernodeRegistry;
        emit CiphernodeRegistrySet(address(_ciphernodeRegistry));
    }

    /// @inheritdoc IEnclave
    function setBondingRegistry(
        IBondingRegistry _bondingRegistry
    ) public onlyOwner {
        require(
            address(_bondingRegistry) != address(0) &&
                _bondingRegistry != bondingRegistry,
            InvalidBondingRegistry(_bondingRegistry)
        );
        bondingRegistry = _bondingRegistry;
        emit BondingRegistrySet(address(_bondingRegistry));
    }

    /// @inheritdoc IEnclave
    function setFeeToken(IERC20 _feeToken) public onlyOwner {
        require(
            address(_feeToken) != address(0) && _feeToken != feeToken,
            InvalidFeeToken(_feeToken)
        );
        feeToken = _feeToken;
        emit FeeTokenSet(address(_feeToken));
    }

    /// @inheritdoc IEnclave
    function enableE3Program(IE3Program e3Program) public onlyOwner {
        require(
            !e3Programs[e3Program],
            ModuleAlreadyEnabled(address(e3Program))
        );
        e3Programs[e3Program] = true;
        emit E3ProgramEnabled(e3Program);
    }

    /// @inheritdoc IEnclave
    function disableE3Program(IE3Program e3Program) public onlyOwner {
        require(e3Programs[e3Program], ModuleNotEnabled(address(e3Program)));
        delete e3Programs[e3Program];
        emit E3ProgramDisabled(e3Program);
    }

    /// @inheritdoc IEnclave
    function setDecryptionVerifier(
        bytes32 encryptionSchemeId,
        IDecryptionVerifier decryptionVerifier
    ) public onlyOwner {
        require(
            decryptionVerifier != IDecryptionVerifier(address(0)) &&
                decryptionVerifiers[encryptionSchemeId] != decryptionVerifier,
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = decryptionVerifier;
        emit EncryptionSchemeEnabled(encryptionSchemeId);
    }

    /// @inheritdoc IEnclave
    function disableEncryptionScheme(
        bytes32 encryptionSchemeId
    ) public onlyOwner {
        require(
            decryptionVerifiers[encryptionSchemeId] !=
                IDecryptionVerifier(address(0)),
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = IDecryptionVerifier(
            address(0)
        );
        emit EncryptionSchemeDisabled(encryptionSchemeId);
    }

    /// @inheritdoc IEnclave
    function setE3ProgramsParams(
        bytes[] memory _e3ProgramsParams
    ) public onlyOwner {
        uint256 length = _e3ProgramsParams.length;
        for (uint256 i; i < length; ) {
            e3ProgramsParams[_e3ProgramsParams[i]] = true;
            unchecked {
                ++i;
            }
        }
        emit AllowedE3ProgramsParamsSet(_e3ProgramsParams);
    }

    /// @notice Sets the E3 Refund Manager contract address
    /// @param _e3RefundManager The new E3 Refund Manager contract address
    function setE3RefundManager(
        IE3RefundManager _e3RefundManager
    ) public onlyOwner {
        require(
            address(_e3RefundManager) != address(0),
            "Invalid E3RefundManager address"
        );
        e3RefundManager = _e3RefundManager;
        emit E3RefundManagerSet(address(_e3RefundManager));
    }

    /// @notice Sets the Slashing Manager contract address
    /// @param _slashingManager The new Slashing Manager contract address
    function setSlashingManager(
        ISlashingManager _slashingManager
    ) public onlyOwner {
        require(
            address(_slashingManager) != address(0),
            "Invalid SlashingManager address"
        );
        slashingManager = _slashingManager;
    }

    /// @notice Process a failed E3 and calculate refunds
    /// @dev Can be called by anyone once E3 is in failed state.
    ///      Passes the current feeToken so the refund manager stores the correct token per-E3.
    /// @param e3Id The ID of the failed E3
    function processE3Failure(uint256 e3Id) external {
        E3Stage stage = _e3Stages[e3Id];
        require(stage == E3Stage.Failed, "E3 not failed");

        uint256 payment = e3Payments[e3Id];
        require(payment > 0, "No payment to refund");
        e3Payments[e3Id] = 0; // Prevent double processing

        address[] memory honestNodes = _getHonestNodes(e3Id);

        feeToken.safeTransfer(address(e3RefundManager), payment);
        e3RefundManager.calculateRefund(e3Id, payment, honestNodes, feeToken);

        emit E3FailureProcessed(e3Id, payment, honestNodes.length);
    }

    /// @inheritdoc IEnclave
    function onCommitteeFinalized(
        uint256 e3Id
    ) external onlyCiphernodeRegistry {
        // Update E3 lifecycle stage - committee finalized, DKG starting
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.Requested) {
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        }
        _e3Stages[e3Id] = E3Stage.CommitteeFinalized;
        _e3Deadlines[e3Id].dkgDeadline =
            block.timestamp +
            _timeoutConfig.dkgWindow;

        emit CommitteeFinalized(e3Id);
        emit E3StageChanged(
            e3Id,
            E3Stage.Requested,
            E3Stage.CommitteeFinalized
        );
    }

    /// @inheritdoc IEnclave
    function onCommitteePublished(
        uint256 e3Id,
        bytes32 committeePublicKeyHash
    ) external onlyCiphernodeRegistry {
        // DKG complete, key published
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.CommitteeFinalized) {
            revert InvalidStage(e3Id, E3Stage.CommitteeFinalized, current);
        }
        _e3Stages[e3Id] = E3Stage.KeyPublished;

        e3s[e3Id].committeePublicKey = committeePublicKeyHash;

        emit CommitteeFormed(e3Id);
        emit E3StageChanged(
            e3Id,
            E3Stage.CommitteeFinalized,
            E3Stage.KeyPublished
        );
    }

    /// @inheritdoc IEnclave
    function onE3Failed(
        uint256 e3Id,
        uint8 reason
    ) external onlyCiphernodeRegistryOrSlashingManager {
        require(reason > 0 && reason <= 12, "Invalid failure reason");
        // Mark E3 as failed with the given reason
        _markE3FailedWithReason(e3Id, FailureReason(reason));
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Lifecycle Functions                  //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Anyone can mark an E3 as failed if timeout passed
    /// @param e3Id The E3 ID
    /// @return reason The failure reason
    function markE3Failed(
        uint256 e3Id
    ) external returns (FailureReason reason) {
        E3Stage current = _e3Stages[e3Id];

        if (current == E3Stage.None)
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        if (current == E3Stage.Complete) revert E3AlreadyComplete(e3Id);
        if (current == E3Stage.Failed) revert E3AlreadyFailed(e3Id);

        bool canFail;
        (canFail, reason) = _checkFailureCondition(e3Id, current);
        if (!canFail) revert FailureConditionNotMet(e3Id);

        _e3Stages[e3Id] = E3Stage.Failed;
        _e3FailureReasons[e3Id] = reason;

        emit E3StageChanged(e3Id, current, E3Stage.Failed);
        emit E3Failed(e3Id, current, reason);
    }

    /// @notice Internal function to mark E3 as failed with specific reason
    /// @param e3Id The E3 ID
    /// @param reason The failure reason
    function _markE3FailedWithReason(
        uint256 e3Id,
        FailureReason reason
    ) internal {
        E3Stage current = _e3Stages[e3Id];

        if (current == E3Stage.None)
            revert InvalidStage(e3Id, E3Stage.Requested, current);
        if (current == E3Stage.Complete) revert E3AlreadyComplete(e3Id);
        if (current == E3Stage.Failed) revert E3AlreadyFailed(e3Id);

        _e3Stages[e3Id] = E3Stage.Failed;
        _e3FailureReasons[e3Id] = reason;

        emit E3StageChanged(e3Id, current, E3Stage.Failed);
        emit E3Failed(e3Id, current, reason);
    }

    /// @notice Check if E3 can be marked as failed
    /// @param e3Id The E3 ID
    /// @return canFail Whether failure condition is met
    /// @return reason The failure reason if applicable
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
        E3Deadlines memory d = _e3Deadlines[e3Id];

        uint256 committeeDeadline = ciphernodeRegistry.getCommitteeDeadline(
            e3Id
        );

        if (stage == E3Stage.Requested && block.timestamp > committeeDeadline) {
            return (true, FailureReason.CommitteeFormationTimeout);
        }
        if (
            stage == E3Stage.CommitteeFinalized &&
            block.timestamp > d.dkgDeadline
        ) {
            return (true, FailureReason.DKGTimeout);
        }
        if (
            stage == E3Stage.KeyPublished && block.timestamp > d.computeDeadline
        ) {
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

    /// @notice Get current stage of an E3
    /// @param e3Id The E3 ID
    /// @return stage The current stage
    function getE3Stage(uint256 e3Id) external view returns (E3Stage stage) {
        return _e3Stages[e3Id];
    }

    /// @notice Get failure reason for an E3
    /// @param e3Id The E3 ID
    /// @return reason The failure reason
    function getFailureReason(
        uint256 e3Id
    ) external view returns (FailureReason reason) {
        return _e3FailureReasons[e3Id];
    }

    /// @notice Get requester address for an E3
    /// @param e3Id The E3 ID
    /// @return requester The requester address
    function getRequester(
        uint256 e3Id
    ) external view returns (address requester) {
        return _e3Requesters[e3Id];
    }

    /// @notice Get deadlines for an E3
    /// @param e3Id The E3 ID
    /// @return deadlines The E3 deadlines
    function getDeadlines(
        uint256 e3Id
    ) external view returns (E3Deadlines memory deadlines) {
        return _e3Deadlines[e3Id];
    }

    /// @notice Get timeout configuration
    /// @return config The current timeout config
    function getTimeoutConfig()
        external
        view
        returns (E3TimeoutConfig memory config)
    {
        return _timeoutConfig;
    }

    /// @notice Set timeout configuration
    /// @param config The new timeout config
    function setTimeoutConfig(
        E3TimeoutConfig calldata config
    ) external onlyOwner {
        _setTimeoutConfig(config);
    }

    /// @notice Internal function to set timeout config
    function _setTimeoutConfig(E3TimeoutConfig calldata config) internal {
        require(config.dkgWindow > 0, "Invalid DKG window");
        require(config.computeWindow > 0, "Invalid compute window");
        require(config.decryptionWindow > 0, "Invalid decryption window");
        require(config.gracePeriod > 0, "Invalid grace period");

        _timeoutConfig = config;

        emit TimeoutConfigUpdated(config);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc IEnclave
    function getE3(uint256 e3Id) public view returns (E3 memory e3) {
        e3 = e3s[e3Id];
        require(e3.e3Program != IE3Program(address(0)), E3DoesNotExist(e3Id));
    }

    /// @inheritdoc IEnclave
    function getE3Quote(
        E3RequestParams calldata
    ) public pure returns (uint256 fee) {
        fee = 1 * 10 ** 6;
        require(fee > 0, PaymentRequired(fee));
    }

    /// @inheritdoc IEnclave
    function getDecryptionVerifier(
        bytes32 encryptionSchemeId
    ) public view returns (IDecryptionVerifier) {
        return decryptionVerifiers[encryptionSchemeId];
    }
}
