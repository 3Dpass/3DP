// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev Uniswap V2 standard events
/// @custom:selector 0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822
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

contract SwapRouter {
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;
    
    // Uniswap V2 standard selectors
    bytes4 constant SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS = 0x38ed1739;
    bytes4 constant SELECTOR_SWAP_TOKENS_FOR_EXACT_TOKENS = 0x8803dbee;

    /// @custom:selector 0x38ed1739
    /// @notice Swap exact tokens for tokens (Uniswap V2 compatible)
    function swapExactTokensForTokens(
        uint256 amountIn,
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts) {
        bytes memory input = abi.encodeWithSelector(
            SELECTOR_SWAP_EXACT_TOKENS_FOR_TOKENS,
            amountIn,
            amountOutMin,
            path,
            to,
            deadline
        );
        (bool success, bytes memory data) = PRECOMPILE_ADDR.call(input);
        require(success, "Precompile call failed");
        return abi.decode(data, (uint[]));
    }

    /// @custom:selector 0x8803dbee
    /// @notice Swap tokens for exact tokens (Uniswap V2 compatible)
    function swapTokensForExactTokens(
        uint256 amountOut,
        uint256 amountInMax,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external returns (uint[] memory amounts) {
        bytes memory input = abi.encodeWithSelector(
            SELECTOR_SWAP_TOKENS_FOR_EXACT_TOKENS,
            amountOut,
            amountInMax,
            path,
            to,
            deadline
        );
        (bool success, bytes memory data) = PRECOMPILE_ADDR.call(input);
        require(success, "Precompile call failed");
        return abi.decode(data, (uint[]));
    }
}