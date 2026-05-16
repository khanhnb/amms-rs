//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// import "../utils/BytesLib.sol";
import "../interfaces/ILBPair.sol";
import "forge-std/console.sol";
/**
 * @dev This contract is not meant to be deployed. Instead, use a static call with the
 *       deployment bytecode as payload.
 */

contract GetMoeV22PoolBinDataBatchRequest {
    // using BytesLib for bytes;
    uint256 private constant OFFSET = 128;
    uint256 private constant MASK_128 = 0xffffffffffffffffffffffffffffffff;

    struct PoolInfo {
        address pool;
        uint24 minId;
        uint24 maxId;
    }
    //
    // struct TickInfo {
    //     uint128 liquidityGross;
    //     int128 liquidityNet;
    //     bool initialized;
    // }
    //
    // struct PoolData {
    //     uint256[] tickBitmap;
    //     int24[] tickIndices;
    //     TickInfo[] ticks;
    // }

    struct BinData {
        uint24 id;
        bytes32 reserves;
    }

    constructor(PoolInfo[] memory poolInfo) {
        BinData[][] memory allPoolData = new BinData[][](poolInfo.length);
        for (uint256 i = 0; i < poolInfo.length; ++i) {
            uint24 binDataIndex = 0;
            uint24 length = poolInfo[i].maxId - poolInfo[i].minId + 1;
            BinData[] memory allBinData = new BinData[](length);
            address pool = poolInfo[i].pool;
            // TODO: fix this condition id != 0
            for (
                uint24 id = poolInfo[i].minId;
                id <= poolInfo[i].maxId && id != 0;
                id = ILBPair(pool).getNextNonEmptyBin(false, id)
            ) {
                (uint128 reserveX, uint128 reserveY) = ILBPair(pool).getBin(id);
                if (reserveX == 0 && reserveY == 0) {
                    continue;
                }
                allBinData[binDataIndex] = BinData(id, encode(reserveX, reserveY));
                ++binDataIndex;
            }
            assembly {
                mstore(allBinData, binDataIndex)
            }
            allPoolData[i] = allBinData;
        }
        //
        // // ensure abi encoding, not needed here but increase reusability for different return types
        // // note: abi.encode add a first 32 bytes word with the address of the original data
        bytes memory abiEncodedData = abi.encode(allPoolData);
        //
        assembly {
            // Return from the start of the data (discarding the original data address)
            // up to the end of the memory used
            let dataStart := add(abiEncodedData, 0x20)
            return(dataStart, sub(msize(), dataStart))
        }
    }

    /**
     * @dev Encodes two uint128 into a single bytes32
     * @param x1 The first uint128
     * @param x2 The second uint128
     * @return z The encoded bytes32 as follows:
     * [0 - 128[: x1
     * [128 - 256[: x2
     */
    function encode(uint128 x1, uint128 x2) internal pure returns (bytes32 z) {
        assembly {
            z := or(and(x1, MASK_128), shl(OFFSET, x2))
        }
    }
}
