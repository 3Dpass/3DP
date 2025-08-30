# PoScan Precompile

The PoScan Precompile provides a Solidity interface for authentication of objects as well as digital assets management within the The Ledger of Things network. It allows users to submit, manage, and query 3D objects, drawings, music, biometrics, movements, and texts.

## Address
```
0x0000000000000000000000000000000000000903
```

## Data Structures

### Property
```solidity
struct Property {
    uint32 propIdx;
    uint128 maxValue;
}
```

### Permission
```solidity
struct Permission {
    address who;
    uint32 maxCopies;
    uint64 until;
}
```

### PropertyInfo
```solidity
struct PropertyInfo {
    bool isValid;
    uint32 propIdx;
    string name;
    uint8 class; // 0=Relative, 1=Absolute
    uint128 maxValue;
}
```

### ObjectInfo
```solidity
struct ObjectInfo {
    bool isValid;
    uint8 state; // 0=Created, 1=Estimating, 2=Estimated, 3=NotApproved, 4=Approved, 5=QCInspecting, 6=QCPassed, 7=QCRejected, 8=SelfProved
    uint64 stateBlock;
    uint8 category; // 0=Objects3D, 1=Drawings2D, 2=Music, 3=Biometrics, 4=Movements, 5=Texts
    uint64 whenCreated;
    uint64 whenApproved;
    bool isPrivate;
    bytes32[] hashes;
    uint8 numApprovals;
    Property[] prop;
    bool isReplica;
    uint32 originalObj;
    uint64 snHash;
    bool isOwnershipAbdicated;
    bool isSelfProved;
    bytes32 proofOfExistence;
    string ipfsLink;
    uint32 qcTimeout;
}
```

## Events

The PoScan precompile emits the following events:

### ObjectSubmitted
```solidity
event ObjectSubmitted(address indexed submitter, uint32 indexed objIdx);
```

### PermissionsSet
```solidity
event PermissionsSet(uint32 indexed objIdx, address indexed setter);
```

### ObjectOwnershipTransferred
```solidity
event ObjectOwnershipTransferred(uint32 indexed objIdx, address indexed oldOwner, address indexed newOwner);
```

### QCInspecting
```solidity
event QCInspecting(uint32 indexed objIdx, address indexed inspector);
```

### QCPassed
```solidity
event QCPassed(uint32 indexed objIdx, address indexed inspector);
```

### QCRejected
```solidity
event QCRejected(uint32 indexed objIdx, address indexed inspector);
```

### InspectorFeePaid
```solidity
event InspectorFeePaid(uint32 indexed objIdx, address indexed inspector, uint256 fee);
```

### QCInspectionTimeout
```solidity
event QCInspectionTimeout(uint32 indexed objIdx, address indexed feePayer, uint256 fee);
```

### UnspentRewardsUnlocked
```solidity
event UnspentRewardsUnlocked(uint32 indexed objIdx, address indexed feePayer, uint256 amount);
```

## Functions

### View Functions

#### `getObject(uint32 objIdx)`
Returns detailed information about a specific object.

Parameters:
- `uint32 objIdx`: The index of the object to retrieve.

Returns:
- `ObjectInfo`: Detailed information about the object.

#### `getObjectsOf(address owner)`
Returns all object indices owned by a specific address.

Parameters:
- `address owner`: The address to query.

Returns:
- `uint32[]`: Array of object indices owned by the address.

#### `getObjectsOfInspector(address inspector)`
Returns all object indices assigned to an inspector.

Parameters:
- `address inspector`: The address of the inspector to query.

Returns:
- `uint32[]`: Array of object indices assigned to the inspector.

#### `getObjectCount()`
Returns the total number of objects in the system.

Returns:
- `uint32`: The total number of objects.

#### `getProperties()`
Returns all available properties for object tokenization.

Returns:
- `PropertyInfo[]`: Array of available properties.

#### `getFeePerByte()`
Returns the storage fee per byte.

Returns:
- `uint256`: The storage fee per byte.

#### `getAccountLock(address owner)`
Returns the account lock value for a given address.

Parameters:
- `address owner`: The address to query.

Returns:
- `uint256`: The account lock value.

#### `getRewards()`
Returns the council-controlled rewards.

Returns:
- `uint256`: The total rewards.

#### `getDynamicRewardsGrowthRate()`
Returns the dynamic rewards growth rate.

Returns:
- `uint256`: The dynamic rewards growth rate.

#### `getAuthorPart()`
Returns the author share percentage.

Returns:
- `uint256`: The author share percentage.

#### `getUnspentRewards(uint32 objIdx)`
Returns the unspent rewards for a NotApproved object.

Parameters:
- `uint32 objIdx`: The index of the NotApproved object.

Returns:
- `uint256`: The unspent rewards amount.

#### `getPendingStorageFees()`
Returns the pending storage fees to be distributed to validators.

Returns:
- `uint256`: The total pending storage fees.

#### `getQCTimeout(uint32 objIdx)`
Returns the QC timeout for an object.

Parameters:
- `uint32 objIdx`: The index of the object.

