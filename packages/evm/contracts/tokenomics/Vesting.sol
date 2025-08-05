// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract VestingEscrow is Ownable, ReentrancyGuard {
    struct VestingStream {
        uint256 totalAmount;
        uint256 startTime;
        uint256 cliffDuration;
        uint256 vestingDuration;
        uint256 claimed;
        bool revoked;
    }

    IERC20 public immutable token;
    mapping(address => VestingStream) public vestingStreams;
    uint256 public totalEscrowed;
    uint256 public totalClaimed;

    constructor(address _token, address _owner) Ownable(_owner) {
        require(_token != address(0), "Zero token address");
        token = IERC20(_token);
    }

    function createVestingStream(
        address beneficiary,
        uint256 totalAmount,
        uint256 startTime,
        uint256 cliffDuration,
        uint256 vestingDuration
    ) external onlyOwner {
        require(beneficiary != address(0), "Zero address");
        require(totalAmount > 0, "Zero amount");
        require(vestingDuration > 0, "Zero vesting");
        require(cliffDuration <= vestingDuration, "Cliff too long");
        require(vestingStreams[beneficiary].totalAmount == 0, "Stream exists");

        token.transferFrom(msg.sender, address(this), totalAmount);

        vestingStreams[beneficiary] = VestingStream({
            totalAmount: totalAmount,
            startTime: startTime,
            cliffDuration: cliffDuration,
            vestingDuration: vestingDuration,
            claimed: 0,
            revoked: false
        });

        totalEscrowed += totalAmount;
    }

    function claim() external nonReentrant {
        _claimVested(msg.sender);
    }

    function claimFor(address beneficiary) external onlyOwner nonReentrant {
        _claimVested(beneficiary);
    }

    function _claimVested(address beneficiary) internal {
        VestingStream storage stream = vestingStreams[beneficiary];
        require(stream.totalAmount > 0, "No stream");
        require(!stream.revoked, "Revoked");

        uint256 claimable = getClaimableAmount(beneficiary);
        require(claimable > 0, "Nothing to claim");

        stream.claimed += claimable;
        totalClaimed += claimable;

        token.transfer(beneficiary, claimable);
    }

    function getClaimableAmount(
        address beneficiary
    ) public view returns (uint256) {
        VestingStream memory stream = vestingStreams[beneficiary];
        if (stream.totalAmount == 0 || stream.revoked) return 0;

        uint256 vested = getVestedAmount(beneficiary);
        return vested - stream.claimed;
    }

    function getVestedAmount(
        address beneficiary
    ) public view returns (uint256) {
        VestingStream memory stream = vestingStreams[beneficiary];
        if (stream.totalAmount == 0) return 0;

        uint256 currentTime = block.timestamp;

        if (currentTime < stream.startTime + stream.cliffDuration) return 0;
        if (currentTime >= stream.startTime + stream.vestingDuration)
            return stream.totalAmount;

        uint256 timeElapsed = currentTime - stream.startTime;
        return (stream.totalAmount * timeElapsed) / stream.vestingDuration;
    }

    function revokeVestingStream(address beneficiary) external onlyOwner {
        VestingStream storage stream = vestingStreams[beneficiary];
        require(stream.totalAmount > 0, "No stream");
        require(!stream.revoked, "Already revoked");

        uint256 claimable = getClaimableAmount(beneficiary);
        uint256 unvested = stream.totalAmount - stream.claimed - claimable;

        stream.revoked = true;

        if (claimable > 0) {
            stream.claimed += claimable;
            totalClaimed += claimable;
            token.transfer(beneficiary, claimable);
        }

        if (unvested > 0) {
            totalEscrowed -= unvested;
            token.transfer(owner(), unvested);
        }
    }

    function getRemainingVestingTime(
        address beneficiary
    ) external view returns (uint256) {
        VestingStream memory stream = vestingStreams[beneficiary];
        if (stream.totalAmount == 0) return 0;

        uint256 endTime = stream.startTime + stream.vestingDuration;
        uint256 currentTime = block.timestamp;

        return currentTime >= endTime ? 0 : endTime - currentTime;
    }

    function emergencyWithdraw(uint256 amount) external onlyOwner {
        uint256 contractBalance = token.balanceOf(address(this));
        require(amount <= contractBalance, "Too much");

        uint256 reserved = totalEscrowed - totalClaimed;
        require(contractBalance - amount >= reserved, "Would break vesting");

        token.transfer(owner(), amount);
    }

    function sweepToken(
        address tokenAddress,
        uint256 amount
    ) external onlyOwner {
        require(tokenAddress != address(token), "Can't sweep vesting token");
        IERC20(tokenAddress).transfer(owner(), amount);
    }
}
