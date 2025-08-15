// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract VestingEscrow is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // Custom errors
    error ZeroTokenAddress();
    error ZeroAddress();
    error ZeroAmount();
    error ZeroVestingDuration();
    error CliffExceedsVesting();
    error StreamAlreadyExists();
    error NoVestingStream();
    error StreamRevoked();
    error NoTokensToClaim();
    error CanOnlyClaimForYourself();
    error AlreadyRevoked();
    error ArrayLengthMismatch();
    error InsufficientAllowance();
    error InsufficientBalance();
    error WouldBreakVestingInvariant();
    error CannotSweepVestingToken();

    struct VestingStream {
        uint256 totalAmount;
        uint256 startTime;
        uint256 cliffDuration;
        uint256 vestingDuration;
        uint256 claimed;
        bool revoked;
    }

    /// @notice The ENCL token contract
    IERC20 public immutable ENCL_TOKEN;

    /// @notice Mapping of beneficiary to their vesting stream
    mapping(address beneficiary => VestingStream stream) public vestingStreams;

    /// @notice Total amount of tokens held in escrow
    uint256 public totalEscrowed;

    /// @notice Total amount of tokens claimed from escrow
    uint256 public totalClaimed;

    /// @notice Event emitted when a vesting stream is created
    event VestingStreamCreated(
        address indexed beneficiary,
        uint256 totalAmount,
        uint256 startTime,
        uint256 cliffDuration,
        uint256 vestingDuration
    );

    /// @notice Event emitted when tokens are claimed
    event TokensClaimed(address indexed beneficiary, uint256 amount);

    /// @notice Event emitted when a vesting stream is revoked
    event VestingStreamRevoked(
        address indexed beneficiary,
        uint256 unvestedAmount
    );

    /**
     * @notice Initialize the vesting escrow
     * @param _token Address of the ENCL token contract
     * @param _owner Initial owner of the contract
     */
    constructor(address _token, address _owner) Ownable(_owner) {
        require(_token != address(0), ZeroTokenAddress());
        ENCL_TOKEN = IERC20(_token);
    }

    /**
     * @notice Create a vesting stream for a beneficiary
     * @param beneficiary Address of the beneficiary
     * @param totalAmount Total amount of tokens to vest
     * @param startTime Timestamp when vesting starts
     * @param cliffDuration Duration of cliff period in seconds
     * @param vestingDuration Total vesting duration in seconds
     */
    function createVestingStream(
        address beneficiary,
        uint256 totalAmount,
        uint256 startTime,
        uint256 cliffDuration,
        uint256 vestingDuration
    ) external onlyOwner {
        require(beneficiary != address(0), ZeroAddress());
        require(totalAmount > 0, ZeroAmount());
        require(vestingDuration > 0, ZeroVestingDuration());
        require(cliffDuration <= vestingDuration, CliffExceedsVesting());
        require(
            vestingStreams[beneficiary].totalAmount == 0,
            StreamAlreadyExists()
        );

        // Transfer tokens to escrow
        ENCL_TOKEN.safeTransferFrom(msg.sender, address(this), totalAmount);

        // Create vesting stream
        vestingStreams[beneficiary] = VestingStream({
            totalAmount: totalAmount,
            startTime: startTime,
            cliffDuration: cliffDuration,
            vestingDuration: vestingDuration,
            claimed: 0,
            revoked: false
        });

        totalEscrowed += totalAmount;

        emit VestingStreamCreated(
            beneficiary,
            totalAmount,
            startTime,
            cliffDuration,
            vestingDuration
        );
    }

    /**
     * @notice Batch create vesting streams for multiple beneficiaries
     * @dev Caller must pre-approve the total amount before calling
     * @param beneficiaries Array of beneficiary addresses
     * @param totalAmounts Array of total amounts for each beneficiary
     * @param startTimes Array of start times for each beneficiary
     * @param cliffDurations Array of cliff durations for each beneficiary
     * @param vestingDurations Array of vesting durations for each beneficiary
     */
    function batchCreateVestingStreams(
        address[] memory beneficiaries,
        uint256[] memory totalAmounts,
        uint256[] memory startTimes,
        uint256[] memory cliffDurations,
        uint256[] memory vestingDurations
    ) external onlyOwner {
        require(
            beneficiaries.length == totalAmounts.length &&
                totalAmounts.length == startTimes.length &&
                startTimes.length == cliffDurations.length &&
                cliffDurations.length == vestingDurations.length,
            ArrayLengthMismatch()
        );

        uint256 totalTransfer = 0;
        for (uint256 i = 0; i < beneficiaries.length; i++) {
            totalTransfer += totalAmounts[i];
        }

        // Check allowance before attempting transfer
        require(
            ENCL_TOKEN.allowance(msg.sender, address(this)) >= totalTransfer,
            InsufficientAllowance()
        );

        // Transfer all tokens in one transaction
        ENCL_TOKEN.safeTransferFrom(msg.sender, address(this), totalTransfer);

        // Create all streams
        for (uint256 i = 0; i < beneficiaries.length; i++) {
            require(beneficiaries[i] != address(0), ZeroAddress());
            require(totalAmounts[i] > 0, ZeroAmount());
            require(vestingDurations[i] > 0, ZeroVestingDuration());
            require(
                cliffDurations[i] <= vestingDurations[i],
                CliffExceedsVesting()
            );
            require(
                vestingStreams[beneficiaries[i]].totalAmount == 0,
                StreamAlreadyExists()
            );

            vestingStreams[beneficiaries[i]] = VestingStream({
                totalAmount: totalAmounts[i],
                startTime: startTimes[i],
                cliffDuration: cliffDurations[i],
                vestingDuration: vestingDurations[i],
                claimed: 0,
                revoked: false
            });

            totalEscrowed += totalAmounts[i];

            emit VestingStreamCreated(
                beneficiaries[i],
                totalAmounts[i],
                startTimes[i],
                cliffDurations[i],
                vestingDurations[i]
            );
        }
    }

    /**
     * @notice Claim vested tokens for yourself
     */
    function claim() external nonReentrant {
        _claimVested(msg.sender);
    }

    /**
     * @notice Claim vested tokens for another address (owner only)
     * @param beneficiary Address of the beneficiary to claim for
     */
    function claimFor(address beneficiary) external onlyOwner nonReentrant {
        _claimVested(beneficiary);
    }

    /**
     * @notice Internal claim logic
     * @param beneficiary Address of the beneficiary to claim for
     */
    function _claimVested(address beneficiary) internal {
        VestingStream storage stream = vestingStreams[beneficiary];
        require(stream.totalAmount > 0, NoVestingStream());
        require(!stream.revoked, StreamRevoked());

        uint256 claimable = getClaimableAmount(beneficiary);
        require(claimable > 0, NoTokensToClaim());

        stream.claimed += claimable;
        totalClaimed += claimable;

        ENCL_TOKEN.safeTransfer(beneficiary, claimable);

        emit TokensClaimed(beneficiary, claimable);
    }

    /**
     * @notice Legacy function for backward compatibility - claims for msg.sender
     * @param beneficiary Must be msg.sender
     */
    function claimVested(address beneficiary) external nonReentrant {
        require(beneficiary == msg.sender, CanOnlyClaimForYourself());
        _claimVested(beneficiary);
    }

    /**
     * @notice Revoke a vesting stream and return unvested tokens
     * @param beneficiary Address of the beneficiary whose stream to revoke
     */
    function revokeVestingStream(address beneficiary) external onlyOwner {
        VestingStream storage stream = vestingStreams[beneficiary];
        require(stream.totalAmount > 0, NoVestingStream());
        require(!stream.revoked, AlreadyRevoked());

        uint256 claimable = getClaimableAmount(beneficiary);
        uint256 unvested = stream.totalAmount - stream.claimed - claimable;

        stream.revoked = true;

        // Transfer any claimable tokens to beneficiary
        if (claimable > 0) {
            stream.claimed += claimable;
            totalClaimed += claimable;
            ENCL_TOKEN.safeTransfer(beneficiary, claimable);
            emit TokensClaimed(beneficiary, claimable);
        }

        // Return unvested tokens to owner
        if (unvested > 0) {
            totalEscrowed -= unvested;
            ENCL_TOKEN.safeTransfer(owner(), unvested);
        }

        emit VestingStreamRevoked(beneficiary, unvested);
    }

    /**
     * @notice Get the amount of tokens that can be claimed by a beneficiary
     * @param beneficiary Address of the beneficiary
     * @return claimable Amount of tokens that can be claimed
     */
    function getClaimableAmount(
        address beneficiary
    ) public view returns (uint256 claimable) {
        VestingStream memory stream = vestingStreams[beneficiary];

        if (stream.totalAmount == 0 || stream.revoked) {
            return 0;
        }

        uint256 vested = getVestedAmount(beneficiary);
        claimable = vested - stream.claimed;
    }

    /**
     * @notice Get the total amount of tokens vested for a beneficiary
     * @param beneficiary Address of the beneficiary
     * @return vested Total amount of tokens vested
     */
    function getVestedAmount(
        address beneficiary
    ) public view returns (uint256 vested) {
        VestingStream memory stream = vestingStreams[beneficiary];

        if (stream.totalAmount == 0) {
            return 0;
        }

        uint256 currentTime = block.timestamp;

        // If before cliff, nothing is vested
        if (currentTime < stream.startTime + stream.cliffDuration) {
            return 0;
        }

        // If fully vested
        if (currentTime >= stream.startTime + stream.vestingDuration) {
            return stream.totalAmount;
        }

        // Linear vesting calculation
        uint256 timeElapsed = currentTime - stream.startTime;
        vested = (stream.totalAmount * timeElapsed) / stream.vestingDuration;
    }

    /**
     * @notice Get remaining vesting time for a beneficiary
     * @param beneficiary Address of the beneficiary
     * @return remainingTime Time remaining until fully vested (0 if fully vested)
     */
    function getRemainingVestingTime(
        address beneficiary
    ) external view returns (uint256 remainingTime) {
        VestingStream memory stream = vestingStreams[beneficiary];

        if (stream.totalAmount == 0) {
            return 0;
        }

        uint256 endTime = stream.startTime + stream.vestingDuration;
        uint256 currentTime = block.timestamp;

        if (currentTime >= endTime) {
            return 0;
        }

        remainingTime = endTime - currentTime;
    }

    /**
     * @notice Emergency withdrawal function for owner
     * @dev Only for excess tokens not part of vesting streams
     * @param amount Amount of tokens to withdraw
     */
    function emergencyWithdraw(uint256 amount) external onlyOwner {
        uint256 contractBalance = ENCL_TOKEN.balanceOf(address(this));
        require(amount <= contractBalance, InsufficientBalance());

        uint256 reservedTokens = totalEscrowed - totalClaimed;
        require(
            contractBalance - amount >= reservedTokens,
            WouldBreakVestingInvariant()
        );

        ENCL_TOKEN.safeTransfer(owner(), amount);
    }

    /**
     * @notice Sweep tokens
     * @param tokenAddress Address of token to sweep
     * @param amount Amount to sweep
     */
    function sweepToken(
        address tokenAddress,
        uint256 amount
    ) external onlyOwner {
        require(tokenAddress != address(ENCL_TOKEN), CannotSweepVestingToken());
        IERC20(tokenAddress).safeTransfer(owner(), amount);
    }
}
