// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


error StrategicErrorInt(int);

bytes4 constant STRATEGIC_ERR_INT_SIG = bytes4(keccak256("StrategicErrorInt(int256)"));

function decodeStrategicErrInt(bytes memory res) pure returns (int output) {
    bytes4 errSig4bytes;
    assembly {
        errSig4bytes := mload(add(res, 0x20))
        output := mload(add(res, 0x24))
    }
    require(errSig4bytes == STRATEGIC_ERR_INT_SIG, "Not a StrategicErrorInt");
}