# PoScan Precompile

The PoScan Precompile provides an interface for managing 3D objects and digital assets on the 3Dpass network. It allows users to submit, manage, and query 3D objects, drawings, music, biometrics, movements, and texts.

## Address
```
0x0000000000000000000000000000000000000903
```

## Data Structures

### Property
```solidity
struct Property {
    uint32 propIdx;    // Property index
    uint128 maxValue;  // Maximum value for the property
}
```

### Permission
```solidity
struct Permission {
    address who;       // Address with permission
    uint32 maxCopies;  // Maximum copies allowed
    uint64 until;      // Permission expiry timestamp
}
```

### PropertyInfo
```solidity
struct PropertyInfo {
    bool isValid;      // Whether the property exists
    uint32 propIdx;    // Property index
    string name;       // Property name
    uint8 class;       // 0=Relative, 1=Absolute
    uint128 maxValue;  // Maximum value
}
```

### ObjectInfo
```solidity
struct ObjectInfo {
    bool isValid;           // Whether the object exists
    uint8 state;            // Object state
    uint64 stateBlock;      // Block when state changed
    uint8 category;         // Object category
    uint64 whenCreated;     // Creation timestamp
    uint64 whenApproved;    // Approval timestamp (0 if not approved)
    bytes32 owner;          // Owner address
    bool isPrivate;         // Whether object is private
    bytes32[] hashes;       // Object hashes
    uint8 numApprovals;     // Number of approvals required
    Property[] prop;        // Object properties
}
```

## Events

The PoScan precompile emits the following events:

### ObjectSubmitted
Emitted when a new object is successfully submitted to the PoScan pallet.

```solidity
event ObjectSubmitted(address indexed submitter, uint32 indexed objIdx);
```

**Parameters:**
- `address indexed submitter` - The address that submitted the object
- `uint32 indexed objIdx` - The index of the submitted object

### PermissionsSet
Emitted when permissions are set for a private object.

```solidity
event PermissionsSet(uint32 indexed objIdx, address indexed setter);
```

**Parameters:**
- `uint32 indexed objIdx` - The object index for which permissions were set
- `address indexed setter` - The address that set the permissions

## Functions

### View Functions

#### `getObject(uint32 objIdx)`
Retrieves detailed information about a specific object.

**Parameters:**
- `uint32 objIdx` - Object index

**Returns:**
- `ObjectInfo memory info` - Complete object information

**Example:**
```solidity
// Get object with index 1
ObjectInfo memory obj = poscan.getObject(1);
if (obj.isValid) {
    console.log("Object owner:", obj.owner);
    console.log("Object state:", obj.state);
    console.log("Is private:", obj.isPrivate);
}
```

#### `getObjectsOf(address owner)`
Returns all object indices owned by a specific address.

**Parameters:**
- `address owner` - Owner address (EVM H160)

**Returns:**
- `uint32[] memory objectIndices` - Array of object indices

**Example:**
```solidity
// Get all objects owned by msg.sender
uint32[] memory myObjects = poscan.getObjectsOf(msg.sender);
for (uint i = 0; i < myObjects.length; i++) {
    console.log("My object index:", myObjects[i]);
}
```

#### `getObjectCount()`
Returns the total number of objects in the system.

**Returns:**
- `uint32 count` - Total number of objects

**Example:**
```solidity
uint32 totalObjects = poscan.getObjectCount();
console.log("Total objects in system:", totalObjects);
```

#### `getProperties()`
Returns all available properties for object tokenization.

**Returns:**
- `PropertyInfo[] memory properties` - Array of property information

**Example:**
```solidity
PropertyInfo[] memory properties = poscan.getProperties();
for (uint i = 0; i < properties.length; i++) {
    if (properties[i].isValid) {
        console.log("Property:", properties[i].name);
        console.log("Class:", properties[i].class);
        console.log("Max value:", properties[i].maxValue);
    }
}
```

#### `getFeePerByte()`
Returns the storage fee per byte.

**Returns:**
- `(bool exists, uint64 fee)` - Whether fee exists and the fee amount

**Example:**
```solidity
(bool exists, uint64 fee) = poscan.getFeePerByte();
if (exists) {
    console.log("Storage fee per byte:", fee);
}
```

#### `getAccountLock(address owner)`
Returns the account lock value for a given address.

**Parameters:**
- `address owner` - Owner address

**Returns:**
- `uint128 lock` - Account lock value

**Example:**
```solidity
uint128 lockValue = poscan.getAccountLock(msg.sender);
console.log("Account lock value:", lockValue);
```

#### `getReplicasOf(uint32 originalObj)`
Returns all replica indices for a given original object.

**Parameters:**
- `uint32 originalObj` - Original object index

**Returns:**
- `uint32[] memory` - Array of replica indices

**Example:**
```solidity
uint32[] memory replicas = poscan.getReplicasOf(1);
console.log("Number of replicas:", replicas.length);
for (uint i = 0; i < replicas.length; i++) {
    console.log("Replica index:", replicas[i]);
}
```

### State-Changing Functions

#### `putObject(uint8 category, uint8 algo3d, bool isPrivate, bytes calldata obj, uint8 numApprovals, bytes32[] calldata hashes, Property[] calldata properties, bool isReplica, uint32 originalObj)`
Submits a new object to the PoScan pallet.

