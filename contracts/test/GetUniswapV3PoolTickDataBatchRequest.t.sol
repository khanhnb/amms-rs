//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/UniswapV3/GetUniswapV3PoolTickDataBatchRequest.sol";

contract GetUniswapV3PoolTickDataBatchRequestTest is Test {
    GetUniswapV3PoolTickDataBatchRequest batch;

    function setUp() public {}

    int24[] public ticks = [
        -887220,
        -82920,
        -80400,
        -79740,
        -79680,
        -77520,
        -77100,
        -75900,
        -74880
    ];

    address public pool = 0xF8090C06C9086ca9aBa39a89D6792291d0a06fd2;

    function test_Batch() public {
        console.log("start");
        GetUniswapV3PoolTickDataBatchRequest.TickDataInfo[]
            memory input = new GetUniswapV3PoolTickDataBatchRequest.TickDataInfo[](
                1
            );

        input[0] = GetUniswapV3PoolTickDataBatchRequest.TickDataInfo(
            pool,
            ticks
        );
        batch = new GetUniswapV3PoolTickDataBatchRequest(input);
        console.log("done");
    }
}
