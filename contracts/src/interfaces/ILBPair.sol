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

    function getBin(uint24 id)
        external
        view
        returns (uint128 binReserveX, uint128 binReserveY);

    function getNextNonEmptyBin(bool swapForY, uint24 id)
        external
        view
        returns (uint24 nextId);

    function totalSupply(uint256 id) external view returns (uint256);
}
