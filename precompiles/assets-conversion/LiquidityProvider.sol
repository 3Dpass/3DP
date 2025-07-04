// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title LiquidityProvider
/// @notice Front-end contract for adding and removing liquidity via the asset conversion precompile
contract LiquidityProvider {
    /// @dev Emitted when liquidity is added to a pool
    event LiquidityAdded(address indexed asset1, address indexed asset2, address indexed mintTo, uint256 amount1Desired, uint256 amount2Desired);
    /// @dev Emitted when liquidity is removed from a pool
    event LiquidityRemoved(address indexed asset1, address indexed asset2, address indexed withdrawTo, uint256 lpTokenBurn);

    /// @dev Interface for the assets conversion precompile
    interface IAssetsConversion {
        function addLiquidity(
            address asset1,
            address asset2,
            uint256 amount1Desired,
            uint256 amount2Desired,
            uint256 amount1Min,
            uint256 amount2Min,
            address mintTo
        ) external returns (bool success);
        function removeLiquidity(
            address asset1,
            address asset2,
            uint256 lpTokenBurn,
            uint256 amount1MinReceive,
            uint256 amount2MinReceive,
            address withdrawTo
        ) external returns (bool success);
    }

    IAssetsConversion public immutable assetsConversion;

    /// @dev Precompile address for asset conversion
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;

    /// @dev Sets the precompile address for asset conversion
    constructor() {
        assetsConversion = IAssetsConversion(PRECOMPILE_ADDR);
    }

    /// @custom:selector 0x2195995c
    /// @notice Add liquidity to a pool
    /// @dev Emits a LiquidityAdded event on success
    function addLiquidity(
        address asset1,
        address asset2,
        uint256 amount1Desired,
        uint256 amount2Desired,
        uint256 amount1Min,
        uint256 amount2Min,
        address mintTo
    ) external returns (bool success) {
        success = assetsConversion.addLiquidity(
            asset1,
            asset2,
            amount1Desired,
            amount2Desired,
            amount1Min,
            amount2Min,
            mintTo
        );
        if (success) {
            emit LiquidityAdded(asset1, asset2, mintTo, amount1Desired, amount2Desired);
        }
    }

    /// @custom:selector 0x5b0d5984
    /// @notice Remove liquidity from a pool
    /// @dev Emits a LiquidityRemoved event on success
    function removeLiquidity(
        address asset1,
        address asset2,
        uint256 lpTokenBurn,
        uint256 amount1MinReceive,
        uint256 amount2MinReceive,
        address withdrawTo
    ) external returns (bool success) {
        success = assetsConversion.removeLiquidity(
            asset1,
            asset2,
            lpTokenBurn,
            amount1MinReceive,
            amount2MinReceive,
            withdrawTo
        );
        if (success) {
            emit LiquidityRemoved(asset1, asset2, withdrawTo, lpTokenBurn);
        }
    }
} 