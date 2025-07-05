# Identity Precompile

The Identity Precompile provides an interface for managing on-chain identities on the 3Dpass network. It allows users to set, query, and manage identity information including display names, legal names, web presence, social media, and more.

## Address
```
0x0000000000000000000000000000000000000904
```

## Data Structures

### Data
```solidity
struct Data {
    bool hasData;  // Whether the data field has a value
    bytes value;   // The actual data value
}
```

### Additional
```solidity
struct Additional {
    Data key;   // Additional field key
    Data value; // Additional field value
}
```

### IdentityInfo
```solidity
struct IdentityInfo {
    Additional[] additional;  // Additional identity fields
    Data display;            // Display name
    Data legal;              // Legal name
    Data web;                // Website URL
    Data riot;               // Riot/Matrix handle
    Data email;              // Email address
    bool hasPgpFingerprint;  // Whether PGP fingerprint is set
    bytes pgpFingerprint;    // PGP fingerprint
    Data image;              // Profile image hash
    Data twitter;            // Twitter handle
}
```

## Functions

### View Functions

#### `identity(address account)`
Retrieves complete identity information for a given account.

**Parameters:**
- `address account` - Account address

**Returns:**
- `(bool, (uint32, (bool, bool, uint256, bool, bool, bool, bool, bool))[], uint256, IdentityInfo)` - Identity information
  - `bool` - `isValid` - Whether the identity exists
  - `(uint32, (bool, bool, uint256, bool, bool, bool, bool, bool))[]` - `JudgementInfo[]` - Array of judgement information
  - `uint256` - `deposit` - Identity deposit amount
  - `IdentityInfo` - Complete identity information

**Example:**
```solidity
// Get identity for an account
(bool isValid, , uint256 deposit, IdentityInfo memory info) = identity.identity(accountAddress);
if (isValid) {
    console.log("Identity deposit:", deposit);
    console.log("Display name:", string(info.display.value));
    console.log("Email:", string(info.email.value));
}
```

#### `superOf(address who)`
Returns the super-identity information for an account.

**Parameters:**
- `address who` - Account address

**Returns:**
- `(bool, bytes, Data)` - Super identity information
  - `bool` - `isValid` - Whether super identity exists
  - `bytes` - `account` - Super account (AccountId32 as bytes)
  - `Data` - `data` - Super identity data

**Example:**
```solidity
(bool isValid, bytes memory superAccount, Data memory data) = identity.superOf(accountAddress);
if (isValid) {
    console.log("Super account:", superAccount);
    console.log("Super data:", string(data.value));
}
```

#### `subsOf(address who)`
Returns all sub-identities for an account.

**Parameters:**
- `address who` - Account address

**Returns:**
- `(uint256, bytes[])` - Sub identities information
  - `uint256` - `deposit` - Sub identities deposit
  - `bytes[]` - `accounts` - Array of sub account addresses (AccountId32 as bytes)

**Example:**
```solidity
(uint256 deposit, bytes[] memory subAccounts) = identity.subsOf(accountAddress);
console.log("Sub identities deposit:", deposit);
console.log("Number of sub identities:", subAccounts.length);
for (uint i = 0; i < subAccounts.length; i++) {
    console.log("Sub account", i, ":", subAccounts[i]);
}
```

#### `registrars()`
Returns all available registrars.

**Returns:**
- `(bool, uint32, bytes, uint256, (bool, bool, bool, bool, bool, bool, bool, bool))[]` - Array of registrar information
  - `bool` - `isValid` - Whether registrar is valid
  - `uint32` - `index` - Registrar index
  - `bytes` - `account` - Registrar account (AccountId32 as bytes)
  - `uint256` - `fee` - Registrar fee
  - `(bool, bool, bool, bool, bool, bool, bool, bool)` - `IdentityFields` - Fields the registrar can provide

**Example:**
```solidity
(bool[] memory isValid, uint32[] memory indices, bytes[] memory accounts, uint256[] memory fees, ) = identity.registrars();
for (uint i = 0; i < isValid.length; i++) {
    if (isValid[i]) {
        console.log("Registrar", indices[i], "fee:", fees[i]);
    }
}
```

#### `suspendedRegistrars()`
Returns all suspended registrars.

**Returns:**
- `uint32[]` - Array of suspended registrar indices

**Example:**
```solidity
uint32[] memory suspended = identity.suspendedRegistrars();
console.log("Suspended registrars:", suspended.length);
```

### State-Changing Functions

#### `setIdentity(IdentityInfo info)`
Sets the identity information for the caller.

**Parameters:**
- `IdentityInfo info` - Complete identity information

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Create identity information
Additional[] memory additional = new Additional[](1);
additional[0] = Additional(
    Data(true, "github"), 
    Data(true, "john_doe")
);

