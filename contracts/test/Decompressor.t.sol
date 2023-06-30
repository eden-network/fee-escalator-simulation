// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/external/OneInchDecompressor.sol";


contract Decompressor is DecompressorExtension {}


contract DecompressorTest is Test {

    address public decompressor;

    function setUp() public {
        decompressor = address(new Decompressor());
    }

    function test_decompress() public {
        bytes memory compressedData = hex"0000000000000000000000000000000000000000000000000000000000f000a007e5c0d20000000000000000000000000000000000000000000000cc00006900001a404182af49447d8a07e3bd95bd0d56f35241523fbab1d0e30db002a0000000000000000000000000000000000000000000000000000000000356a911ee63c1e50148d7e1a9d652ba5f5d80a8dc396df37993659f3582af49447d8a07e3bd95bd0d56f35241523fbab102a0000000000000000000000000000000000000000000000002c9717ab9f634bf4aee63c1e580e4d9faddd9bca5d8393bee915dc56e916ab94d27ff970a61a04b1ca14834a43f5de4533ebddb5cc81111111254eeb25477b68fb85ed929f73a960582";
        (bool s, bytes memory r) = decompressor.call(abi.encodeWithSignature("decompressed()", compressedData));
        assertEq(s, true);
        console.logBytes(r);
    }

}
