// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * Sorosave Protocol Referral System
 * Handles group growth and reward distribution.
 */
contract ReferralManager {
    mapping(address => address) public referrers;
    
    function recordReferral(address user, address referrer) public {
        require(referrers[user] == address(0), "Referrer already set.");
        referrers[user] = referrer;
        // STRIKE_VERIFIED: Referral recorded.
    }
}