IdentityInfo memory info = IdentityInfo({
    additional: additional,
    display: Data(true, "John Doe"),
    legal: Data(true, "John Smith"),
    web: Data(true, "https://johndoe.com"),
    riot: Data(false, ""),
    email: Data(true, "john@example.com"),
    hasPgpFingerprint: true,
    pgpFingerprint: hex"1234567890abcdef",
    image: Data(false, ""),
    twitter: Data(true, "@johndoe")
});

bool success = identity.setIdentity(info);
```

#### `setSubs((bytes32, Data)[] subs)`
Sets sub-identities for the caller.

**Parameters:**
- `(bytes32, Data)[] subs` - Array of sub-identity pairs (account, data)

**Returns:**
- `bool` - Success status

**Example:**
```solidity
// Set sub-identities
(bytes32, Data)[] memory subs = new (bytes32, Data)[](2);
subs[0] = (bytes32(0x1234...), Data(true, "Sub account 1"));
subs[1] = (bytes32(0x5678...), Data(true, "Sub account 2"));

bool success = identity.setSubs(subs);
```

#### `clearIdentity()`
Clears the caller's identity information.

**Returns:**
- `bool` - Success status

**Example:**
```solidity
bool success = identity.clearIdentity();
```

#### `requestJudgement(uint32 reg_index, uint256 max_fee)`
Requests judgement from a registrar.

**Parameters:**
- `uint32 reg_index` - Registrar index
- `uint256 max_fee` - Maximum fee willing to pay

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
uint256 maxFee = 100 * 10**18; // 100 tokens
bool success = identity.requestJudgement(registrarIndex, maxFee);
```

#### `cancelRequest(uint32 reg_index)`
Cancels a pending judgement request.

**Parameters:**
- `uint32 reg_index` - Registrar index

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
bool success = identity.cancelRequest(registrarIndex);
```

#### `setFee(uint32 reg_index, uint256 fee)`
Sets the fee for a registrar (registrar only).

**Parameters:**
- `uint32 reg_index` - Registrar index
- `uint256 fee` - New fee amount

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
uint256 newFee = 50 * 10**18; // 50 tokens
bool success = identity.setFee(registrarIndex, newFee);
```

#### `setAccountId(uint32 reg_index, bytes32 new_account)`
Sets the account ID for a registrar (registrar only).

**Parameters:**
- `uint32 reg_index` - Registrar index
- `bytes32 new_account` - New account ID

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
bytes32 newAccount = bytes32(0x1234...);
bool success = identity.setAccountId(registrarIndex, newAccount);
```

#### `setFields(uint32 reg_index, (bool, bool, bool, bool, bool, bool, bool, bool) fields)`
Sets the fields a registrar can provide (registrar only).

**Parameters:**
- `uint32 reg_index` - Registrar index
- `(bool, bool, bool, bool, bool, bool, bool, bool) fields` - Identity fields

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
(bool, bool, bool, bool, bool, bool, bool, bool) memory fields = 
    (true, true, false, false, true, true, false, false);
bool success = identity.setFields(registrarIndex, fields);
```

#### `provideJudgement(uint32 reg_index, bytes32 target, (bool, bool, uint256, bool, bool, bool, bool, bool) judgement, bytes32 identity)`
Provides judgement for an identity (registrar only).

**Parameters:**
- `uint32 reg_index` - Registrar index
- `bytes32 target` - Target account
- `(bool, bool, uint256, bool, bool, bool, bool, bool) judgement` - Judgement information
- `bytes32 identity` - Identity hash

**Returns:**
- `bool` - Success status

**Example:**
```solidity
uint32 registrarIndex = 0;
bytes32 target = bytes32(0x1234...);
(bool, bool, uint256, bool, bool, bool, bool, bool) memory judgement = 
    (true, true, 100 * 10**18, false, false, false, false, false);
bytes32 identityHash = bytes32(0x5678...);

bool success = identity.provideJudgement(registrarIndex, target, judgement, identityHash);
```

#### `addSub(bytes32 sub, Data data)`
Adds a sub-identity.

**Parameters:**
- `bytes32 sub` - Sub account
- `Data data` - Sub identity data

**Returns:**
- `bool` - Success status

**Example:**
```solidity
bytes32 subAccount = bytes32(0x1234...);
Data memory data = Data(true, "Sub account data");
bool success = identity.addSub(subAccount, data);
```

#### `renameSub(bytes32 sub, Data data)`
Renames a sub-identity.

**Parameters:**
- `bytes32 sub` - Sub account
- `Data data` - New sub identity data

**Returns:**
- `bool` - Success status

**Example:**
```solidity
bytes32 subAccount = bytes32(0x1234...);
Data memory data = Data(true, "New sub account name");
bool success = identity.renameSub(subAccount, data);
```

