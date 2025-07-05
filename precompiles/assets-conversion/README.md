# Assets Conversion Precompile

The Assets Conversion Precompile provides an interface for managing liquidity pools and token swaps on the 3Dpass network. It allows users to create pools, add/remove liquidity, and perform token swaps between different assets.

## Address
```
0x0000000000000000000000000000000000000902
```

## Functions

### View Functions

#### `queryPairs()`
Returns all available liquidity pools with their token information.

**Returns:**
- `(bool, address, (bool, address), bool, (bool, address), bool)[]` - Array of pool information
  - `bool` - `isValid` - Whether the pool is valid
  - `address` - `pair_id` - The LP token address for the pool
  - `(bool, address)` - `token0` - (hasData, id) for first token
  - `bool` - `token0_is_native` - Whether token0 is the native token
  - `(bool, address)` - `token1` - (hasData, id) for second token
  - `bool` - `token1_is_native` - Whether token1 is the native token

**Example:**
```solidity
// Get all pools
(bool[] memory isValid, address[] memory pairIds, ...) = assetsConversion.queryPairs();
```

#### `nextPoolAssetId()`
Returns the next available pool asset ID.

**Returns:**
- `(bool, uint32)` - (exists, poolAssetId)
  - `bool` - Whether a next pool asset ID exists
  - `uint32` - The next pool asset ID

**Example:**
```solidity
(bool exists, uint32 nextId) = assetsConversion.nextPoolAssetId();
```

### State-Changing Functions

#### `createPool(address asset1, address asset2)`
Creates a new liquidity pool between two assets.

**Parameters:**
- `address asset1` - First asset address (native token or local asset)
- `address asset2` - Second asset address (native token or local asset)

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Create pool between native token and asset ID 1
address nativeToken = 0x0000000000000000000000000000000000000802;
address asset1 = 0xfbfbfbfa00000000000000000000000000000001; // Asset ID 1
bool success = assetsConversion.createPool(nativeToken, asset1);
```

#### `addLiquidity(address asset1, address asset2, uint256 amount1_desired, uint256 amount2_desired, uint256 amount1_min, uint256 amount2_min, address mint_to)`
Adds liquidity to an existing pool.

**Parameters:**
- `address asset1` - First asset address
- `address asset2` - Second asset address
- `uint256 amount1_desired` - Desired amount of asset1
- `uint256 amount2_desired` - Desired amount of asset2
- `uint256 amount1_min` - Minimum amount of asset1 to accept
- `uint256 amount2_min` - Minimum amount of asset2 to accept
- `address mint_to` - Address to receive LP tokens

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Add liquidity to pool
uint256 amount1 = 1000 * 10**18; // 1000 tokens
uint256 amount2 = 500 * 10**18;  // 500 tokens
bool success = assetsConversion.addLiquidity(
    asset1, asset2, amount1, amount2, amount1, amount2, msg.sender
);
```

#### `removeLiquidity(address asset1, address asset2, uint256 lp_token_burn, uint256 amount1_min_receive, uint256 amount2_min_receive, address withdraw_to)`
Removes liquidity from a pool.

**Parameters:**
- `address asset1` - First asset address
- `address asset2` - Second asset address
- `uint256 lp_token_burn` - Amount of LP tokens to burn
- `uint256 amount1_min_receive` - Minimum amount of asset1 to receive
- `uint256 amount2_min_receive` - Minimum amount of asset2 to receive
- `address withdraw_to` - Address to receive the tokens

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Remove liquidity
uint256 lpTokens = 100 * 10**18; // 100 LP tokens
bool success = assetsConversion.removeLiquidity(
    asset1, asset2, lpTokens, 0, 0, msg.sender
);
```

#### `swapExactTokensForTokens(address[] path, uint256 amount_in, uint256 amount_out_min, address send_to, bool keep_alive)`
Swaps an exact amount of input tokens for output tokens.

**Parameters:**
- `address[] path` - Array of token addresses representing the swap path
- `uint256 amount_in` - Exact amount of input tokens
- `uint256 amount_out_min` - Minimum amount of output tokens to receive
- `address send_to` - Address to receive the output tokens
- `bool keep_alive` - Whether to keep the sender alive (for dust protection)

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Swap exact tokens for tokens
address[] memory path = new address[](2);
path[0] = asset1;
path[1] = asset2;
uint256 amountIn = 100 * 10**18; // 100 tokens
uint256 amountOutMin = 95 * 10**18; // 95 tokens minimum
bool success = assetsConversion.swapExactTokensForTokens(
    path, amountIn, amountOutMin, msg.sender, true
);
```

#### `swapTokensForExactTokens(address[] path, uint256 amount_out, uint256 amount_in_max, address send_to, bool keep_alive)`
Swaps tokens for an exact amount of output tokens.

**Parameters:**
- `address[] path` - Array of token addresses representing the swap path
- `uint256 amount_out` - Exact amount of output tokens to receive
- `uint256 amount_in_max` - Maximum amount of input tokens to spend
- `address send_to` - Address to receive the output tokens
- `bool keep_alive` - Whether to keep the sender alive (for dust protection)

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Swap tokens for exact tokens
address[] memory path = new address[](2);
path[0] = asset1;
path[1] = asset2;
uint256 amountOut = 100 * 10**18; // 100 tokens exact
uint256 amountInMax = 105 * 10**18; // 105 tokens maximum
bool success = assetsConversion.swapTokensForExactTokens(
    path, amountOut, amountInMax, msg.sender, true
);
```

## Address Format

### Native Token Address
```
0x0000000000000000000000000000000000000802
```

### Local Asset Address Format
```
0xfbfbfbfa00000000000000000000000000000001
```
- Prefix: `0xfbfbfbfa` (4 bytes)
- Asset ID: `0x00000000000000000000000000000001` (16 bytes, big-endian)

## Usage Examples

### Complete Pool Creation and Liquidity Addition
```solidity
// 1. Create a pool
address nativeToken = 0x0000000000000000000000000000000000000802;
address asset1 = 0xfbfbfbfa00000000000000000000000000000001;
bool created = assetsConversion.createPool(nativeToken, asset1);

// 2. Add liquidity
uint256 amount1 = 1000 * 10**18;
uint256 amount2 = 500 * 10**18;
bool added = assetsConversion.addLiquidity(
    nativeToken, asset1, amount1, amount2, amount1, amount2, msg.sender
);
```

### Token Swap Example
```solidity
// Swap native tokens for asset1
address[] memory path = new address[](2);
path[0] = 0x0000000000000000000000000000000000000802; // Native token
path[1] = 0xfbfbfbfa00000000000000000000000000000001; // Asset1

uint256 amountIn = 100 * 10**18;
uint256 amountOutMin = 95 * 10**18;

bool success = assetsConversion.swapExactTokensForTokens(
    path, amountIn, amountOutMin, msg.sender, true
);
```

## Error Handling

The precompile will revert with descriptive error messages for:
- Invalid asset addresses
- Insufficient liquidity
- Slippage protection (amount_out_min not met)
- Overflow/underflow conditions
- Invalid pool operations

## Gas Considerations

- Pool creation: ~50,000 gas
- Adding liquidity: ~100,000 gas
- Removing liquidity: ~80,000 gas
- Token swaps: ~120,000 gas (varies with path length)

## Security Notes

- Always use slippage protection (`amount_out_min` or `amount_in_max`)
- Verify asset addresses before use
- Check pool existence before operations
- Use `keep_alive: true` for dust protection in swaps 