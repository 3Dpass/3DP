// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev Uniswap V2 standard events
event Sync(uint112 reserve0, uint112 reserve1);
event PairCreated(address indexed token0, address indexed token1, address pair, uint256 allPairsLength);

interface IAssetsConversion {
    // Custom function unique to this system
    function nextPoolAssetId() external view returns (bool exists, uint256 id);
    
    // Uniswap V2 compatible functions
    function allPairsLength() external view returns (uint256);
    function allPairs(uint256 index) external view returns (address pair);
    function getPair(address tokenA, address tokenB) external view returns (address pair);
    function createPair(address tokenA, address tokenB) external returns (address pair);
}

contract Pairs {
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;
    
    // Custom function selector
    bytes4 constant SELECTOR_NEXT_POOL_ASSET_ID = 0xb422c986;
    
    // Uniswap V2 standard selectors
    bytes4 constant SELECTOR_ALL_PAIRS_LENGTH = 0x21b7f165;
    bytes4 constant SELECTOR_ALL_PAIRS = 0x7160c041;
    bytes4 constant SELECTOR_GET_PAIR = 0xc1efffec;
    bytes4 constant SELECTOR_CREATE_PAIR = 0xed95fd3e;

    /// @notice Calls the precompile to get the next pool asset id (if any).
    function nextPoolAssetId() public view returns (bool exists, uint256 id) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_NEXT_POOL_ASSET_ID);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        (exists, id) = abi.decode(data, (bool, uint256));
    }
    
    // Uniswap V2 compatible functions
    
    /// @custom:selector 0x21b7f165
    /// @notice Get the total number of pairs (Uniswap V2 compatible)
    function allPairsLength() public view returns (uint256) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_ALL_PAIRS_LENGTH);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        return abi.decode(data, (uint256));
    }
    
    /// @custom:selector 0x7160c041
    /// @notice Get pair at index (Uniswap V2 compatible)
    function allPairs(uint256 index) public view returns (address pair) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_ALL_PAIRS, index);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        return abi.decode(data, (address));
    }
    
    /// @custom:selector 0xc1efffec
    /// @notice Get pair for token pair (Uniswap V2 compatible)
    function getPair(address tokenA, address tokenB) public view returns (address pair) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_GET_PAIR, tokenA, tokenB);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        return abi.decode(data, (address));
    }
    
    /// @custom:selector 0xed95fd3e
    /// @notice Create pair for token pair (Uniswap V2 compatible)
    function createPair(address tokenA, address tokenB) public returns (address pair) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_CREATE_PAIR, tokenA, tokenB);
        (bool ok, bytes memory data) = PRECOMPILE_ADDR.call(input);
        require(ok, "Precompile call failed");
        return abi.decode(data, (address));
    }
} 