Returns:
- `uint32`: The QC timeout in seconds.

#### `getObjectIdxByProofOfExistence(bytes32 proofOfExistence)`
Returns the object index by proof of existence hash.

Parameters:
- `bytes32 proofOfExistence`: The hash of the proof of existence.

Returns:
- `uint32`: The object index.

#### `getReplicasOf(uint32 originalObj)`
Returns all replica indices for a given original object.

Parameters:
- `uint32 originalObj`: The index of the original object.

Returns:
- `uint32[]`: Array of replica indices.

### State-Changing Functions

#### `putObject(uint8 category, uint8 algo3d, bool isPrivate, bytes calldata obj, uint8 numApprovals, bytes32[] calldata hashes, Property[] calldata properties, bool isReplica, uint32 originalObj, uint64 snHash, bool isSelfProved, bytes32 proofOfExistence, string calldata ipfsLink)`
Submits a new object to the PoScan pallet. Replicas must be submitted as self-proved objects.

Parameters:
- `uint8 category`: The category of the object (0=Objects3D, 1=Drawings2D, 2=Music, 3=Biometrics, 4=Movements, 5=Texts).
- `uint8 algo3d`: The Algo3D variant (0=Grid2dLow, 1=Grid2dHigh).
- `bool isPrivate`: Whether the object is private.
- `bytes calldata obj`: The raw object data.
- `uint8 numApprovals`: The required number of approvals.
- `bytes32[] calldata hashes`: Array of hashes of the object data.
- `Property[] calldata properties`: Array of properties for the object.
- `bool isReplica`: Whether the object is a replica.
- `uint32 originalObj`: The index of the original object (if it's a replica).
- `uint64 snHash`: The hash of the object data.
- `bool isSelfProved`: Whether the object is self-proved.
- `bytes32 proofOfExistence`: The hash of the proof of existence.
- `string calldata ipfsLink`: The IPFS link for the object data.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `inspectPutObject(uint8 category, uint8 algo3d, bool isPrivate, bytes calldata obj, uint8 numApprovals, bytes32[] calldata hashes, Property[] calldata properties, bool isReplica, uint32 originalObj, uint64 snHash, address inspector, uint256 inspectorFee, uint32 qcTimeout, bool isSelfProved, bytes32 proofOfExistence, string calldata ipfsLink)`
Submits a new object for QC inspection. Replicas must be submitted as self-proved objects.

Parameters:
- `uint8 category`: The category of the object (0=Objects3D, 1=Drawings2D, 2=Music, 3=Biometrics, 4=Movements, 5=Texts).
- `uint8 algo3d`: The Algo3D variant (0=Grid2dLow, 1=Grid2dHigh).
- `bool isPrivate`: Whether the object is private.
- `bytes calldata obj`: The raw object data.
- `uint8 numApprovals`: The required number of approvals.
- `bytes32[] calldata hashes`: Array of hashes of the object data.
- `Property[] calldata properties`: Array of properties for the object.
- `bool isReplica`: Whether the object is a replica.
- `uint32 originalObj`: The index of the original object (if it's a replica).
- `uint64 snHash`: The hash of the object data.
- `address inspector`: The address of the inspector.
- `uint256 inspectorFee`: The fee paid by the inspector.
- `uint32 qcTimeout`: The QC timeout in seconds.
- `bool isSelfProved`: Whether the object is self-proved.
- `bytes32 proofOfExistence`: The hash of the proof of existence.
- `string calldata ipfsLink`: The IPFS link for the object data.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `setPrivateObjectPermissions(uint32 objIdx, Permission[] calldata permissions)`
Sets permissions for private object replicas.

Parameters:
- `uint32 objIdx`: The index of the private object.
- `Permission[] calldata permissions`: Array of permissions to set.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `qcApprove(uint32 objIdx)`
Inspector approves the object (QC passed).

Parameters:
- `uint32 objIdx`: The index of the object.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `qcReject(uint32 objIdx)`
Inspector rejects the object (QC rejected).

Parameters:
- `uint32 objIdx`: The index of the object.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `transferObjectOwnership(uint32 objIdx, address newOwner)`
Transfers object ownership to another account.

Parameters:
- `uint32 objIdx`: The index of the object.
- `address newOwner`: The new owner address.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `abdicateTheObjOwnership(uint32 objIdx)`
Abdicates object ownership (irreversible).

Parameters:
- `uint32 objIdx`: The index of the object.

Returns:
- `bool`: `true` on success, `false` on failure.

#### `unlockUnspentRewards(uint32 objIdx)`
Unlocks unspent rewards for a NotApproved object (fee payer only).

Parameters:
- `uint32 objIdx`: The index of the NotApproved object.

Returns:
- `bool`: `true` on success, `false` on failure.

## Object Categories

| Value | Category    | Description                  |
|-------|-------------|------------------------------|
| 0     | Objects3D   | 3D objects with Algo3D       |
| 1     | Drawings2D  | 2D drawings                  |
| 2     | Music       | Musical compositions         |
| 3     | Biometrics  | Biometric data               |
| 4     | Movements   | Movement patterns            |
| 5     | Texts       | Text documents               |

## Object States

| Value | State         | Description                  |
|-------|--------------|------------------------------|
| 0     | Created      | Object has been created      |
| 1     | Estimating   | Object is being estimated    |
| 2     | Estimated    | Object has been estimated    |
| 3     | NotApproved  | Object was not approved      |
| 4     | Approved     | Object has been approved     |
| 5     | QCInspecting | Object is under QC inspection|
| 6     | QCPassed     | Object passed QC             |
| 7     | QCRejected   | Object rejected by QC        |
| 8     | SelfProved   | Object is self-proved        |

## Algo3D Variants

| Value | Variant      | Description                  |
|-------|-------------|------------------------------|
| 0     | Grid2dLow   | Low-resolution 2D grid       |
| 1     | Grid2dHigh  | High-resolution 2D grid      |

## Usage Examples

### Submit a 3D object
```solidity
uint8 category = 0; // Objects3D
uint8 algo3d = 0;   // Grid2dLow
bool isPrivate = false;
bytes memory objFile = hex"7665727469636573202e2e2e"; // OBJ file content
uint8 numApprovals = 3;
bytes32[] memory hashes = new bytes32[](1);
hashes[0] = keccak256(objFile);
Property[] memory properties = new Property[](1);
properties[0] = Property(0, 1000); // Property 0 with max value 1000

bool success = poscan.putObject(
    category, algo3d, isPrivate, objFile, numApprovals, 
    hashes, properties, false, 0, 0, false, bytes32(0), ""
);
```

### Submit a 3D object for QC inspection
```solidity
bool success = poscan.inspectPutObject(
    0, 0, false, objFile, 1, hashes, properties, false, 0, 0, inspector, 1000000000000000000, 100, false, bytes32(0), ""
);
```

### Set permissions for private object
```solidity
uint32 objIdx = 1;
Permission[] memory permissions = new Permission[](1);
permissions[0] = Permission(
    0x1234567890123456789012345678901234567890, // Address
    5,  // Max copies
    1640995200  // Until timestamp (2022-01-01)
);

bool success = poscan.setPrivateObjectPermissions(objIdx, permissions);
```

### Transfer object ownership
```solidity
bool success = poscan.transferObjectOwnership(1, 0x1234567890123456789012345678901234567890);
```

### Abdicate object ownership
```solidity
bool success = poscan.abdicateTheObjOwnership(1);
```

### Unlock unspent rewards
```solidity
bool success = poscan.unlockUnspentRewards(1);
```

### Querying Object Information
```solidity
uint32[] memory myObjects = poscan.getObjectsOf(msg.sender);
for (uint i = 0; i < myObjects.length; i++) {
    ObjectInfo memory obj = poscan.getObject(myObjects[i]);
    if (obj.isValid) {
        console.log("Object", myObjects[i], "state:", obj.state);
        console.log("Category:", obj.category);
        console.log("Is private:", obj.isPrivate);
        console.log("Required approvals:", obj.numApprovals);
    }
}
```

## Event Monitoring

### Listen for ObjectSubmitted events
```javascript
poscanContract.on('ObjectSubmitted', (submitter, objIdx, event) => {
    console.log('New object submitted:', {
        submitter: submitter,
        objectIndex: objIdx.toString(),
        blockNumber: event.blockNumber,
        transactionHash: event.transactionHash
    });
});
```

### Listen for PermissionsSet events
```javascript
poscanContract.on('PermissionsSet', (objIdx, setter, event) => {
    console.log('Permissions updated:', {
        objectIndex: objIdx.toString(),
        setter: setter,
        blockNumber: event.blockNumber,
        transactionHash: event.transactionHash
    });
});
```

### Listen for QC and Ownership events
```javascript
poscanContract.on('QCInspecting', (objIdx, inspector, event) => { ... });
poscanContract.on('QCPassed', (objIdx, inspector, event) => { ... });
poscanContract.on('QCRejected', (objIdx, inspector, event) => { ... });
poscanContract.on('InspectorFeePaid', (objIdx, inspector, fee, event) => { ... });
poscanContract.on('ObjectOwnershipTransferred', (objIdx, oldOwner, newOwner, event) => { ... });
poscanContract.on('UnspentRewardsUnlocked', (objIdx, feePayer, amount, event) => { ... });
```

## Error Handling

The precompile will revert with descriptive error messages for:
- Invalid object categories or Algo3D variants
- Object file too large (>1MB)
- Invalid OBJ file format
- Insufficient permissions
- Invalid property values
- Overflow/underflow conditions

## Gas Considerations

- Object submission: ~200,000 gas (varies with file size)
- Permission setting: ~50,000 gas
- Object queries: ~30,000 gas
- Property queries: ~20,000 gas
- QC inspection: ~250,000 gas
- Ownership transfer: ~40,000 gas

## Security Notes

- Always validate OBJ file format before submission
- Check object ownership before setting permissions or transferring ownership
- Verify property values are within acceptable ranges
- Use appropriate approval requirements for sensitive objects
- Consider gas costs for large object files 