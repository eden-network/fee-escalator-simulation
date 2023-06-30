// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


abstract contract Borrower {
    
    function borrowAndCall(
        address borrowToken,
        uint borrowAmount,
        address receiver, 
        bytes memory data
    ) public virtual returns (bytes memory response);

}