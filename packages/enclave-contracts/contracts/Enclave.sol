// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave, E3, IE3Program } from "./interfaces/IEnclave.sol";
import { ICiphernodeRegistry } from "./interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "./interfaces/IBondingRegistry.sol";
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

    /// @notice Thrown when the E3 start window or computation period has expired.
    error E3Expired();

    /// @notice Thrown when attempting operations on an E3 that has not been activated yet.
    /// @param e3Id The ID of the E3 that is not activated.
    error E3NotActivated(uint256 e3Id);

    /// @notice Thrown when attempting to activate an E3 before its start window begins.
    error E3NotReady();

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

    /// @notice Thrown when attempting to publish input after the computation deadline has passed.
    /// @param e3Id The ID of the E3.
    /// @param expiration The expiration timestamp that has passed.
    error InputDeadlinePassed(uint256 e3Id, uint256 expiration);

    /// @notice Thrown when attempting to publish output before the input deadline has passed.
    /// @param e3Id The ID of the E3.
    /// @param expiration The expiration timestamp that has not yet passed.
    error InputDeadlineNotPassed(uint256 e3Id, uint256 expiration);

    /// @notice Thrown when attempting to set an invalid ciphernode registry address.
    /// @param ciphernodeRegistry The invalid ciphernode registry address.
    error InvalidCiphernodeRegistry(ICiphernodeRegistry ciphernodeRegistry);

    /// @notice Thrown when the requested duration exceeds maxDuration or is zero.
    /// @param duration The invalid duration value.
    error InvalidDuration(uint256 duration);

    /// @notice Thrown when output verification fails.
    /// @param output The invalid output data.
    error InvalidOutput(bytes output);

    /// @notice Thrown when input data is invalid.
    error InvalidInput();

    /// @notice Thrown when the start window parameters are invalid.
    error InvalidStartWindow();

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

    /// @notice Caller not authorized
    error Unauthorized();

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
        uint256 e3Fee = getE3Quote(requestParams);
        require(
            requestParams.threshold[1] >= requestParams.threshold[0] &&
                requestParams.threshold[0] > 0,
            InvalidThreshold(requestParams.threshold)
        );
        require(
            // TODO: do we need a minimum start window to allow time for committee selection?
            requestParams.startWindow[1] >= requestParams.startWindow[0] &&
                requestParams.startWindow[1] >= block.timestamp,
            InvalidStartWindow()
        );
        require(
            requestParams.duration > 0 && requestParams.duration <= maxDuration,
            InvalidDuration(requestParams.duration)
        );
        require(
            e3Programs[requestParams.e3Program],
            E3ProgramNotAllowed(requestParams.e3Program)
        );

        // TODO: should IDs be incremental or produced deterministically?
        e3Id = nexte3Id;
        nexte3Id++;
        uint256 seed = uint256(keccak256(abi.encode(block.prevrandao, e3Id)));

        e3.seed = seed;
        e3.threshold = requestParams.threshold;
        e3.requestBlock = block.number;
        e3.startWindow = requestParams.startWindow;
        e3.duration = requestParams.duration;
        e3.expiration = 0;
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
            requestParams.computeProviderParams
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
        _e3Deadlines[e3Id].committeeDeadline =
            block.timestamp +
            _timeoutConfig.committeeFormationWindow;

        emit E3Requested(e3Id, e3, requestParams.e3Program);
        emit E3StageChanged(e3Id, E3Stage.None, E3Stage.Requested);
    }

    /// @inheritdoc IEnclave
    function activate(uint256 e3Id) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.KeyPublished) {
            revert InvalidStage(e3Id, E3Stage.KeyPublished, current);
        }

        require(e3.startWindow[0] <= block.timestamp, E3NotReady());
        // TODO: handle what happens to the payment if the start window has passed.
        require(e3.startWindow[1] >= block.timestamp, E3Expired());

        bytes32 publicKeyHash = ciphernodeRegistry.committeePublicKey(e3Id);

        uint256 expiresAt = block.timestamp + e3.duration;
        e3s[e3Id].expiration = expiresAt;
        e3s[e3Id].committeePublicKey = publicKeyHash;

        _e3Stages[e3Id] = E3Stage.Activated;
        _e3Deadlines[e3Id].computeDeadline =
            expiresAt +
            _timeoutConfig.computeWindow;

        emit E3Activated(e3Id, expiresAt, publicKeyHash);
        emit E3StageChanged(e3Id, E3Stage.KeyPublished, E3Stage.Activated);

        return true;
    }

    /// @inheritdoc IEnclave
    function publishInput(
        uint256 e3Id,
        bytes calldata data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        // TODO: should we have an input window, including both a start and end timestamp?
        require(
            e3.expiration > block.timestamp,
            InputDeadlinePassed(e3Id, e3.expiration)
        );

        e3.e3Program.validateInput(e3Id, msg.sender, data);

        success = true;
    }

    /// @inheritdoc IEnclave
    function publishCiphertextOutput(
        uint256 e3Id,
        bytes calldata ciphertextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        require(
            e3.expiration <= block.timestamp,
            InputDeadlineNotPassed(e3Id, e3.expiration)
        );
        // TODO: should the output verifier be able to change its mind?
        //i.e. should we be able to call this multiple times?
        require(
            e3.ciphertextOutput == bytes32(0),
            CiphertextOutputAlreadyPublished(e3Id)
        );

        bytes32 ciphertextOutputHash = keccak256(ciphertextOutput);
        e3s[e3Id].ciphertextOutput = ciphertextOutputHash;

        (success) = e3.e3Program.verify(e3Id, ciphertextOutputHash, proof);
        require(success, InvalidOutput(ciphertextOutput));

        // Update lifecycle stage
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.Activated) {
            revert InvalidStage(e3Id, E3Stage.Activated, current);
        }
        _e3Stages[e3Id] = E3Stage.CiphertextReady;
        _e3Deadlines[e3Id].decryptionDeadline =
            block.timestamp +
            _timeoutConfig.decryptionWindow;

        emit CiphertextOutputPublished(e3Id, ciphertextOutput);
        emit E3StageChanged(e3Id, E3Stage.Activated, E3Stage.CiphertextReady);
    }

    /// @inheritdoc IEnclave
    function publishPlaintextOutput(
        uint256 e3Id,
        bytes calldata plaintextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        require(
            e3.ciphertextOutput != bytes32(0),
            CiphertextOutputNotPublished(e3Id)
        );
        require(
            e3.plaintextOutput.length == 0,
            PlaintextOutputAlreadyPublished(e3Id)
        );

        e3s[e3Id].plaintextOutput = plaintextOutput;

        (success) = e3.decryptionVerifier.verify(
            e3Id,
            keccak256(plaintextOutput),
            proof
        );
        require(success, InvalidOutput(plaintextOutput));

        // Update lifecycle stage to Complete
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.CiphertextReady) {
            revert InvalidStage(e3Id, E3Stage.CiphertextReady, current);
        }
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

    /// @notice Distributes rewards to committee members after successful E3 completion.
    /// @dev Divides the E3 payment equally among all committee members and transfers via bonding registry.
    /// @dev Emits RewardsDistributed event upon successful distribution.
    /// @param e3Id The ID of the E3 for which to distribute rewards.
    function _distributeRewards(uint256 e3Id) internal {
        address[] memory committeeNodes = ciphernodeRegistry.getCommitteeNodes(
            e3Id
        );
        uint256 committeeLength = committeeNodes.length;
        uint256[] memory amounts = new uint256[](committeeLength);

        // TODO: do we need to pay different amounts to different nodes?
        // For now, we'll pay the same amount to all nodes.
        uint256 amount = e3Payments[e3Id] / committeeLength;
        for (uint256 i = 0; i < committeeLength; i++) {
            amounts[i] = amount;
        }

        uint256 totalAmount = e3Payments[e3Id];
        e3Payments[e3Id] = 0;

        feeToken.approve(address(bondingRegistry), totalAmount);

        bondingRegistry.distributeRewards(feeToken, committeeNodes, amounts);

        // TODO: decide where does dust go? Treasury maybe?
        feeToken.approve(address(bondingRegistry), 0);

        emit RewardsDistributed(e3Id, committeeNodes, amounts);
    }

    /// @notice Retrieves the honest committee nodes for a given E3.
    /// @dev Determines honest nodes based on failure reason and committee publication status.
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

        // Try to get published committee nodes
        try ciphernodeRegistry.getCommitteeNodes(e3Id) returns (
            address[] memory nodes
        ) {
            // TODO: Implement fault attribution to filter honest from faulting nodes
            return nodes; // Assume all are honest for now
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

    /// @notice Process a failed E3 and calculate refunds
    /// @dev Can be called by anyone once E3 is in failed state
    /// @param e3Id The ID of the failed E3
    function processE3Failure(uint256 e3Id) external {
        E3Stage stage = _e3Stages[e3Id];
        require(stage == E3Stage.Failed, "E3 not failed");

        uint256 payment = e3Payments[e3Id];
        require(payment > 0, "No payment to refund");
        e3Payments[e3Id] = 0; // Prevent double processing

        address[] memory honestNodes = _getHonestNodes(e3Id);

        feeToken.safeTransfer(address(e3RefundManager), payment);
        e3RefundManager.calculateRefund(e3Id, payment, honestNodes);

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
        uint256 e3Id
    ) external onlyCiphernodeRegistry {
        // DKG complete, key published
        E3 memory e3 = e3s[e3Id];
        E3Stage current = _e3Stages[e3Id];
        if (current != E3Stage.CommitteeFinalized) {
            revert InvalidStage(e3Id, E3Stage.CommitteeFinalized, current);
        }
        _e3Stages[e3Id] = E3Stage.KeyPublished;
        _e3Deadlines[e3Id].activationDeadline = e3.startWindow[1];

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
    ) external onlyCiphernodeRegistry {
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
