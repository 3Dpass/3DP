// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title I3DPRC2
 * @notice Interface for the PoScan precompile
 * @dev
 */
interface I3DPRC2 {
    /// @dev Precompile address for PoScan
    address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000903;

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
        bytes32 owner;
        bool isPrivate;
        bytes32[] hashes;
        uint8 numApprovals;
        Property[] prop;
    }

    /// @dev Emitted when a new object is submitted
    event ObjectSubmitted(address indexed submitter, uint32 indexed objIdx);

    /**
     * @notice Get object details by index
     * @param objIdx The object index
     * @return info The object info struct
     * @custom:selector 0xa1b2c001
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
     * @return success True if the object was submitted successfully
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
        uint32 originalObj
    ) external returns (bool);

    function setPrivateObjectPermissions(
        uint32 objIdx,
        Permission[] calldata permissions
    ) external returns (bool);
} 