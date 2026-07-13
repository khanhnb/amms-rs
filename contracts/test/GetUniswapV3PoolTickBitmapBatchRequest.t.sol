//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/UniswapV3/GetUniswapV3PoolTickBitmapBatchRequest.sol";

contract GetUniswapV3PoolTickBitmapBatchRequestTest is Test {
    GetUniswapV3PoolTickBitmapBatchRequest batch;

    function setUp() public {}

    function test_Batch() public {
        GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo[] memory input =
            new GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo[](1);
        // input[0] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
        //     0x0464a4205a6176f665037FD4B74041da7eE11585, -694, 693
        //     // 0x0464a4205a6176f665037FD4B74041da7eE11585, -694, 693
        // );
        // input[0] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
        //     0xB05088D53f2Dbc0e2723C0aFe28471736875dAd2, -3466, 3465
        // );
        // input[2] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
        //     0xF7b5113492b5F642075bBCAA02494df8f188CaDe, -58, 57
        // );
        // input[3] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
        //     0x121c12361A6726d70c53eB958F4461feE307EDdB, -347, 346
        // );
        batch = new GetUniswapV3PoolTickBitmapBatchRequest(input);
    }
}
