//SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface ILBPair {
    function getTokenX() external view returns (address tokenX);
    function getTokenY() external view returns (address tokenY);
    function getBinStep() external view returns (uint16 binStep);
    function getReserves()
        external
        view
        returns (uint128 reserveX, uint128 reserveY);
    function getActiveId() external view returns (uint24 activeId);
    function getBin(uint24 id)
        external
        view
        returns (uint128 binReserveX, uint128 binReserveY);
}

interface IERC20 {
    function decimals() external view returns (uint8);
}

/**
 * @dev This contract is not meant to be deployed. Instead, use a static call with the
 *       deployment bytecode as payload.
 */
contract GetMoeV22PoolDataBatchRequest {
    struct PoolData {
        address tokenA;
        address tokenB;
        uint128 reserve0;
        uint128 reserve1;
        uint8 tokenADecimals;
        uint8 tokenBDecimals;
        string symbolA;
        string symbolB;
    }

    constructor(address[] memory pools) {
        PoolData[] memory allPoolData = new PoolData[](pools.length);

        for (uint256 i = 0; i < pools.length; ++i) {
            address poolAddress = pools[i];

            if (codeSizeIsZero(poolAddress)) continue;

            PoolData memory poolData;

            // Get tokens A and B
            poolData.tokenA = ILBPair(poolAddress).getTokenX();
            poolData.tokenB = ILBPair(poolAddress).getTokenY();

            // Check that tokenA and tokenB do not have codesize of 0
            if (codeSizeIsZero(poolData.tokenA)) continue;
            if (codeSizeIsZero(poolData.tokenB)) continue;

            // Get tokenA decimals and symbol
            (bool symbolASuccess, bytes memory symbolAData) = poolData.tokenA
            .call{gas: 20000}(
                abi.encodeWithSignature("symbol()")
            );
            (bool tokenADecimalsSuccess, bytes memory tokenADecimalsData) = poolData.tokenA
            .call{gas: 20000}(
                abi.encodeWithSignature("decimals()")
            );

            if (tokenADecimalsSuccess && symbolASuccess) {
                poolData.symbolA = abi.decode(symbolAData, (string));
                uint256 tokenADecimals;

                if (tokenADecimalsData.length == 32) {
                    (tokenADecimals) = abi.decode(tokenADecimalsData, (uint256));

                    if (tokenADecimals == 0 || tokenADecimals > 255) {
                        continue;
                    } else {
                        poolData.tokenADecimals = uint8(tokenADecimals);
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            }

            // Get tokenA decimals and symbol
            (bool symbolBSuccess, bytes memory symbolBData) = poolData.tokenB
            .call{gas: 20000}(
                abi.encodeWithSignature("symbol()")
            );
            (bool tokenBDecimalsSuccess, bytes memory tokenBDecimalsData) = poolData.tokenB
            .call{gas: 20000}(
                abi.encodeWithSignature("decimals()")
            );

            if (tokenBDecimalsSuccess && symbolBSuccess) {
                poolData.symbolB = abi.decode(symbolBData, (string));
                uint256 tokenBDecimals;

                if (tokenBDecimalsData.length == 32) {
                    (tokenBDecimals) = abi.decode(tokenBDecimalsData, (uint256));

                    if (tokenBDecimals == 0 || tokenBDecimals > 255) {
                        continue;
                    } else {
                        poolData.tokenBDecimals = uint8(tokenBDecimals);
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            }

            // Get reserves
            (poolData.reserve0, poolData.reserve1) =
                ILBPair(poolAddress).getReserves();

            allPoolData[i] = poolData;
        }

        // ensure abi encoding, not needed here but increase reusability for different return types
        // note: abi.encode add a first 32 bytes word with the address of the original data
        bytes memory _abiEncodedData = abi.encode(allPoolData);

        assembly {
            // Return from the start of the data (discarding the original data address)
            // up to the end of the memory used
            let dataStart := add(_abiEncodedData, 0x20)
            return(dataStart, sub(msize(), dataStart))
        }
    }

    function codeSizeIsZero(address target) internal view returns (bool) {
        if (target.code.length == 0) {
            return true;
        } else {
            return false;
        }
    }
}
