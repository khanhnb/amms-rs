//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/UniswapV3/GetUniswapV3PoolTickBitmapBatchRequest.sol";

contract GetUniswapV3PoolTickBitmapBatchRequestTest is Test {
    GetUniswapV3PoolTickBitmapBatchRequest batch;

    function setUp() public {}

    function test_Batch() public {
        GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo[]
            memory input = new GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo[](
                4
            );
        input[0] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
            0xCd07Bcf06F3Ad0EaC869BDec3E9065864A348875,
            656,
            694
        );
        input[1] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
            0xcd3848389078c1cD47038aEf975f4c3Ff7f8b31f,
            -58,
            58
        );
        input[2] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
            0xe46CFde1AFA8E87bb543Af0e064aF52Dd6805C93,
            -58,
            58
        );
        input[3] = GetUniswapV3PoolTickBitmapBatchRequest.TickBitmapInfo(
            0xEb67e93E820BE77e7ED134D39fE1E18B62E86fba,
            -3466,
            -3362
        );
        batch = new GetUniswapV3PoolTickBitmapBatchRequest(input);
    }
}
