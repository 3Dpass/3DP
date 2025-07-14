// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// Place the constant outside the interface
address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000903;

/// @title 3DPRC2 Object/Tokenization Interface
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
        string ipfsLink; // IPFS link for self-proved objects
    }

    /// @dev Emitted when a new object is submitted
    event ObjectSubmitted(address indexed submitter, uint32 indexed objIdx);

    /// @dev Emitted when permissions are set for a private object
    event PermissionsSet(uint32 indexed objIdx, address indexed setter);

    /// @dev Emitted when object ownership is transferred
    event ObjectOwnershipTransferred(uint32 indexed objIdx, address indexed oldOwner, address indexed newOwner);
    /// @dev Emitted when unspent rewards are unlocked
    event UnspentRewardsUnlocked(uint32 indexed objIdx, address indexed feePayer, uint256 amount);

    /**
     * @notice Get object details by index
     * @param objIdx The object index
     * @return info The object info struct
     * @custom:selector 0xa1b2c001
     * @dev state: 0=Created, 1=Estimating, 2=Estimated, 3=NotApproved, 4=Approved, 5=QCInspecting, 6=QCPassed, 7=QCRejected, 8=SelfProved
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
     * @notice Get all object indices assigned to an inspector
     * @param inspector The inspector address (EVM H160)
     * @return objectIndices Array of object indices (uint32[])
     * @custom:selector 0x7e2c4b2b
     */
    function getObjectsOfInspector(address inspector) external view returns (uint32[] memory objectIndices);

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
     * @notice Get the council-controlled rewards amount
     * @return exists True if the rewards are set, false otherwise
     * @return rewards The rewards amount (uint128)
     * @custom:selector 0x8e2b7b1d
     */
    function getRewards() external view returns (bool exists, uint128 rewards);

    /**
     * @notice Get the dynamic rewards growth rate
     * @return exists True if the growth rate is set, false otherwise
     * @return rate The dynamic rewards growth rate (uint32)
     * @custom:selector 0x8e2b7b1e
     */
    function getDynamicRewardsGrowthRate() external view returns (bool exists, uint32 rate);

    /**
     * @notice Get the author share percentage
     * @return exists True if the share percentage is set, false otherwise
     * @return part The author share percentage (uint8, 0-100)
     * @custom:selector 0x8e2b7b1f
     */
    function getAuthorPart() external view returns (bool exists, uint8 part);

    /**
     * @notice Get unspent rewards for a NotApproved object
     * @param objIdx The object index
     * @return exists True if the object exists and has unspent rewards, false otherwise
     * @return amount The unspent rewards amount (uint128)
     * @custom:selector 0x4e2b7b1e
     */
    function getUnspentRewards(uint32 objIdx) external view returns (bool exists, uint128 amount);

    /**
     * @notice Get pending storage fees that will be distributed to validators
     * @return exists True if there are pending fees, false otherwise
     * @return amount The pending storage fees amount (uint128)
     * @custom:selector 0x4e2b7b1f
     */
    function getPendingStorageFees() external view returns (bool exists, uint128 amount);

    /**
     * @notice Get object index by proof of existence hash
     * @param proofOfExistence The proof of existence hash (bytes32)
     * @return exists True if an object with the given proof exists, false otherwise
     * @return objIdx The object index (uint32, 0 if not found)
     * @custom:selector 0x9e2c4b2c
     */
    function getObjectIdxByProofOfExistence(bytes32 proofOfExistence) external view returns (bool exists, uint32 objIdx);

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
     * @param isSelfProved Whether this is a self-proved object
     * @param proofOfExistence The proof of existence hash (if self-proved)
     * @param ipfsLink The IPFS link (if self-proved)
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
        uint64 snHash,
        bool isSelfProved,
        bytes32 proofOfExistence,
        string calldata ipfsLink
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

    /**
     * @notice Verify a self-proved object by providing the actual object data
     * @param obj The OBJ file bytes (max 1MB)
     * @return success True if the object was verified successfully
     * @dev Emits an ObjectSubmitted event on success
     * @custom:selector 0x0c53c524
     */
    function verifySelfProvedObject(bytes calldata obj) external returns (bool);

    /**
     * @notice Unlock unspent rewards for a NotApproved object (fee payer only)
     * @param objIdx The object index
     * @return success True if the rewards were unlocked successfully
     * @dev Only the fee payer can call this function
     * @dev Only works for objects in NotApproved state
     * @custom:selector 0x0c53c525
     */
    function unlockUnspentRewards(uint32 objIdx) external returns (bool);
} 