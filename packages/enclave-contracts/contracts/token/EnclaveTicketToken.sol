// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity ^0.8.27;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {
    ERC20Wrapper
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Wrapper.sol";
import {
    ERC20Permit
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {
    ERC20Votes
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Votes.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { Nonces } from "@openzeppelin/contracts/utils/Nonces.sol";

/**
 * @title EnclaveTicketToken (ETK)
 * @notice Non-transferable, non-delegatable ERC20Votes wrapper over a stablecoin for operator staking
 * @dev ERC20 wrapper token that represents staked stablecoins (e.g., USDC, DAI) used for operator
 *      bonding in the Enclave protocol. Implements voting power tracking through ERC20Votes but
 *      prevents transfers between users and manual delegation. Deposits automatically delegate to
 *      self to enable voting power tracking. Only the designated registry contract can mint
 *      (deposit) and burn (withdraw) tokens.
 */
contract EnclaveTicketToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable,
    ERC20Wrapper
{
    using SafeERC20 for IERC20;
    // Custom errors

    /// @notice Thrown when a function is called by an address other than the registry
    error NotRegistry();

    /// @notice Thrown when attempting to transfer tokens between non-zero addresses
    error TransferNotAllowed();

    /// @notice Thrown when a zero address is provided where a valid address is required
    error ZeroAddress();

    /// @notice Thrown when attempting to manually delegate voting power
    error DelegationLocked();

    /// @notice Address of the registry contract authorized to mint, burn, and manage ticket tokens
    /// @dev Only this contract can call restricted functions like depositFor, withdrawTo, burnTickets, and payout
    address public registry;

    /// @notice Restricts function access to only the registry contract
    /// @dev Reverts with NotRegistry if caller is not the registry address
    modifier onlyRegistry() {
        if (msg.sender != registry) revert NotRegistry();
        _;
    }

    /**
     * @notice Initializes the Enclave Ticket Token with name "Enclave Ticket Token" and symbol "ETK"
     * @dev Sets up the token as an ERC20 wrapper around the provided base token (stablecoin).
     *      Initializes voting and permit functionality. The decimals will match the base token.
     * @param baseToken The underlying ERC20 stablecoin to wrap (e.g., USDC, DAI)
     * @param registry_ Address of the registry contract that will manage deposits and withdrawals
     * @param initialOwner_ Address that will own the contract and can update the registry
     */
    constructor(
        IERC20 baseToken,
        address registry_,
        address initialOwner_
    )
        ERC20("Enclave Ticket Token", "ETK")
        ERC20Permit("Enclave Ticket Token")
        ERC20Wrapper(baseToken)
        Ownable(initialOwner_)
    {
        setRegistry(registry_);
    }

    /**
     * @notice Updates the registry contract address
     * @dev Only callable by the contract owner. The new registry address cannot be zero.
     *      This function grants the new registry exclusive rights to mint, burn, and manage tokens.
     * @param newRegistry Address of the new registry contract (must not be zero address)
     */
    function setRegistry(address newRegistry) public onlyOwner {
        require(newRegistry != address(0), ZeroAddress());
        registry = newRegistry;
    }

    /**
     * @notice Deposits underlying tokens from the registry and mints ticket tokens to an operator
     * @dev Only callable by the registry contract. Transfers underlying tokens from the registry to
     *      this contract and mints an equivalent amount of ticket tokens. Automatically delegates
     *      voting power to the operator on their first deposit to enable voting power tracking.
     * @param operator Address to receive the minted ticket tokens
     * @param amount Number of underlying tokens to deposit and ticket tokens to mint
     * @return success True if the deposit and minting succeeded
     */
    function depositFor(
        address operator,
        uint256 amount
    ) public override onlyRegistry returns (bool success) {
        success = super.depositFor(operator, amount);

        // Auto-delegate on first deposit to track voting power
        if (delegates(operator) == address(0)) {
            _delegate(operator, operator);
        }
    }

    /**
     * @notice Deposits underlying tokens from a specified account and mints ticket tokens to another account
     * @dev Only callable by the registry contract. Transfers underlying tokens from the 'from' address
     *      to this contract and mints ticket tokens to the 'to' address. Useful for scenarios where
     *      the source and destination differ. Automatically delegates voting power to recipient on
     *      their first deposit.
     * @param from Address to transfer underlying tokens from (must have approved this contract)
     * @param to Address to receive the minted ticket tokens
     * @param amount Number of underlying tokens to deposit and ticket tokens to mint
     * @return bool True if the deposit and minting succeeded
     */
    function depositFrom(
        address from,
        address to,
        uint256 amount
    ) external onlyRegistry returns (bool) {
        SafeERC20.safeTransferFrom(
            IERC20(address(underlying())),
            from,
            address(this),
            amount
        );
        _mint(to, amount);
        if (delegates(to) == address(0)) _delegate(to, to);
        return true;
    }

    /**
     * @notice Burns ticket tokens from the registry and transfers underlying tokens to a receiver
     * @dev Only callable by the registry contract. Burns ticket tokens from the registry's balance
     *      and transfers an equivalent amount of underlying tokens to the receiver address. Used
     *      when operators unstake their tokens.
     * @param receiver Address to receive the underlying tokens
     * @param amount Number of ticket tokens to burn and underlying tokens to transfer
     * @return success True if the burn and transfer succeeded
     */
    function withdrawTo(
        address receiver,
        uint256 amount
    ) public override onlyRegistry returns (bool success) {
        return super.withdrawTo(receiver, amount);
    }

    /**
     * @notice Burns ticket tokens from an operator's balance without transferring underlying tokens
     * @dev Only callable by the registry contract. Used for slashing or penalizing operators where
     *      the underlying tokens should remain in the contract or be handled separately. Does not
     *      return underlying tokens to the operator.
     * @param operator Address whose ticket tokens will be burned
     * @param amount Number of ticket tokens to burn from the operator's balance
     */
    function burnTickets(
        address operator,
        uint256 amount
    ) external onlyRegistry {
        _burn(operator, amount);
    }

    /**
     * @notice Transfer underlying tokens to an address without burning ticket tokens.
     * @dev Only callable by the registry contract.
     * @param to Address to payout to.
     * @param amount Amount of ticket tokens to payout.
     */
    function payout(address to, uint256 amount) external onlyRegistry {
        SafeERC20.safeTransfer(IERC20(address(underlying())), to, amount);
    }

    /**
     * @dev Override ERC20Votes update hook to prevent transfers between users.
     */
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override(ERC20, ERC20Votes) {
        if (from != address(0) && to != address(0)) {
            revert TransferNotAllowed();
        }
        super._update(from, to, value);
    }

    /**
     * @dev Prevent delegation of voting power.
     */
    function delegate(address) public pure override {
        revert DelegationLocked();
    }

    /**
     * @dev Prevent delegation of voting power via signature.
     */
    function delegateBySig(
        address,
        uint256,
        uint256,
        uint8,
        bytes32,
        bytes32
    ) public pure override {
        revert DelegationLocked();
    }

    /**
     * @dev Expose decimals from the underlying token.
     */
    function decimals()
        public
        view
        override(ERC20, ERC20Wrapper)
        returns (uint8)
    {
        return super.decimals();
    }

    /**
     * @dev Expose permit nonces via both ERC20Permit and OpenZeppelin Nonces.
     */
    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }
}
