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
 * @notice Non-transferable non-delegatable ERC20Votes wrapper over a Stable token (USDC, DAI etc.) for operator staking
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
    error NotRegistry();
    error TransferNotAllowed();
    error ZeroAddress();
    error DelegationLocked();

    /// @dev Address of the registry contract that manages deposits and withdrawals.
    address public registry;

    modifier onlyRegistry() {
        if (msg.sender != registry) revert NotRegistry();
        _;
    }

    /**
     * @notice Deploy the Enclave Ticket Token.
     * @param baseToken The underlying ERC20 token to wrap (e.g., USDC, DAI).
     * @param registry_ The address of the registry contract.
     * @param initialOwner_ The address that will own the contract.
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
     * @notice Set a new registry contract address.
     * @dev Only callable by the contract owner.
     * @param newRegistry The address of the new registry contract.
     */
    function setRegistry(address newRegistry) public onlyOwner {
        require(newRegistry != address(0), ZeroAddress());
        registry = newRegistry;
    }

    /**
     * @notice Deposit Base token and mint ticket tokens to operator.
     * @dev Only callable by the registry contract. Auto-delegates on first deposit.
     * @param operator Address to receive the ticket tokens.
     * @param amount Amount of tokens to deposit.
     * @return success True if successful.
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
     * @notice Deposit Base token from an account and mint ticket tokens to an account.
     * @dev Only callable by the registry contract. Auto-delegates on first deposit.
     * @param from Address to deposit from.
     * @param to Address to mint to.
     * @param amount Amount of tokens to deposit.
     * @return True if successful.
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
     * @notice Burn ticket tokens and transfer Base token to receiver.
     * @dev Only callable by the registry contract.
     * @param receiver Address to receive the Underlying token.
     * @param amount Amount of ticket tokens to burn.
     * @return success True if successful.
     */
    function withdrawTo(
        address receiver,
        uint256 amount
    ) public override onlyRegistry returns (bool success) {
        return super.withdrawTo(receiver, amount);
    }

    /**
     * @notice Burn ticket tokens from an operator.
     * @dev Only callable by the registry contract.
     * @param operator Address to burn from.
     * @param amount Amount of ticket tokens to burn.
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
