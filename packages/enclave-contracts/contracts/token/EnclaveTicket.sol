// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity ^0.8.27;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
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
 * @notice Non-transferable ERC20Votes wrapper over USDC for operator staking
 * @dev Features:
 *      - Only BondingRegistry can deposit/withdraw
 *      - Auto self-delegation on first mint for voting
 *      - Slashing burns shares and sends USDC to treasury
 *      - Non-transferable between users
 */
contract EnclaveTicketToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable,
    ERC20Wrapper
{
    address public registry;
    address public slashedFundsTreasury;

    error NotRegistry();
    error TransferNotAllowed();
    error ZeroAddress();
    error DelegationLocked();

    modifier onlyRegistry() {
        if (msg.sender != registry) revert NotRegistry();
        _;
    }

    constructor(
        IERC20 underlyingUSDC,
        address registry_,
        address treasury_,
        address initialOwner_
    )
        ERC20("Enclave Ticket Token", "ETK")
        ERC20Permit("Enclave Ticket Token")
        ERC20Wrapper(underlyingUSDC)
        Ownable(initialOwner_)
    {
        require(registry_ != address(0), ZeroAddress());
        require(treasury_ != address(0), ZeroAddress());
        registry = registry_;
        slashedFundsTreasury = treasury_;
    }

    function setRegistry(address newRegistry) external onlyOwner {
        require(newRegistry != address(0), ZeroAddress());
        registry = newRegistry;
    }

    function setTreasury(address newTreasury) external onlyOwner {
        require(newTreasury != address(0), ZeroAddress());
        slashedFundsTreasury = newTreasury;
    }

    /**
     * @notice Deposit USDC and mint ticket tokens to operator
     * @param operator Address to receive the ticket tokens
     * @param amount Amount of USDC to wrap
     * @return success True if successful
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
     * @notice Deposit USDC from an account and mint ticket tokens to an account
     * @param from Address to deposit from
     * @param to Address to mint to
     * @param amount Amount of USDC to deposit
     * @return success True if successful
     */
    function depositFrom(
        address from,
        address to,
        uint256 amount
    ) external onlyRegistry returns (bool) {
        IERC20(address(underlying())).transferFrom(from, address(this), amount);
        _mint(to, amount);
        if (delegates(to) == address(0)) _delegate(to, to);
        return true;
    }

    /**
     * @notice Burn ticket tokens and transfer USDC to receiver
     * @dev Registry must have approval or use permit before calling
     * @param receiver Address to receive the USDC
     * @param amount Amount of ticket tokens to burn
     * @return success True if successful
     */
    function withdrawTo(
        address receiver,
        uint256 amount
    ) public override onlyRegistry returns (bool success) {
        return super.withdrawTo(receiver, amount);
    }

    /**
     * @notice Lock ticket tokens for exit
     * @param operator Address to lock from
     * @param amount Amount of ticket tokens to lock
     */
    function lockForExit(
        address operator,
        uint256 amount
    ) external onlyRegistry {
        _burn(operator, amount);
    }

    /**
     * @notice Payout ticket tokens to an address
     * @param to Address to payout to
     * @param amount Amount of ticket tokens to payout
     */
    function payout(address to, uint256 amount) external onlyRegistry {
        IERC20(address(underlying())).transfer(to, amount);
    }

    /**
     * @notice Slash ticket tokens by burning shares and transferring USDC to treasury
     * @param operator Operator to slash
     * @param amount Amount to slash
     */
    function slash(address operator, uint256 amount) external onlyRegistry {
        _burn(operator, amount);
    }

    /**
     * @notice Prevent transfers between users (only mint/burn allowed)
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
     * @notice Delegate voting power to an address.
     * @dev This function is locked and cannot be used.
     */

    function delegate(address) public pure override {
        revert DelegationLocked();
    }

    /**
     * @notice Delegate voting power to an address using a signature.
     * @dev This function is locked and cannot be used.
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

    function decimals()
        public
        view
        override(ERC20, ERC20Wrapper)
        returns (uint8)
    {
        return super.decimals();
    }

    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }
}
