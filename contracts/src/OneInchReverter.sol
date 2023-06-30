// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "./interface/IBalDiffReverter.sol";


contract OneInchReverter {

    address immutable owner = msg.sender;
    address reverter;
    address borrower;
    
    constructor(address _reverter, address _borrower) {
        setReverter(_reverter);
        setBorrower(_borrower);
    }

    function setReverter(address _reverter) public {
        require(msg.sender == owner, "only owner");
        reverter = _reverter;
    }

    function setBorrower(address _borrower) public {
        require(msg.sender == owner, "only owner");
        borrower = _borrower;
    }

    // ! Someone could call this function with custom agg, approve malicious contract and drain the reverter
    // ! Reverter should not hold any funds, but nonetheless it's not optimal
    function simulate(
        address fromToken,
        address toToken,
        uint fromAmount,
        address aggregator,
        bytes memory data
    ) external returns (int balDiff) {
        IBalDiffReverter(reverter).approve(fromToken, aggregator, type(uint).max);

        balDiff = IBalDiffReverter(reverter).borrowAndRevert(
            BorrowParams({
                borrower: address(borrower),
                borrowToken: fromToken,
                borrowAmount: fromAmount
            }), 
            ExecutionParams({
                targetToken: toToken,
                target: aggregator,
                data: data
            })
        );
    }

}