// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface IAssetsConversion {
    /// @custom:selector 0xA1B2C3D4
    /// @dev Token struct with explicit presence and native flags
    struct Token {
        bool hasData; // true if the token address is valid
        address id;
        bool isNative;
    }
    /// @custom:selector 0xA1B2C3D4
    /// @dev Pair struct with explicit validity and token info
    struct Pair {
        bool isValid; // true if both tokens are valid
        address id;
        Token token0;
        Token token1;
    }
    /// @custom:selector 0xA1B2C3D4
    /// @notice Query all pairs
    function queryPairs() external view returns (Pair[] memory);
    function nextPoolAssetId() external view returns (bool exists, uint256 id);
    /// @custom:selector 0xC1C2C3C4
    function createPool(address asset1, address asset2) external returns (bool success);
}

contract Pairs {
    struct TokenInfo {
        address id;
        bool isNative;
    }

    struct PairInfo {
        address id;
        TokenInfo token0;
        TokenInfo token1;
    }

    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000902;
    bytes4 constant SELECTOR_QUERY_PAIRS = 0xA1B2C3D4;
    /// @custom:selector 0xB1B2B3B4
    bytes4 constant SELECTOR_NEXT_POOL_ASSET_ID = 0xB1B2B3B4;
    bytes4 constant SELECTOR_CREATE_POOL = 0x0c53c51c;

    /// @notice Calls the precompile and decodes the result into an array of PairInfo structs.
    function queryPairs() public view returns (PairInfo[] memory pairs) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_QUERY_PAIRS);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        pairs = _decodePairs(data);
    }

    /// @notice Decodes the output from the precompile (array of (address, address, bool, address, bool))
    ///         into an array of PairInfo structs.
    /// @param data The ABI-encoded bytes output from the precompile
    /// @return pairs Array of PairInfo structs
    function _decodePairs(bytes memory data) internal pure returns (PairInfo[] memory pairs) {
        uint256 tupleSize = 5 * 32;
        require(data.length % tupleSize == 0, "Invalid data length");
        uint256 numPairs = data.length / tupleSize;
        pairs = new PairInfo[](numPairs);

        for (uint256 i = 0; i < numPairs; i++) {
            uint256 offset = i * tupleSize;
            address pairId;
            address token0Id;
            bool token0IsNative;
            address token1Id;
            bool token1IsNative;

            assembly {
                pairId := mload(add(data, add(32, offset)))
                token0Id := mload(add(data, add(64, offset)))
                token0IsNative := mload(add(data, add(96, offset)))
                token1Id := mload(add(data, add(128, offset)))
                token1IsNative := mload(add(data, add(160, offset)))
            }

            pairs[i] = PairInfo({
                id: pairId,
                token0: TokenInfo(token0Id, token0IsNative),
                token1: TokenInfo(token1Id, token1IsNative)
            });
        }
    }

    /// @notice Calls the precompile to get the next pool asset id (if any).
    function nextPoolAssetId() public view returns (bool exists, uint256 id) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_NEXT_POOL_ASSET_ID);
        (bool success, bytes memory data) = PRECOMPILE_ADDR.staticcall(input);
        require(success, "Precompile call failed");
        (exists, id) = abi.decode(data, (bool, uint256));
    }

    /// @notice Calls the precompile to create a pool with two assets.
    function createPool(address asset1, address asset2) public returns (bool success) {
        bytes memory input = abi.encodeWithSelector(SELECTOR_CREATE_POOL, asset1, asset2);
        (bool ok, bytes memory data) = PRECOMPILE_ADDR.call(input);
        require(ok, "Precompile call failed");
        success = abi.decode(data, (bool));
    }
} 