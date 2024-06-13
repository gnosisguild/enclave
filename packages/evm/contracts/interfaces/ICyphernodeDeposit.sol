// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

/*
 * NOTE: the comments are for now targetted at facilitating the design discussion.
 * We'll later convert to natspec
 */

/*
 * ------
 * This contract manages the custody and collateral of Cyphernodes.
 * Cyphernodes must stake a bond to be eligible for commission selection.
 * A Cyphernode can leave at any time, provided it is not currently fulfilling any duties.
 */

/*
 * ------
 * Although not yet integrated into the Registry flow, when receiving the committee
 * selection message from a Filter, the registry is supposed to validate that enough
 * available bond remains for the candidate node. Filters should also be aware of the
 * bond state of a node, as it impacts the sortition function. Perhaps this can be
 * made transparently available by having the Registry include it in the
 * isCyphernodeAvailable result.
 */

interface ICyphernodeDeposit {
    /*
     *
     * Self-service
     *
     */

    // Called by the Cyphernode whenever staking
    function stake(uint256 bond) external;
    // Called by the Cyphernode whenever leaving. Must have no active duties
    function unstake(address node) external;

    /*
     *
     * Called by the protocol
     *
     */

    // This function ties a portion of the node's bond to a specific E3.
    // To be called exclusively by the Registry, when a committee selection message received
    function joinE3(uint256 e3Id, address node, uint256 amount) external;
    // Releases a node after E3 cancellation
    function abortE3(uint256 e3Id, address node) external;
    // Conclude a completed E3 process associated with a node, releasing the portion of the bond allocated to it.
    // Note: There is currently no final idea on who should be responsible for calling this function. Perhaps the Deposit should be granted access to the Enclave, and it could query that base layer in order to validate E3 is concluded.
    function concludeE3(uint256 e3Id, address node) external;
    // Slash a node's bond as a penalty for misconduct or failure to act in a timely manner.
    // Note: same as conclude, with the added difficulty of needed to prove failure to act, or improper conduct
    function slashE3(
        uint256 e3Id,
        address node,
        bytes32 reasonOrProof
    ) external;
    // Retrieve a list of E3s that a node is bound to
    function duties(address node) external view returns (uint256[] e3Ids);

    // Retrieve bond-related information for a node, including total (all the bond) and available (bonds not allocated to an E3)
    function bond(
        address node
    ) external view returns (uint256 totalBond, uint256 availableBond);
}
