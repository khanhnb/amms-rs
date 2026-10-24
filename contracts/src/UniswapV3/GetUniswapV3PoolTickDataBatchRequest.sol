//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "../utils/BytesLib.sol";
import "./interfaces/IUniswapV3.sol";

/**
 * @dev This contract is not meant to be deployed. Instead, use a static call with the
 *       deployment bytecode as payload.
 */

contract GetUniswapV3PoolTickDataBatchRequest {
    using BytesLib for bytes;

    struct TickDataInfo {
        address pool;
        int24[] ticks;
    }

    // ticks(int24) return data sizes for the supported pool flavors
    // standard UniswapV3: 8 static words
    uint256 internal constant TICK_INFO_SIZE = 8 * 32;
    // boosted (Ramses V2 style, ex: https://mantlescan.xyz/address/0xF8090C06C9086ca9aBa39a89D6792291d0a06fd2):
    // adds boostedLiquidityGross and boostedLiquidityNet after liquidityNet
    uint256 internal constant BOOSTED_TICK_INFO_SIZE = 10 * 32;

    struct BoostedInfo {
        uint128 liquidityGross;
        int128 liquidityNet;
        uint128 boostedLiquidityGross;
        int128 boostedLiquidityNet;
        uint256 feeGrowthOutside0X128;
        uint256 feeGrowthOutside1X128;
        int56 tickCumulativeOutside;
        uint160 secondsPerLiquidityOutsideX128;
        uint32 secondsOutside;
        bool initialized;
    }

    constructor(TickDataInfo[] memory allPoolInfo) {
        IUniswapV3Pool.Info[][] memory tickInfoReturn =
            new IUniswapV3Pool.Info[][](allPoolInfo.length);

        for (uint256 i = 0; i < allPoolInfo.length; ++i) {
            IUniswapV3Pool.Info[] memory tickInfo =
                new IUniswapV3Pool.Info[](allPoolInfo[i].ticks.length);
            for (uint256 j = 0; j < allPoolInfo[i].ticks.length; ++j) {
                tickInfo[j] =
                    getTickInfo(allPoolInfo[i].pool, allPoolInfo[i].ticks[j]);
            }
            tickInfoReturn[i] = tickInfo;
        }

        // ensure abi encoding, not needed here but increase reusability for different return types
        // note: abi.encode add a first 32 bytes word with the address of the original data
        bytes memory abiEncodedData = abi.encode(tickInfoReturn);

        assembly {
            // Return from the start of the data (discarding the original data address)
            // up to the end of the memory used
            let dataStart := add(abiEncodedData, 0x20)
            return(dataStart, sub(msize(), dataStart))
        }
    }

    /// @dev Calls ticks(int24) and normalizes the result to the standard UniswapV3 Info,
    ///      detecting the pool flavor from the return data size.
    function getTickInfo(address pool, int24 tick)
        internal
        view
        returns (IUniswapV3Pool.Info memory info)
    {
        (bool success, bytes memory returnData) = pool.staticcall(
            abi.encodeWithSelector(IUniswapV3Pool.ticks.selector, tick)
        );
        if (!success) revert("ticks(int24) failed");

        if (returnData.length == TICK_INFO_SIZE) {
            info = abi.decode(returnData, (IUniswapV3Pool.Info));
        } else if (returnData.length == BOOSTED_TICK_INFO_SIZE) {
            // boosted liquidity is only used for reward accounting, not for swaps
            BoostedInfo memory boosted = abi.decode(returnData, (BoostedInfo));
            info.liquidityGross = boosted.liquidityGross;
            info.liquidityNet = boosted.liquidityNet;
            info.feeGrowthOutside0X128 = boosted.feeGrowthOutside0X128;
            info.feeGrowthOutside1X128 = boosted.feeGrowthOutside1X128;
            info.tickCumulativeOutside = boosted.tickCumulativeOutside;
            info.secondsPerLiquidityOutsideX128 =
                boosted.secondsPerLiquidityOutsideX128;
            info.secondsOutside = boosted.secondsOutside;
            info.initialized = boosted.initialized;
        } else {
            revert("unsupported ticks(int24) return data");
        }
    }
}
