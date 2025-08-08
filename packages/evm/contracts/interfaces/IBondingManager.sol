// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IBondingManager
 * @notice Interface for the BondingManager contract
 */
interface IBondingManager {
    /// @notice Custom errors
    error ZeroUsdcAddress();
    error ZeroEnclAddress();
    error ZeroRegistryAddress();
    error ZeroAmount();
    error ZeroAmounts();
    error NotBonded();
    error AlreadyRequested();
    error DecommissionNotRequested();
    error DecommissionDelayNotPassed();
    error NotAuthorizedSlasher();
    error ZeroSlashAmount();
    error NodeNotBonded();
    error InsufficientBond();
    error ZeroPrice();
    error ZeroAddress();

    /// @notice Event emitted when a node bonds USDC collateral
    event NodeBondedUSDC(
        address indexed node,
        uint256 usdcAmount,
        uint256 totalUsdValue
    );

    /// @notice Event emitted when a node bonds ENCL collateral
    event NodeBondedENCL(
        address indexed node,
        uint256 enclAmount,
        uint256 totalUsdValue
    );

    /// @notice Event emitted when a node requests decommission
    event DecommissionRequested(address indexed node, uint256 requestTime);

    /// @notice Event emitted when a node is decommissioned and collateral returned
    event NodeDecommissioned(
        address indexed node,
        uint256 usdcReturned,
        uint256 enclReturned
    );

    /// @notice Event emitted when a node is slashed
    event NodeSlashed(
        address indexed node,
        uint256 usdcSlashed,
        uint256 enclSlashed,
        uint256 totalUsdSlashed,
        string reason
    );

    /// @notice Event emitted when the chainlink price feed fails
    event ChainlinkPriceFailed();

    /// @notice Event emitted when a slasher is added
    event SlasherAdded(address indexed slasher);

    /// @notice Event emitted when a slasher is removed
    event SlasherRemoved(address indexed slasher);

    /// @notice Event emitted when minimum bond requirement is updated
    event MinBondUpdated(uint256 newMinBondUsd);

    /// @notice Event emitted when registration delay is updated
    event RegistrationDelayUpdated(uint256 newDelay);

    /// @notice Event emitted when decommission delay is updated
    event DecommissionDelayUpdated(uint256 newDelay);

    /**
     * @notice Check if a node is properly bonded
     * @param node Address of the node to check
     * @return isBonded Whether the node meets bonding requirements
     */
    function isBonded(address node) external view returns (bool isBonded);

    /**
     * @notice Get the total USD value of a node's bond
     * @param node Address of the node
     * @return totalUsdValue Total USD value of the node's collateral
     */
    function getBondValue(
        address node
    ) external view returns (uint256 totalUsdValue);

    /**
     * @notice Get detailed bond information for a node
     * @param node Address of the node
     * @return usdcAmount Amount of USDC bonded
     * @return enclAmount Amount of ENCL bonded
     * @return totalUsdValue Total USD value of collateral
     * @return bondedAt Timestamp when node was bonded
     * @return canDecommission Whether node can currently decommission
     */
    function getBondInfo(
        address node
    )
        external
        view
        returns (
            uint256 usdcAmount,
            uint256 enclAmount,
            uint256 totalUsdValue,
            uint256 bondedAt,
            bool canDecommission
        );

    /**
     * @notice Bond USDC collateral to become a ciphernode
     * @param usdcAmount Amount of USDC to bond
     */
    function bondUSDC(uint256 usdcAmount) external;

    /**
     * @notice Bond ENCL tokens as collateral
     * @param enclAmount Amount of ENCL tokens to bond
     */
    function bondENCL(uint256 enclAmount) external;

    /**
     * @notice Bond ENCL tokens as collateral with permit
     * @param enclAmount Amount of ENCL tokens to bond
     * @param deadline Deadline for the permit
     * @param v ECDSA signature
     * @param r ECDSA signature
     * @param s ECDSA signature
     */
    function bondENCLWithPermit(
        uint256 enclAmount,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;

    /**
     * @notice Request decommission from being a ciphernode
     * @dev Starts the decommission delay period
     */
    function requestDecommission() external;

    /**
     * @notice Complete decommission and withdraw collateral
     * @dev Can only be called after decommission delay has passed
     */
    function completeDecommission(uint256[] calldata siblingNodes) external;

    /**
     * @notice Slash a node's collateral for misbehavior
     * @param node Address of the node to slash
     * @param usdAmount USD amount to slash
     * @param reason Reason for slashing
     */
    function slash(
        address node,
        uint256 usdAmount,
        string calldata reason,
        uint256[] calldata siblingNodes
    ) external;
}
