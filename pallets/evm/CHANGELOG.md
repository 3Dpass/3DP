# Changelog for `pallet-evm`

## Unreleased
- Added associated type `BlockHashMapping` that requires a `BlockHashMapping` trait implementor. Projects that integrate pallet-ethereum can use this trait to return the ethereum block hash when using `blockhash` Solidity function.

## Added `withdraw_assets` Function

### Overview
Added `withdraw_assets` function to solve the irreversible address mapping issue for assets. This allows users to withdraw assets from their EVM-mapped accounts to their original Substrate accounts, similar to how the existing `withdraw` function works for native currency.

### Problem Solved
When an EVM address is mapped to a Substrate account using `HashedAddressMapping<BlakeTwo256>`, the mapping is irreversible. This creates a problem when users want to transfer assets from their EVM-mapped account back to their original Substrate account, as the `assets-erc20` precompile can only transfer to the hashed account, not the original one.

### Implementation Details

#### Function Signature
```rust
#[pallet::weight(0)]
pub fn withdraw_assets(
    origin: OriginFor<T>,
    address: H160,
    asset_id: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::AssetId,
    value: <T::AssetsPallet as fungibles::Inspect<T::AccountId>>::Balance,
) -> DispatchResult
```

#### Key Features
- **Runtime-based direct transfer**: Assets are transferred immediately within the same runtime execution
- **Address resolution**: Uses `EnsureAddressTruncated` to verify caller ownership and resolve original Substrate account
- **Trait abstraction**: Uses `fungibles::Transfer` trait to avoid circular dependencies
- **Account cleanup**: Uses `keep_alive: false` to clean up empty EVM accounts (consistent with native currency `withdraw`)
- **Audit trail**: Emits `AssetsWithdrawn` event after successful transfer

#### Configuration Requirements
- Added `AssetsPallet` associated type to EVM pallet Config trait:
  ```rust
  type AssetsPallet: fungibles::Inspect<Self::AccountId> 
                   + fungibles::Mutate<Self::AccountId> 
                   + fungibles::Transfer<Self::AccountId>;
  ```
- Runtime must configure: `type AssetsPallet = PoscanAssets;`

#### Usage
```rust
// From Substrate side
EVM::withdraw_assets(
    origin,           // User's origin
    evm_address,      // 0x1234... (user's EVM address)
    asset_id,         // Asset ID in poscan-assets
    amount            // Amount to withdraw
)
```

#### Security
- **Origin validation**: Only the owner of the EVM address can withdraw assets
- **Address mapping integrity**: `EnsureAddressTruncated` ensures legitimate ownership
- **Balance validation**: Transfer fails if insufficient balance (prevents overdrafts)
- **Minimum balance enforcement**: Destination account must meet asset's minimum balance requirement
- **Atomic execution**: All operations within the same runtime transaction