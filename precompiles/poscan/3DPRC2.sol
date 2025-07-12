// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// Place the constant outside the interface
address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000903;

/**
 * @title 3DPRC2
 * @notice Interface for the PoScan precompile
 * @dev
 */
interface I3DPRC2 {
    struct Property {
        uint32 propIdx;
        uint128 maxValue;
    }
    struct Permission {
        address who;
        uint32 maxCopies;
        uint64 until;
    }

    /**
     * @notice Property struct for getProperties
     */
    struct PropertyInfo {
        bool isValid; // true if the property exists
        uint32 propIdx;
        string name;
        uint8 class; // 0=Relative, 1=Absolute
        uint128 maxValue;
    }

    /**
     * @notice Object struct for getObject
     */
    struct ObjectInfo {
        bool isValid; // true if the object exists
        uint8 state;
        uint64 stateBlock;
        uint8 category;
        uint64 whenCreated;
        uint64 whenApproved;
        address owner; // Changed from bytes32 to address (H160)
        bool isPrivate;
        bytes32[] hashes;
        uint8 numApprovals;
        Property[] prop;
        bool isReplica;
        uint32 originalObj;
        uint64 snHash;
        address inspector;
        bool isOwnershipAbdicated;
    }

    /// @dev Emitted when a new object is submitted
    event ObjectSubmitted(address indexed submitter, uint32 indexed objIdx);

    /// @dev Emitted when permissions are set for a private object
    event PermissionsSet(uint32 indexed objIdx, address indexed setter);

    /// @dev Emitted when an object enters QC inspection
    event QCInspecting(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object passes QC
    event QCPassed(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object is rejected by QC
    event QCRejected(uint32 indexed objIdx, address indexed inspector);

    /**
     * @notice Get object details by index
     * @param objIdx The object index
     * @return info The object info struct
     * @custom:selector 0xa1b2c001
     * @dev state: 0=Created, 1=Estimating, 2=Estimated, 3=NotApproved, 4=Approved, 5=QCInspecting, 6=QCPassed, 7=QCRejected
     */
    function getObject(uint32 objIdx) external view returns (ObjectInfo memory info);

    /**
     * @notice Get all object indices owned by an address
     * @param owner The owner address (EVM H160)
     * @return objectIndices Array of object indices (uint32[])
     * @custom:selector 0x7e2c4b2a
     */
    function getObjectsOf(address owner) external view returns (uint32[] memory objectIndices);

    /**
     * @notice Get the total number of objects
     * @return count The total number of objects (uint32)
     * @custom:selector 0x2f7b5f32
     */
    function getObjectCount() external view returns (uint32 count);

    /**
     * @notice Get all available properties for object tokenization
     * @return properties Array of PropertyInfo structs
     * @custom:selector 0x6e2b7b1a
     */
    function getProperties() external view returns (PropertyInfo[] memory properties);

    /**
     * @notice Get the storage fee per byte
     * @return exists True if the fee is set, false otherwise
     * @return fee The storage fee per byte (uint64)
     * @custom:selector 0x4e2b7b1b
     */
    function getFeePerByte() external view returns (bool exists, uint64 fee);

    /**
     * @notice Get the account lock value for a given address
     * @param owner The owner address (EVM H160)
     * @return lock The account lock value (uint128)
     * @custom:selector 0x8e2b7b1c
     */
    function getAccountLock(address owner) external view returns (uint128 lock);

    /**
     * @notice Submit a new object to the PoScan pallet
     * @param category The object category (enum discriminant)
     * @param algo3d The Algo3D variant (only used if category == Objects3D, otherwise set to 0)
     * @param isPrivate Whether the object is private
     * @param obj The OBJ file bytes (max 1MB)
     * @param numApprovals Number of approvals required
     * @param hashes Optional array of object hashes (can be empty for None)
     * @param properties Array of property values
     * @param isReplica Whether this is a replica
     * @param originalObj The original object index (if replica)
     * @param snHash The serial number hash (if replica, else 0)
     * @return success True if the object was submitted successfully
     * @dev Emits an ObjectSubmitted event on success
     * @custom:selector 0x0c53c51c
     */
    function putObject(
        uint8 category,
        uint8 algo3d,
        bool isPrivate,
        bytes calldata obj,
        uint8 numApprovals,
        bytes32[] calldata hashes,
        Property[] calldata properties,
        bool isReplica,
        uint32 originalObj,
        uint64 snHash
    ) external returns (bool);

    /**
     * @notice Inspect and put file to poscan storage (QC flow)
     * @param category The object category
     * @param algo3d The Algo3D variant
     * @param isPrivate Whether the object is private
     * @param obj The OBJ file bytes
     * @param numApprovals Number of approvals required
     * @param hashes Optional array of object hashes
     * @param properties Array of property values
     * @param isReplica Whether this is a replica
     * @param originalObj The original object index (if replica)
     * @param snHash The serial number hash (if replica, else 0)
     * @param inspector The inspector address
     * @return success True if the object was submitted for QC
     * @custom:selector 0x0c53c51f
     */
    function inspectPutObject(
        uint8 category,
        uint8 algo3d,
        bool isPrivate,
        bytes calldata obj,
        uint8 numApprovals,
        bytes32[] calldata hashes,
        Property[] calldata properties,
        bool isReplica,
        uint32 originalObj,
        uint64 snHash,
        address inspector
    ) external returns (bool);

    /**
     * @notice Inspector approves the object (QC passed)
     * @param objIdx The object index
     * @return success True if approved
     * @custom:selector 0x0c53c520
     */
    function qcApprove(uint32 objIdx) external returns (bool);

    /**
     * @notice Inspector rejects the object (QC rejected)
     * @param objIdx The object index
     * @return success True if rejected
     * @custom:selector 0x0c53c521
     */
    function qcReject(uint32 objIdx) external returns (bool);

    /**
     * @notice Set permissions for private object replicas
     * @param objIdx The object index
     * @param permissions Array of permission structs
     * @return success True if permissions were set successfully
     * @dev Emits a PermissionsSet event on success
     * @custom:selector 0x0c53c51d
     */
    function setPrivateObjectPermissions(
        uint32 objIdx,
        Permission[] calldata permissions
    ) external returns (bool);

    /**
     * @notice Transfer object ownership to another account
     * @param objIdx The object index
     * @param newOwner The new owner address
     * @return success True if ownership was transferred
     * @custom:selector 0x0c53c522
     */
    function transferObjectOwnership(uint32 objIdx, address newOwner) external returns (bool);

    /**
     * @notice Abdicate object ownership (irreversible)
     * @param objIdx The object index
     * @return success True if ownership was abdicated
     * @custom:selector 0x0c53c523
     */
    function abdicateTheObjOwnership(uint32 objIdx) external returns (bool);
} 