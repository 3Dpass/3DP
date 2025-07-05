// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev Uniswap V2 standard events
event Mint(address indexed sender, uint amount0, uint amount1);
event Burn(address indexed sender, uint amount0, uint amount1, address indexed to);

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
    ) external returns (uint256 amountA, uint256 amountB, uint256 liquidity);
    function removeLiquidity(
        address asset1,
        address asset2,
        uint256 lpTokenBurn,
        uint256 amount1MinReceive,
        uint256 amount2MinReceive,
        address withdrawTo
    ) external returns (uint256 amountA, uint256 amountB);
}

/// @title LiquidityProvider
/// @notice Front-end contract for adding and removing liquidity via the asset conversion precompile
contract LiquidityProvider {
    /// @dev Emitted when liquidity is added to a pool
    event LiquidityAdded(address indexed asset1, address indexed asset2, address indexed mintTo, uint256 amount1Desired, uint256 amount2Desired);
    /// @dev Emitted when liquidity is removed from a pool
    event LiquidityRemoved(address indexed asset1, address indexed asset2, address indexed withdrawTo, uint256 lpTokenBurn);

    IAssetsConversion public immutable assetsConversion;

    /// @dev Precompile address for asset conversion
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;

    /// @dev Sets the precompile address for asset conversion
    constructor() {
        assetsConversion = IAssetsConversion(PRECOMPILE_ADDR);
    }

    /// @custom:selector 0x7ce7d726
    /// @notice Add liquidity to a pool
    /// @dev The precompile will automatically emit a Mint event. This contract also emits a custom LiquidityAdded event.
    function addLiquidity(
        address asset1,
        address asset2,
        uint256 amount1Desired,
        uint256 amount2Desired,
        uint256 amount1Min,
        uint256 amount2Min,
        address mintTo
    ) external returns (uint256 amountA, uint256 amountB, uint256 liquidity) {
        (amountA, amountB, liquidity) = assetsConversion.addLiquidity(
            asset1,
            asset2,
            amount1Desired,
            amount2Desired,
            amount1Min,
            amount2Min,
            mintTo
        );
        emit LiquidityAdded(asset1, asset2, mintTo, amount1Desired, amount2Desired);
    }

    /// @custom:selector 0x12b49fc2
    /// @notice Remove liquidity from a pool
    /// @dev The precompile will automatically emit a Burn event. This contract also emits a custom LiquidityRemoved event.
    function removeLiquidity(
        address asset1,
        address asset2,
        uint256 lpTokenBurn,
        uint256 amount1MinReceive,
        uint256 amount2MinReceive,
        address withdrawTo
    ) external returns (uint256 amountA, uint256 amountB) {
        (amountA, amountB) = assetsConversion.removeLiquidity(
            asset1,
            asset2,
            lpTokenBurn,
            amount1MinReceive,
            amount2MinReceive,
            withdrawTo
        );
        emit LiquidityRemoved(asset1, asset2, withdrawTo, lpTokenBurn);
    }
} 