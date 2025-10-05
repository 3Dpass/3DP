# Withdraw Assets Implementation

## Overview

The `withdraw_assets` function in the EVM pallet provides a solution to the irreversible address mapping issue for assets. It allows users to withdraw assets from their EVM-mapped accounts to their original Substrate accounts, similar to how the existing `withdraw` function works for native currency.

## Problem Statement

When an EVM address is mapped to a Substrate account using `HashedAddressMapping<BlakeTwo256>`, the mapping is irreversible. This means:

1. **EVM Address** → **Hashed Substrate Account** (via `HashedAddressMapping`)
2. **Original Substrate Account** ≠ **Hashed Substrate Account**

This creates a problem when users want to transfer assets from their EVM-mapped account back to their original Substrate account, as the `assets-erc20` precompile can only transfer to the hashed account, not the original one.

## Solution Architecture

### 1. Runtime-Based Direct Transfer

We use a runtime-based approach that directly transfers assets within the same runtime execution:

```rust
// Direct transfer using fungibles trait
T::AssetsPallet::transfer(
    asset_id,
    &address_account_id,
    &destination,
    value,
    false, // keep_alive
)?;
```

### 2. Address Resolution

The function uses the same address resolution mechanism as the existing `withdraw` function:

```rust
let destination = T::WithdrawOrigin::ensure_address_origin(&address, origin)?;
```

This leverages `EnsureAddressTruncated` to:
- Verify that the caller's Substrate account matches the first 20 bytes of the EVM address
- Return the original Substrate `AccountId` for the destination

## Implementation Details

### 1. Function Signature

```rust
#[pallet::weight(0)]
pub fn withdraw_assets(
    origin: OriginFor<T>,
    address: H160,
    asset_id: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::AssetId,
    value: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance,
) -> DispatchResult
```

**Parameters:**
- `origin`: The transaction origin (must be signed by the account that matches the EVM address)
- `address`: The EVM address from which assets will be withdrawn
- `asset_id`: The asset ID from the `poscan-assets` pallet (generic type from fungibles trait)
- `value`: The amount of assets to withdraw (generic type from fungibles trait)

### 2. Address Validation

```rust
let destination = T::WithdrawOrigin::ensure_address_origin(&address, origin)?;
```

This step:
1. Extracts the caller from the origin
2. Checks if the first 20 bytes of the caller's `AccountId` match the provided `H160` address
3. Returns the original `AccountId` if the match is successful
4. Fails with an error if the match fails

### 3. EVM Account Mapping

```rust
let address_account_id = T::AddressMapping::into_account_id(address);
```

This maps the EVM address to its corresponding hashed Substrate account where the assets are stored.

### 4. Direct Asset Transfer

```rust
T::AssetsPallet::transfer(
    asset_id,
    &address_account_id,
    &destination,
    value,
    false, // keep_alive
)?;
```

The function directly transfers assets from the EVM-mapped account to the original Substrate account using the fungibles trait. The transfer will fail if the destination account doesn't meet the asset's minimum balance requirement.

### 5. Success Event Emission

```rust
Self::deposit_event(Event::AssetsWithdrawn {
    evm_address: address,
    asset_id,
    amount: value,
    destination,
});
```

The function emits a success event after the transfer is completed for audit trail purposes.

## Success Event Definition

```rust
/// Assets withdrawn from EVM-mapped account to Substrate account.
AssetsWithdrawn {
    evm_address: H160,
    asset_id: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::AssetId,
    amount: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance,
    destination: T::AccountId,
},
```

**Event Fields:**
- `evm_address`: The EVM address from which assets were withdrawn
- `asset_id`: The asset ID in the `poscan-assets` pallet (generic type)
- `amount`: The amount of assets that were transferred (generic type)
- `destination`: The original Substrate account that received the assets

**Note:** This event is emitted only after the successful completion of the asset transfer for audit trail purposes.

## Integration Points

### 1. EVM Pallet Configuration

The function is integrated into the EVM pallet's call enum and can be called as an extrinsic:

```rust
// In the runtime's call enum
EVM::withdraw_assets { address, asset_id, value }
```

### 2. Runtime Integration

The function is available in the runtime through the EVM pallet configuration:

```rust
impl pallet_evm::Config for Runtime {
    // ... other config
    type WithdrawOrigin = EnsureAddressTruncated;
    type AssetsPallet = PoscanAssets;
}
```

### 3. EVM Pallet Trait Configuration

The EVM pallet requires the fungibles traits to be implemented:

```rust
// In EVM pallet Config trait
type AssetsPallet: fungibles::Inspect<Self::AccountId> 
                 + fungibles::Mutate<Self::AccountId> 
                 + fungibles::Transfer<Self::AccountId>;
```