**Parameters:**
- `uint8 category` - Object category (0=Objects3D, 1=Drawings2D, 2=Music, 3=Biometrics, 4=Movements, 5=Texts)
- `uint8 algo3d` - Algo3D variant (only used if category == Objects3D: 0=Grid2dLow, 1=Grid2dHigh)
- `bool isPrivate` - Whether the object is private
- `bytes calldata obj` - The OBJ file bytes (max 1MB)
- `uint8 numApprovals` - Number of approvals required
- `bytes32[] calldata hashes` - Optional array of object hashes (can be empty)
- `Property[] calldata properties` - Array of property values
- `bool isReplica` - Whether this is a replica
- `uint32 originalObj` - Original object index (only used if isReplica is true)

**Returns:**
- `bool` - Success status

**Events:**
- Emits `ObjectSubmitted(address indexed submitter, uint32 indexed objIdx)` on success

**Example:**
```solidity
// Submit a 3D object
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
    hashes, properties, false, 0
);
```

#### `setPrivateObjectPermissions(uint32 objIdx, Permission[] calldata permissions)`
Sets permissions for private object replicas.

**Parameters:**
- `uint32 objIdx` - Object index
- `Permission[] calldata permissions` - Array of permissions

**Returns:**
- `bool` - Success status

**Events:**
- Emits `PermissionsSet(uint32 indexed objIdx, address indexed setter)` on success

**Example:**
```solidity
// Set permissions for private object
uint32 objIdx = 1;
Permission[] memory permissions = new Permission[](1);
permissions[0] = Permission(
    0x1234567890123456789012345678901234567890, // Address
    5,  // Max copies
    1640995200  // Until timestamp (2022-01-01)
);

bool success = poscan.setPrivateObjectPermissions(objIdx, permissions);
```

## Object Categories

| Value | Category | Description |
|-------|----------|-------------|
| 0 | Objects3D | 3D objects with Algo3D variants |
| 1 | Drawings2D | 2D drawings |
| 2 | Music | Musical compositions |
| 3 | Biometrics | Biometric data |
| 4 | Movements | Movement patterns |
| 5 | Texts | Text documents |

## Object States

| Value | State | Description |
|-------|-------|-------------|
| 0 | Created | Object has been created |
| 1 | Estimating | Object is being estimated |
| 2 | Estimated | Object has been estimated |
| 3 | NotApproved | Object was not approved |
| 4 | Approved | Object has been approved |

## Algo3D Variants

| Value | Variant | Description |
|-------|---------|-------------|
| 0 | Grid2dLow | Low-resolution 2D grid |
| 1 | Grid2dHigh | High-resolution 2D grid |

## Usage Examples

### Complete Object Submission Workflow
```solidity
// 1. Check storage fee
(bool feeExists, uint64 feePerByte) = poscan.getFeePerByte();
require(feeExists, "Storage fee not set");

// 2. Calculate fee for object
uint256 objectSize = objFile.length;
uint256 totalFee = objectSize * feePerByte;

// 3. Submit object
bool success = poscan.putObject(
    category, algo3d, isPrivate, objFile, numApprovals, 
    hashes, properties, false, 0
);

// 4. Check object details
ObjectInfo memory obj = poscan.getObject(1);
require(obj.isValid, "Object not found");
```

### Private Object with Permissions
```solidity
// 1. Submit private object
bool success = poscan.putObject(
    0, 0, true, objFile, 1, hashes, properties, false, 0
);

// 2. Set permissions for specific users
Permission[] memory permissions = new Permission[](2);
permissions[0] = Permission(user1, 3, block.timestamp + 30 days);
permissions[1] = Permission(user2, 1, block.timestamp + 7 days);

poscan.setPrivateObjectPermissions(1, permissions);
```

### Querying Object Information
```solidity
// Get all objects owned by user
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

### Listening to ObjectSubmitted Events
```solidity
// Listen for new object submissions
poscan.ObjectSubmitted().watch((address submitter, uint32 objIdx) => {
    console.log("New object submitted by:", submitter);
    console.log("Object index:", objIdx);
});
```

### Listening to PermissionsSet Events
```solidity
// Listen for permission changes
poscan.PermissionsSet().watch((uint32 objIdx, address setter) => {
    console.log("Permissions set for object:", objIdx);
    console.log("Set by:", setter);
});
```

### JavaScript/TypeScript Example
```javascript
// Using ethers.js
const poscanContract = new ethers.Contract(
    '0x0000000000000000000000000000000000000903',
    poscanABI,
    provider
);

// Listen for ObjectSubmitted events
poscanContract.on('ObjectSubmitted', (submitter, objIdx, event) => {
    console.log('New object submitted:', {
        submitter: submitter,
        objectIndex: objIdx.toString(),
        blockNumber: event.blockNumber,
        transactionHash: event.transactionHash
    });
});

// Listen for PermissionsSet events
poscanContract.on('PermissionsSet', (objIdx, setter, event) => {
    console.log('Permissions updated:', {
        objectIndex: objIdx.toString(),
        setter: setter,
        blockNumber: event.blockNumber,
        transactionHash: event.transactionHash
    });
});
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

## Security Notes

- Always validate OBJ file format before submission
- Check object ownership before setting permissions
- Verify property values are within acceptable ranges
- Use appropriate approval requirements for sensitive objects
- Consider gas costs for large object files 