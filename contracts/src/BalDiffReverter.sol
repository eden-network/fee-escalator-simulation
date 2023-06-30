// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {
    decodeStrategicErrInt,
    StrategicErrorInt
} from "../src/StrategicError.sol";
import "./interface/IBalDiffReverter.sol";
import "./interface/IERC20.sol";
import "./Borrower.sol";


contract BalDiffReverter is IBalDiffReverter {

    address immutable owner = msg.sender;

    function approve(
        address token, 
        address spender, 
        uint amount
    ) external {
        require(msg.sender == owner, "only owner");
        IERC20(token).approve(spender, amount);
    }

    function borrowAndRevert(
        BorrowParams memory borrowParams,
        ExecutionParams memory exeParams
    ) external returns (int balDiff) {
        bytes memory exeCalldata = bytes(abi.encodeWithSignature(
            "revertWithBalDiff((address,address,bytes))", 
            exeParams
        ));
        bytes memory borrowRes = Borrower(borrowParams.borrower).borrowAndCall(
            borrowParams.borrowToken, 
            borrowParams.borrowAmount,
            address(this),
            exeCalldata
        );
        balDiff = decodeStrategicErrInt(borrowRes);
    }

    function revertWithBalDiff(ExecutionParams calldata exe) external {
        int bal0 = int(IERC20(exe.targetToken).balanceOf(address(this)));
        (bool s, bytes memory r) = exe.target.call(exe.data);
        require(s, string(r));
        int bal1 = int(IERC20(exe.targetToken).balanceOf(address(this)));
        revert StrategicErrorInt(bal1 - bal0);
    }

}