## Usage Flow

### 1. User Calls the Function

```rust
// From Substrate side
EVM::withdraw_assets(
    origin,           // User's origin
    evm_address,      // 0x1234... (user's EVM address)
    asset_id,         // 1 (asset ID in poscan-assets)
    amount            // 1000000000000000000 (1 token with 18 decimals)
)
```

### 2. Address Validation

The function verifies that:
- The caller is signed
- The caller's account ID matches the first 20 bytes of the EVM address
- Returns the original Substrate account ID

### 3. Direct Asset Transfer

Assets are directly transferred from the EVM-mapped account to the original Substrate account within the same runtime execution.

### 4. Success Event Emission

An `AssetsWithdrawn` event is emitted after the successful transfer for audit trail purposes only.

## Comparison with Native Currency Withdraw

| Aspect | Native Currency (`withdraw`) | Assets (`withdraw_assets`) |
|--------|------------------------------|----------------------------|
| **Direct Transfer** | ✅ Uses `Currency::transfer` | ✅ Uses `fungibles::Transfer::transfer` |
| **Address Resolution** | ✅ Uses `EnsureAddressTruncated` | ✅ Uses `EnsureAddressTruncated` |
| **Pallet Integration** | ✅ Direct pallet call | ✅ Direct pallet call via trait |
| **Asset Type** | Native currency (P3D) | Any asset in `poscan-assets` |
| **Complexity** | Simple, direct | Simple, direct (same pattern) |
| **Runtime Execution** | ✅ Within runtime | ✅ Within runtime |

## Security Considerations

### 1. Origin Validation

The function ensures that only the owner of the EVM address can withdraw assets by:
- Requiring a signed origin
- Validating that the caller's account matches the EVM address

### 2. Address Mapping Integrity

The `EnsureAddressTruncated` mechanism ensures:
- The caller is the legitimate owner of the EVM address
- The destination is the original Substrate account
- No unauthorized transfers can occur

### 3. Runtime-Based Security

The runtime-based direct transfer approach provides:
- Atomic execution within the same runtime transaction
- Immediate transfer completion
- Balance validation (prevents overdrafts)
- Minimum balance enforcement for destination accounts
- Audit trail through success events (for logging only)
- No external dependencies or processing delays
- No event-driven processing or off-chain workers

## Future Enhancements

### 1. Enhanced Error Handling

The current implementation could be enhanced with more detailed error handling:

```rust
// Enhanced error handling
match T::AssetsPallet::transfer(asset_id, &address_account_id, &destination, value, false) {
    Ok(transferred_amount) => {
        // Handle successful transfer
        Self::deposit_event(Event::AssetsWithdrawn { ... });
    }
    Err(e) => {
        // Handle specific error cases
        return Err(e);
    }
}
```

### 2. Batch Withdrawals

Support for withdrawing multiple assets in a single transaction:

```rust
pub fn withdraw_assets_batch(
    origin: OriginFor<T>,
    address: H160,
    withdrawals: Vec<(
        <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::AssetId,
        <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance
    )>, // (asset_id, amount) pairs
) -> DispatchResult
```

### 3. Partial Withdrawals

Allow partial withdrawals with remaining balance checks:

```rust
pub fn withdraw_assets_partial(
    origin: OriginFor<T>,
    address: H160,
    asset_id: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::AssetId,
    value: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance,
    keep_minimum: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance, // Minimum balance to keep
) -> DispatchResult
```

## Testing Considerations

### 1. Unit Tests

- Test address validation with valid/invalid origins
- Test event emission with correct parameters
- Test error cases (insufficient permissions, invalid addresses)

### 2. Integration Tests

- Test end-to-end flow from function call to immediate asset transfer
- Test with different asset types and amounts
- Test edge cases (zero amounts, maximum amounts)
- Test fungibles trait integration
- Test atomic execution (no partial states)

### 3. Security Tests

- Test unauthorized access attempts
- Test address spoofing attempts
- Test origin validation edge cases

## Conclusion

The `withdraw_assets` implementation provides a clean, secure, and efficient solution to the irreversible address mapping problem for assets. The runtime-based direct transfer approach ensures atomic execution within the same transaction while maintaining security through proper origin validation and address resolution.

The implementation follows the same pattern as the native currency `withdraw` function, using trait abstraction to avoid circular dependencies while providing immediate asset transfers. This approach offers:

- **Immediate completion** - no event-driven processing delays
- **Atomic execution** - all operations within the same runtime transaction  
- **Audit trails** - success events for logging purposes only
- **No external dependencies** - no off-chain workers or external systems required

The solution provides a foundation for future enhancements while solving the immediate need for asset withdrawals from EVM-mapped accounts.
