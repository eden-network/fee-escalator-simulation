// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


import "./interface/IUniV3Pool.sol";
import "./interface/IERC20.sol";
import "./Borrower.sol";


contract UniV3Borrower is Borrower {

    mapping(address => address[]) public tokenToPool;

    constructor(address[] memory _pools) {
        for (uint i = 0; i < _pools.length; i++) {
            address pool = _pools[i];
            tokenToPool[IUniV3Pool(pool).token0()].push(pool);
            tokenToPool[IUniV3Pool(pool).token1()].push(pool);
        }
    }

    function borrowAndCall(
        address borrowToken,
        uint borrowAmount,
        address receiver,
        bytes memory data
    ) public override returns (bytes memory response) {
        (address pool, uint tokenBal) = getMostLiquidPool(borrowToken);
        // todo: borrow from multiple pools
        require(tokenBal >= borrowAmount, "Not enough liquidity");
        (uint amount0Out, uint amount1Out) = getAmountsOut(
            pool, 
            borrowToken, 
            borrowAmount
        );
        bytes memory fullData = abi.encode(receiver, data);
        (, response) = pool.call(bytes(abi.encodeWithSignature(
            "flash(address,uint256,uint256,bytes)", 
            receiver, 
            amount0Out, 
            amount1Out, 
            fullData
        )));
    }

    function uniswapV3FlashCallback(
        uint256,  // amount0 
        uint256,  // amount1
        bytes calldata data
    ) external {
        (address callTarget, bytes memory callData) = abi.decode(data, (address, bytes));
        (, bytes memory errMsg) = callTarget.call(callData);
        propagateError(errMsg);
    }

    function getMostLiquidPool(
        address token
    ) public view returns (address bestPool, uint bestTokenBal) {
        address[] memory pools = tokenToPool[token];
        for (uint i = 0; i < pools.length; i++) {
            uint tokenBal = IERC20(token).balanceOf(pools[i]);
            if (tokenBal > bestTokenBal) {
                bestTokenBal = tokenBal;
                bestPool = pools[i];
            }
        }
    }

    function getAmountsOut(
        address pool, 
        address targetToken, 
        uint targetAmount
    ) internal view returns (uint amount0, uint amount1) {
        (amount0, amount1) = targetToken == IUniV3Pool(pool).token0()
            ? (targetAmount, uint(0))
            : (uint(0), targetAmount);
    }

    function propagateError(bytes memory errMsg) internal pure {
        assembly {
            revert(add(0x20, errMsg), mload(errMsg))
        }
    }

}
