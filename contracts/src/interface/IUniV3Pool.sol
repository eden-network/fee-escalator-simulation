// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


interface IUniV3Pool {
    function token0() external view returns (address);
    function token1() external view returns (address);
    function flash(
        address recipient, 
        uint amount0, 
        uint amount1, 
        bytes calldata data
    ) external;
}