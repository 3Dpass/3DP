// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev Interface for swap functions exposed by the asset conversion precompile
interface ISwap {
    function swapExactTokensForTokens(
        address[] memory path,
        uint256 amountIn,
        uint256 amountOutMin,
        address sendTo,
        bool keepAlive
    ) external returns (bool success);

    function swapTokensForExactTokens(
        address[] memory path,
        uint256 amountOut,
        uint256 amountInMax,
        address sendTo,
        bool keepAlive
    ) external returns (bool success);
}

/// @title Swap
/// @notice Front-end contract for interacting with the asset conversion precompile's swap functionality
contract Swap {
    ISwap public immutable swapper;

    /// @dev Precompile address for asset conversion
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;

    constructor() {
        swapper = ISwap(PRECOMPILE_ADDR);
    }

    /// @custom:selector 0x1c0c433a
    /// @notice Swap an exact amount of input tokens for as many output tokens as possible along the path
    function swapExactTokensForTokens(
        address[] memory path,
        uint256 amountIn,
        uint256 amountOutMin,
        address sendTo,
        bool keepAlive
    ) external returns (bool success) {
        success = swapper.swapExactTokensForTokens(
            path,
            amountIn,
            amountOutMin,
            sendTo,
            keepAlive
        );
    }

    /// @custom:selector 0x7c025200
    /// @notice Swap tokens for an exact amount of output tokens along the path
    function swapTokensForExactTokens(
        address[] memory path,
        uint256 amountOut,
        uint256 amountInMax,
        address sendTo,
        bool keepAlive
    ) external returns (bool success) {
        success = swapper.swapTokensForExactTokens(
            path,
            amountOut,
            amountInMax,
            sendTo,
            keepAlive
        );
    }
} 