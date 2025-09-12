//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "../utils/BytesLib.sol";

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

    struct Info {
        bool initialized;
        uint128 liquidityGross;
        int128 liquidityNet;
    }

    constructor(TickDataInfo[] memory allPoolInfo) {
        Info[][] memory tickInfoReturn = new Info[][](allPoolInfo.length);

        for (uint256 i = 0; i < allPoolInfo.length; ++i) {
            Info[] memory tickInfo = new Info[](allPoolInfo[i].ticks.length);
            for (uint256 j = 0; j < allPoolInfo[i].ticks.length; ++j) {
                // IUniswapV3PoolState.Info memory tick = IUniswapV3PoolState(
                //     allPoolInfo[i].pool
                // ).ticks(allPoolInfo[i].ticks[j]);

                // work around - some AMMs have different return data struct;
                // Ex: https://mantlescan.xyz/address/0xF8090C06C9086ca9aBa39a89D6792291d0a06fd2
                (bool success, bytes memory returnData) = allPoolInfo[i]
                    .pool
                    .call(
                        abi.encodeWithSignature(
                            "ticks(int24)",
                            allPoolInfo[i].ticks[j]
                        )
                    );
                if (!success) revert("ticks(int24) failed");

                (uint128 liquidityGross, int128 liquidityNet) = abi.decode(
                    returnData,
                    (uint128, int128)
                );
                tickInfo[j] = Info({
                    liquidityGross: liquidityGross,
                    liquidityNet: liquidityNet,
                    initialized: returnData[returnData.length - 1] != 0x00
                });
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
}
