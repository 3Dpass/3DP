// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev Uniswap V2 standard events
event Swap(
    address indexed sender,
    uint amount0In,
    uint amount1In,
    uint amount0Out,
    uint amount1Out,
    address indexed to
);

/// @dev Interface for swap functions exposed by the asset conversion precompile
interface ISwap {
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts);

    function swapTokensForExactTokens(
        uint256 amountOut,
        uint256 amountInMax,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts);
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

    /// @custom:selector 0x38ed1739
    /// @notice Swap an exact amount of input tokens for as many output tokens as possible along the path
    /// @dev The precompile will automatically emit a Swap event
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts) {
        amounts = swapper.swapExactTokensForTokens(
            amountIn,
            amountOutMin,
            path,
            to,
            deadline
        );
    }

    /// @custom:selector 0x8803dbee
    /// @notice Swap tokens for an exact amount of output tokens along the path
    /// @dev The precompile will automatically emit a Swap event
    function swapTokensForExactTokens(
        uint256 amountOut,
        uint256 amountInMax,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts) {
        amounts = swapper.swapTokensForExactTokens(
            amountOut,
            amountInMax,
            path,
            to,
            deadline
        );
    }
} 