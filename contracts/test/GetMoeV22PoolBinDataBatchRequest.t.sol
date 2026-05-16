//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/MoeV22/GetMoeV22PoolBinDataBatchRequest.sol";

contract GetMoeV22PoolBinBitmapBatchRequestTest is Test {
    GetMoeV22PoolBinDataBatchRequest batch;
    function setUp() public {}

    function test_Batch() public {
        GetMoeV22PoolBinDataBatchRequest.PoolInfo[] memory input =
            new GetMoeV22PoolBinDataBatchRequest.PoolInfo[](1);
        input[0] = GetMoeV22PoolBinDataBatchRequest.PoolInfo(
            0xA8E84F6EaF172C8E3Cd5C53c760FCE54b29E161B, 8368806, 8368806
        );
        batch = new GetMoeV22PoolBinDataBatchRequest(input);
        console.log("Hello World");
    }
}
