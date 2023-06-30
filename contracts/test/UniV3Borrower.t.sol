// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";

import "../src/interface/IERC20.sol";
import "../src/UniV3Borrower.sol";
import { decodeStrategicErrInt, StrategicErrorInt } from "../src/StrategicError.sol";


contract Dummy {
    function return9923() external pure returns (int) {
        revert StrategicErrorInt(9923);
    }   
}

contract UniV3BorrowerTest is Test {
    UniV3Borrower public borrower;
    Dummy public dummy;

    address constant WethUsdcPoolArb = 0xC31E54c7a869B9FcBEcc14363CF510d1c41fa443;
    address constant WethWbtcPoolArb = 0x2f5e87C9312fa29aed5c179E456625D79015299c;
    address constant WethArb = 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1;
    address constant UsdcArb = 0xaf88d065e77c8cC2239327C5EDb3A432268e5831;

    function setUp() public {
        address[] memory pools = new address[](2);
        pools[0] = WethUsdcPoolArb;
        pools[1] = WethWbtcPoolArb;
        borrower = new UniV3Borrower(pools);
        dummy = new Dummy();
    }

    function test_getMostLiquidPool() public {
        (address mostLiquid, uint amount) = borrower.getMostLiquidPool(WethArb);

        uint wethusdcBal = IERC20(WethArb).balanceOf(WethUsdcPoolArb);
        uint wethwbtcBal = IERC20(WethArb).balanceOf(WethWbtcPoolArb);
        if (wethusdcBal > wethwbtcBal) {
            assertEq(mostLiquid, WethUsdcPoolArb);
            assertEq(amount, wethusdcBal);
        } else {
            assertEq(mostLiquid, WethWbtcPoolArb);
            assertEq(amount, wethwbtcBal);
        }
    }

    function test_borrowAndCall() public {
        address borrowToken = WethArb;
        uint borrowAmount = 1 ether;
        address receiver = address(dummy);
        bytes memory data = abi.encodeWithSignature("return9923()");
        bytes memory res = borrower.borrowAndCall(borrowToken, borrowAmount, receiver, data);
        int result = decodeStrategicErrInt(res);
        assertEq(result, 9923);
    }

}
