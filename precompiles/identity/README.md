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

**Events:**
- Emits `IdentitySet(address indexed who)` event

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

**Events:**
- Emits `IdentityCleared(address indexed who)` event

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

**Events:**
- Emits `JudgementRequested(address indexed who, uint32 regIndex)` event

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

**Events:**
- Emits `JudgementUnrequested(address indexed who, uint32 regIndex)` event

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

**Events:**
- Emits `JudgementGiven(address indexed target, uint32 regIndex)` event

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

**Events:**
- Emits `SubIdentityAdded(address indexed sub, address indexed main)` event

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

**Events:**
- Emits `SubIdentityRemoved(address indexed sub, address indexed main)` event

**Example:**
```solidity
bytes32 subAccount = bytes32(0x1234...);
bool success = identity.removeSub(subAccount);
```

#### `quitSub()`
Quits being a sub-identity.

**Returns:**
- `bool` - Success status

**Events:**
- Emits `SubIdentityRevoked(address indexed sub)` event

**Example:**
```solidity
bool success = identity.quitSub();
```

## Events

The Identity precompile emits the following events to provide transparency and enable event monitoring:

### `IdentitySet(address indexed who)`
Emitted when an identity is set or reset (which will remove all judgements).

**Parameters:**
- `address indexed who` - Address of the target account

**Example:**
```solidity
// Listen for identity set events
event IdentitySet(address indexed who);

function onIdentitySet(address who) external {
    console.log("Identity set for:", who);
}
```

### `IdentityCleared(address indexed who)`
Emitted when an identity is cleared, and the given balance returned.

**Parameters:**
- `address indexed who` - Address of the target account

**Example:**
```solidity
// Listen for identity cleared events
event IdentityCleared(address indexed who);

function onIdentityCleared(address who) external {
    console.log("Identity cleared for:", who);
}
```

### `JudgementRequested(address indexed who, uint32 regIndex)`
Emitted when a judgement is asked from a registrar.

**Parameters:**
- `address indexed who` - Address of the requesting account
- `uint32 regIndex` - The registrar's index

**Example:**
```solidity
// Listen for judgement request events
event JudgementRequested(address indexed who, uint32 regIndex);

function onJudgementRequested(address who, uint32 regIndex) external {
    console.log("Judgement requested by:", who, "from registrar:", regIndex);
}
```

### `JudgementUnrequested(address indexed who, uint32 regIndex)`
Emitted when a judgement request is retracted.

**Parameters:**
- `address indexed who` - Address of the target account
- `uint32 regIndex` - The registrar's index

**Example:**
```solidity
// Listen for judgement unrequest events
event JudgementUnrequested(address indexed who, uint32 regIndex);

function onJudgementUnrequested(address who, uint32 regIndex) external {
    console.log("Judgement unrequested by:", who, "from registrar:", regIndex);
}
```

### `JudgementGiven(address indexed target, uint32 regIndex)`
Emitted when a judgement is given by a registrar.

**Parameters:**
- `address indexed target` - Address of the target account
- `uint32 regIndex` - The registrar's index

**Example:**
```solidity
// Listen for judgement given events
event JudgementGiven(address indexed target, uint32 regIndex);

function onJudgementGiven(address target, uint32 regIndex) external {
    console.log("Judgement given to:", target, "by registrar:", regIndex);
}
```

### `SubIdentityAdded(address indexed sub, address indexed main)`
Emitted when a sub-identity is added to an identity and the deposit paid.

**Parameters:**
- `address indexed sub` - Address of the sub account
- `address indexed main` - Address of the main account

**Example:**
```solidity
// Listen for sub-identity added events
event SubIdentityAdded(address indexed sub, address indexed main);

function onSubIdentityAdded(address sub, address main) external {
    console.log("Sub-identity added:", sub, "to main:", main);
}
```

### `SubIdentityRemoved(address indexed sub, address indexed main)`
Emitted when a sub-identity is removed from an identity and the deposit freed.

**Parameters:**
- `address indexed sub` - Address of the sub account
- `address indexed main` - Address of the main account

**Example:**
```solidity
// Listen for sub-identity removed events
event SubIdentityRemoved(address indexed sub, address indexed main);

function onSubIdentityRemoved(address sub, address main) external {
    console.log("Sub-identity removed:", sub, "from main:", main);
}
```

### `SubIdentityRevoked(address indexed sub)`
Emitted when a sub-identity is cleared and the given deposit repatriated from the main identity account to the sub-identity account.

**Parameters:**
- `address indexed sub` - Address of the sub account

**Example:**
```solidity
// Listen for sub-identity revoked events
event SubIdentityRevoked(address indexed sub);

function onSubIdentityRevoked(address sub) external {
    console.log("Sub-identity revoked:", sub);
}
```

### Event Monitoring Examples

#### Solidity Event Monitoring
```solidity
contract IdentityMonitor {
    Identity public identity;
    
    constructor(address identityAddress) {
        identity = Identity(identityAddress);
    }
    
    // Monitor all identity events
    function monitorIdentityEvents() external {
        // Listen for identity set/cleared events
        identity.IdentitySet(msg.sender);
        identity.IdentityCleared(msg.sender);
        
        // Listen for judgement events
        identity.JudgementRequested(msg.sender, 0);
        identity.JudgementUnrequested(msg.sender, 0);
        identity.JudgementGiven(msg.sender, 0);
        
        // Listen for sub-identity events
        identity.SubIdentityAdded(msg.sender, msg.sender);
        identity.SubIdentityRemoved(msg.sender, msg.sender);
        identity.SubIdentityRevoked(msg.sender);
    }
}
```

#### JavaScript/TypeScript Event Monitoring
```javascript
// Using ethers.js
const identityContract = new ethers.Contract(identityAddress, identityABI, provider);

// Listen for identity set events
identityContract.on('IdentitySet', (who, event) => {
    console.log('Identity set for:', who);
    console.log('Transaction hash:', event.transactionHash);
    console.log('Block number:', event.blockNumber);
});

// Listen for judgement requested events
identityContract.on('JudgementRequested', (who, regIndex, event) => {
    console.log('Judgement requested by:', who, 'from registrar:', regIndex.toString());
    console.log('Transaction hash:', event.transactionHash);
});

// Listen for sub-identity added events
identityContract.on('SubIdentityAdded', (sub, main, event) => {
    console.log('Sub-identity added:', sub, 'to main:', main);
    console.log('Transaction hash:', event.transactionHash);
});

// Get past events
async function getPastEvents() {
    const fromBlock = 1000000;
    const toBlock = 'latest';
    
    const identitySetEvents = await identityContract.queryFilter(
        identityContract.filters.IdentitySet(),
        fromBlock,
        toBlock
    );
    
    console.log('Past identity set events:', identitySetEvents.length);
    
    for (const event of identitySetEvents) {
        console.log('Identity set for:', event.args.who);
        console.log('Block:', event.blockNumber);
    }
}
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