#### `removeSub(bytes32 sub)`
Removes a sub-identity.

**Parameters:**
- `bytes32 sub` - Sub account

**Returns:**
- `bool` - Success status

**Example:**
```solidity
bytes32 subAccount = bytes32(0x1234...);
bool success = identity.removeSub(subAccount);
```

#### `quitSub()`
Quits being a sub-identity.

**Returns:**
- `bool` - Success status

**Example:**
```solidity
bool success = identity.quitSub();
```

## Identity Fields

The identity fields tuple represents which fields a registrar can provide:

| Index | Field | Description |
|-------|-------|-------------|
| 0 | Display | Display name |
| 1 | Legal | Legal name |
| 2 | Web | Website URL |
| 3 | Riot | Riot/Matrix handle |
| 4 | Email | Email address |
| 5 | PGP Fingerprint | PGP fingerprint |
| 6 | Image | Profile image |
| 7 | Twitter | Twitter handle |

## Judgement Values

| Value | Judgement | Description |
|-------|-----------|-------------|
| 0 | Unknown | No judgement provided |
| 1 | FeePaid | Fee paid, judgement pending |
| 2 | Reasonable | Reasonable judgement |
| 3 | KnownGood | Known good judgement |
| 4 | OutOfDate | Out of date judgement |
| 5 | LowQuality | Low quality judgement |
| 6 | Erroneous | Erroneous judgement |

## Usage Examples

### Complete Identity Setup
```solidity
// 1. Set basic identity
Additional[] memory additional = new Additional[](2);
additional[0] = Additional(Data(true, "github"), Data(true, "alice_dev"));
additional[1] = Additional(Data(true, "linkedin"), Data(true, "alice-smith"));

IdentityInfo memory info = IdentityInfo({
    additional: additional,
    display: Data(true, "Alice Smith"),
    legal: Data(true, "Alice Johnson Smith"),
    web: Data(true, "https://alice.dev"),
    riot: Data(true, "@alice:matrix.org"),
    email: Data(true, "alice@example.com"),
    hasPgpFingerprint: true,
    pgpFingerprint: hex"abcdef1234567890",
    image: Data(true, "QmHash..."),
    twitter: Data(true, "@alice_dev")
});

identity.setIdentity(info);

// 2. Request judgement from registrar
identity.requestJudgement(0, 100 * 10**18);
```

### Managing Sub-Identities
```solidity
// 1. Add sub-identities
identity.addSub(bytes32(0x1234...), Data(true, "Personal account"));
identity.addSub(bytes32(0x5678...), Data(true, "Business account"));

// 2. Query sub-identities
(uint256 deposit, bytes[] memory subs) = identity.subsOf(msg.sender);
console.log("Sub identities deposit:", deposit);
console.log("Number of sub identities:", subs.length);

// 3. Remove a sub-identity
identity.removeSub(bytes32(0x1234...));
```

### Registrar Operations
```solidity
// 1. Set registrar fee
identity.setFee(0, 50 * 10**18);

// 2. Set registrar fields
(bool, bool, bool, bool, bool, bool, bool, bool) memory fields = 
    (true, true, true, false, true, true, false, true);
identity.setFields(0, fields);

// 3. Provide judgement
(bool, bool, uint256, bool, bool, bool, bool, bool) memory judgement = 
    (true, true, 100 * 10**18, false, false, false, false, false);
identity.provideJudgement(0, targetAccount, judgement, identityHash);
```

### Querying Identity Information
```solidity
// Get complete identity information
(bool isValid, , uint256 deposit, IdentityInfo memory info) = identity.identity(accountAddress);

if (isValid) {
    console.log("Identity deposit:", deposit);
    console.log("Display name:", string(info.display.value));
    console.log("Email:", string(info.email.value));
    console.log("Website:", string(info.web.value));
    console.log("Twitter:", string(info.twitter.value));
    
    // Check additional fields
    for (uint i = 0; i < info.additional.length; i++) {
        if (info.additional[i].key.hasData && info.additional[i].value.hasData) {
            console.log("Additional field:", string(info.additional[i].key.value), "=", string(info.additional[i].value.value));
        }
    }
}
```

## Error Handling

The precompile will revert with descriptive error messages for:
- Invalid registrar indices
- Insufficient funds for fees
- Unauthorized operations
- Invalid judgement values
- Overflow/underflow conditions
- Invalid account formats

## Gas Considerations

- Setting identity: ~150,000 gas
- Adding sub-identity: ~80,000 gas
- Requesting judgement: ~60,000 gas
- Providing judgement: ~100,000 gas
- Identity queries: ~40,000 gas

## Security Notes

- Always verify registrar information before requesting judgements
- Check fee requirements before operations
- Validate identity data before submission
- Use appropriate judgement values
- Consider deposit requirements for identity operations 