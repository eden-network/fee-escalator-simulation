// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "solmate/tokens/ERC20.sol";

import "../src/interface/IERC20.sol";
import "../src/UniV3Borrower.sol";
import "../src/BalDiffReverter.sol";
import { decodeStrategicErrInt, StrategicErrorInt } from "../src/StrategicError.sol";


contract MintableERC20 is ERC20 {

    constructor(string memory symbol) ERC20("MintableERC20", symbol, 18) {}

    function mint(address to, uint amount) public {
        _mint(to, amount);
    }

    function oneToOne(address exchangeToken, uint mintAmount) external {
        IERC20(exchangeToken).transferFrom(msg.sender, address(this), mintAmount);
        mint(msg.sender, mintAmount);

    }

}

contract BalDiffReverterTest is Test {
    UniV3Borrower public borrower;
    BalDiffReverter public reverter;
    MintableERC20 public mintable;

    address constant WethUsdcPoolArb = 0xC31E54c7a869B9FcBEcc14363CF510d1c41fa443;
    address constant WethArb = 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1;
    address constant UsdcArb = 0xaf88d065e77c8cC2239327C5EDb3A432268e5831;

    function setUp() public {
        address[] memory pools = new address[](1);
        pools[0] = WethUsdcPoolArb;
        borrower = new UniV3Borrower(pools);
        reverter = new BalDiffReverter();
        mintable = new MintableERC20("MNT");
    }

    function test_revertWithBalDiff() public {
        int mintAmount = 2.3 ether;
        try reverter.revertWithBalDiff(ExecutionParams({
            targetToken: address(mintable),
            target: address(mintable),
            data: abi.encodeWithSignature(
                "mint(address,uint256)", 
                address(reverter), 
                uint(mintAmount)
            )
        })) {
            revert("should have reverted");
        } catch (bytes memory errMsg) {
            assertEq(decodeStrategicErrInt(errMsg), mintAmount);
        }
    }

    function test_borrowAndRevert() public {
        address borrowToken = WethArb;
        uint mintAmount = 90 ether;
        uint borrowAmount = mintAmount;

        reverter.approve(borrowToken, address(mintable), mintAmount);

        int balDiff = reverter.borrowAndRevert(
            BorrowParams({
                borrower: address(borrower),
                borrowToken: WethArb,
                borrowAmount: borrowAmount
            }), 
            ExecutionParams({
                targetToken: address(mintable),
                target: address(mintable),
                data: abi.encodeWithSignature(
                    "oneToOne(address,uint256)", 
                    borrowToken,
                    mintAmount
                )
            })
        );

        assertEq(balDiff, int(mintAmount));
    }

}
