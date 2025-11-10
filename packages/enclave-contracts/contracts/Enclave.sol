// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave, E3, IE3Program } from "./interfaces/IEnclave.sol";
import { ICiphernodeRegistry } from "./interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "./interfaces/IBondingRegistry.sol";
import { IDecryptionVerifier } from "./interfaces/IDecryptionVerifier.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    InternalLeanIMT,
    LeanIMTData,
    PoseidonT3
} from "@zk-kit/lean-imt.sol/InternalLeanIMT.sol";

/**
 * @title Enclave
 * @notice Main contract for managing Encrypted Execution Environments (E3)
 * @dev Coordinates E3 lifecycle including request, activation, input publishing, and output verification
 */
contract Enclave is IEnclave, OwnableUpgradeable {
    using InternalLeanIMT for LeanIMTData;
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

    /// @notice Thrown when attempting to activate an E3 that is already activated.
    /// @param e3Id The ID of the E3 that is already activated.
    error E3AlreadyActivated(uint256 e3Id);

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

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
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
    /// @param _feeToken The address of the ERC20 token used for E3 fees.
    /// @param _maxDuration The maximum duration of a computation in seconds.
    /// @param _e3ProgramsParams Array of ABI encoded E3 encryption scheme parameters sets (e.g., for BFV).
    function initialize(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        IBondingRegistry _bondingRegistry,
        IERC20 _feeToken,
        uint256 _maxDuration,
        bytes[] memory _e3ProgramsParams
    ) public initializer {
        __Ownable_init(msg.sender);
        setMaxDuration(_maxDuration);
        setCiphernodeRegistry(_ciphernodeRegistry);
        setBondingRegistry(_bondingRegistry);
        setFeeToken(_feeToken);
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

        emit E3Requested(e3Id, e3, requestParams.e3Program);
    }

    /// @inheritdoc IEnclave
    function activate(
        uint256 e3Id,
        bytes calldata publicKey
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        require(e3.expiration == 0, E3AlreadyActivated(e3Id));
        require(e3.startWindow[0] <= block.timestamp, E3NotReady());
        // TODO: handle what happens to the payment if the start window has passed.
        require(e3.startWindow[1] >= block.timestamp, E3Expired());

        bytes32 publicKeyHash = ciphernodeRegistry.committeePublicKey(e3Id);
        require(
            keccak256(publicKey) == publicKeyHash,
            CommitteeSelectionFailed()
        );
        uint256 expiresAt = block.timestamp + e3.duration;
        e3s[e3Id].expiration = expiresAt;
        e3s[e3Id].committeePublicKey = keccak256(publicKey);

        emit E3Activated(e3Id, expiresAt, publicKey);

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

        e3.inputValidator.validate(msg.sender, data);

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

        emit CiphertextOutputPublished(e3Id, ciphertextOutput);
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

        _distributeRewards(e3Id);

        emit PlaintextOutputPublished(e3Id, plaintextOutput);
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

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc IEnclave
    function setMaxDuration(
        uint256 _maxDuration
    ) public onlyOwner returns (bool success) {
        maxDuration = _maxDuration;
        success = true;
        emit MaxDurationSet(_maxDuration);
    }

    /// @inheritdoc IEnclave
    function setCiphernodeRegistry(
        ICiphernodeRegistry _ciphernodeRegistry
    ) public onlyOwner returns (bool success) {
        require(
            address(_ciphernodeRegistry) != address(0) &&
                _ciphernodeRegistry != ciphernodeRegistry,
            InvalidCiphernodeRegistry(_ciphernodeRegistry)
        );
        ciphernodeRegistry = _ciphernodeRegistry;
        success = true;
        emit CiphernodeRegistrySet(address(_ciphernodeRegistry));
    }

    /// @inheritdoc IEnclave
    function setBondingRegistry(
        IBondingRegistry _bondingRegistry
    ) public onlyOwner returns (bool success) {
        require(
            address(_bondingRegistry) != address(0) &&
                _bondingRegistry != bondingRegistry,
            InvalidBondingRegistry(_bondingRegistry)
        );
        bondingRegistry = _bondingRegistry;
        success = true;
        emit BondingRegistrySet(address(_bondingRegistry));
    }

    /// @inheritdoc IEnclave
    function setFeeToken(
        IERC20 _feeToken
    ) public onlyOwner returns (bool success) {
        require(
            address(_feeToken) != address(0) && _feeToken != feeToken,
            InvalidFeeToken(_feeToken)
        );
        feeToken = _feeToken;
        success = true;
        emit FeeTokenSet(address(_feeToken));
    }

    /// @inheritdoc IEnclave
    function enableE3Program(
        IE3Program e3Program
    ) public onlyOwner returns (bool success) {
        require(
            !e3Programs[e3Program],
            ModuleAlreadyEnabled(address(e3Program))
        );
        e3Programs[e3Program] = true;
        success = true;
        emit E3ProgramEnabled(e3Program);
    }

    /// @inheritdoc IEnclave
    function disableE3Program(
        IE3Program e3Program
    ) public onlyOwner returns (bool success) {
        require(e3Programs[e3Program], ModuleNotEnabled(address(e3Program)));
        delete e3Programs[e3Program];
        success = true;
        emit E3ProgramDisabled(e3Program);
    }

    /// @inheritdoc IEnclave
    function setDecryptionVerifier(
        bytes32 encryptionSchemeId,
        IDecryptionVerifier decryptionVerifier
    ) public onlyOwner returns (bool success) {
        require(
            decryptionVerifier != IDecryptionVerifier(address(0)) &&
                decryptionVerifiers[encryptionSchemeId] != decryptionVerifier,
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = decryptionVerifier;
        success = true;
        emit EncryptionSchemeEnabled(encryptionSchemeId);
    }

    /// @inheritdoc IEnclave
    function disableEncryptionScheme(
        bytes32 encryptionSchemeId
    ) public onlyOwner returns (bool success) {
        require(
            decryptionVerifiers[encryptionSchemeId] !=
                IDecryptionVerifier(address(0)),
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = IDecryptionVerifier(
            address(0)
        );
        success = true;
        emit EncryptionSchemeDisabled(encryptionSchemeId);
    }

    /// @inheritdoc IEnclave
    function setE3ProgramsParams(
        bytes[] memory _e3ProgramsParams
    ) public onlyOwner returns (bool success) {
        uint256 length = _e3ProgramsParams.length;
        for (uint256 i; i < length; ) {
            e3ProgramsParams[_e3ProgramsParams[i]] = true;
            unchecked {
                ++i;
            }
        }
        success = true;
        emit AllowedE3ProgramsParamsSet(_e3ProgramsParams);
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
