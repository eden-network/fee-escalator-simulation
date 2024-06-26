// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


interface IERC20 {
    function balanceOf(address) external view returns (uint);
    function transferFrom(address, address, uint) external returns (bool);
    function approve(address,uint) external;
}