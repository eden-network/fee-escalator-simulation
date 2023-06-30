// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


struct BorrowParams {
    address borrower;
    address borrowToken;
    uint borrowAmount;
}

struct ExecutionParams {
    address targetToken;
    address target;
    bytes data;
}

interface IBalDiffReverter {

    function approve(address token, address spender, uint amount) external;
    
    function borrowAndRevert(
        BorrowParams memory, 
        ExecutionParams memory
    ) external returns (int balDiff);
    
}