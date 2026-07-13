//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "../src/UniswapV2/GetUniswapV2PoolDataBatchRequest.sol";
import "../src/MoeV22/GetMoeV22PoolDataBatchRequest.sol";

contract GetPoolDataBatchRequest is Test {
    GetUniswapV2PoolDataBatchRequest uniV2;
    GetMoeV22PoolDataBatchRequest moeV22;
    function setUp() public {}

    function test_univ2() public {
        address[] memory input = new address[](1);
        input[0] = 0xEFC38C1B0d60725B824EBeE8D431aBFBF12BC953;
        uniV2 = new GetUniswapV2PoolDataBatchRequest(input);
    }

    function test_moev22() public {
        address[] memory input = new address[](1);
        input[0] = 0xA8E84F6EaF172C8E3Cd5C53c760FCE54b29E161B;
        moeV22 = new GetMoeV22PoolDataBatchRequest(input);
    } 
}
