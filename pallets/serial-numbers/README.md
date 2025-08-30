# SerialNumbers Pallet

A Substrate pallet for creating and managing deterministic serial numbers (SNs) with stateless verification capabilities.

## Overview

The SerialNumbers pallet allows users to create unique, deterministic serial numbers that can be verified on-chain or off-chain. Each serial number is generated using a blake2_128 hash of the owner's AccountId, current block hash, and block index, ensuring uniqueness and determinism.

## Features

- **Deterministic Generation**: Serial numbers are generated using blake2_128 hashing for WASM compatibility
- **Stateless Verification**: Verify serial numbers without storing them on-chain
- **Batch Creation**: Support for up to 10,000 unique SNs per block per owner
- **Expiration Management**: Mark serial numbers as expired
- **Usage Tracking**: Prevent double-spending with usage tracking
- **Runtime API**: Full query interface for off-chain applications

## Technical Details

### Serial Number Format
- **Hash Algorithm**: blake2_128
- **Hash Input**: `owner.encode() + block_hash.encode() + block_index.to_le_bytes()`
- **Output**: 16-byte array `[u8; 16]`
- **Uniqueness**: Guaranteed by block hash + block index combination

## Extrinsics

### `create_serial_number(origin, block_index)`
Creates one serial number for the specified block index.

**Parameters:**
- `origin`: Signed origin (AccountId in 32-byte format)
- `block_index`: Index within the block (0 to MaxSerialNumbersPerBlock-1)

**Returns:** `DispatchResult`

**Events:**
- `SerialNumberCreated(sn_index, sn_hash, owner, block_index)`

**Limits:**
- Maximum 10,000 serial numbers per block per owner (configurable)
- Block index must be within allowed range

### `turn_sn_expired(origin, sn_index)`
Marks a serial number as expired.

**Parameters:**
- `origin`: Signed origin (must be the owner)
- `sn_index`: Index of the serial number to expire

**Returns:** `DispatchResult`

**Events:**
- `SerialNumberExpired(sn_index, sn_hash)`

### `use_serial_number(origin, sn_hash)`
Marks a serial number as used (prevents double-spending).

**Parameters:**
- `origin`: Signed origin
- `sn_hash`: 16-byte hash of the serial number

**Returns:** `DispatchResult`

**Events:**
- `SerialNumberUsed(sn_hash, user)`

## Runtime API

### `verify_serial_number(sn_hash, block)`
Verify if a serial number is valid for the given block.

### `verify_serial_number_stateless(sn_hash, owner, block, block_index)`
Verify a serial number without storing it (stateless verification).

### `generate_serial_numbers_for_block(owner, block, count)`
Generate multiple serial numbers for a block (off-chain helper).

### `get_serial_numbers(sn_index)`
Get serial number details (all if sn_index is None).

### `get_sn_owners(owner)`
Get all serial number indices owned by an account.

### `is_serial_number_used(sn_hash)`
Check if a serial number has been used.

### `sn_count()`
Get the total count of serial numbers created.

## Storage

### `SNCount<T>`
Total number of serial numbers created.

### `SerialNumbers<T>`
Map of serial number index to details.

### `SNByHash<T>`
Map of serial number hash to index.

### `SNOwners<T>`
Map of owner to their serial number indices.

### `UsedSerialNumbers<T>`
Map of used serial number hashes.

### `OwnerBlockCount<T>`
Double map of owner and block number to count.

## Events

### `SerialNumberCreated(sn_index, sn_hash, owner, block_index)`
Emitted when a new serial number is created.

### `SerialNumberExpired(sn_index, sn_hash)`
Emitted when a serial number is marked as expired.

### `SerialNumberUsed(sn_hash, user)`
Emitted when a serial number is used.

## Errors

### `SerialNumberNotFound`
Serial number does not exist.

### `SerialNumberExpired`
Serial number has been marked as expired.

### `SerialNumberAlreadyUsed`
Serial number has already been used.

### `NotOwner`
Caller is not the owner of the serial number.

### `TooManySerialNumbersPerBlock`
Owner has exceeded the maximum serial numbers per block.

### `InvalidBlockIndex`
Block index is outside the allowed range.

## Configuration

### `MaxSerialNumbersPerBlock`
Maximum number of serial numbers that can be generated per block per owner.
Default: 10,000

## Usage Examples

### Creating a Serial Number
```rust
// Call the extrinsic
let result = SerialNumbers::create_serial_number(origin, 5);

// Listen for the event to get the hash
// SerialNumberCreated(sn_index, sn_hash, owner, 5)
```

### Off-chain Hash Generation
```rust
// Generate the same hash off-chain
let block_hash = get_block_hash(block_number);
let sn_hash = SerialNumbers::generate_serial_number(&owner, &block_hash, block_index);
```

### Verifying a Serial Number
```rust
// On-chain verification
let is_valid = SerialNumbers::verify_serial_number(sn_hash, block_number);

// Stateless verification
let is_valid = SerialNumbers::verify_serial_number_stateless(
    sn_hash, &owner, block_number, block_index
);
```

### Getting User's Serial Numbers
```rust
// Get all serial numbers owned by an account
let my_sns = SerialNumbers::get_sn_owners(owner);

// Get details of a specific serial number
let details = SerialNumbers::get_serial_numbers(Some(sn_index));
```

## Integration

The pallet is integrated into the runtime with:
- Full runtime API implementation
- Event emission for all state changes
- Proper error handling and validation
- Gas-efficient storage design

## Security Considerations

- Serial numbers are deterministic and can be computed off-chain
- Double-spending is prevented by usage tracking
- Expiration mechanism allows for time-limited serial numbers
- Block index limits prevent spam attacks
- Owner-only operations for sensitive functions
