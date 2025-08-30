// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

address constant PRECOMPILE_ADDR = 0x0000000000000000000000000000000000000903;

/// @title QC Inspection Interface for 3DPRC2 Precompile
interface IQCInspection {
    struct Property {
        uint8 propIdx;
        uint256 maxValue;
    }
    struct Permission {
        address who;
        uint32 until;
        uint32 maxCopies;
    }

    /// @dev Emitted when an object enters QC inspection
    /// @custom:selector 99d934e15b1e9f0c54ade770a4cec8c09b0317ecbc2dda047f7f11bd4c09d4b6
    event QCInspecting(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object passes QC
    /// @custom:selector 1b7600d3c3de3e71aceb593db0de1a27988f1f4ee1bf443f6f2b6f4fbc708c97
    event QCPassed(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object is rejected by QC
    /// @custom:selector e71aa02f1750578098ed760fb72ecca09a80be8e2b88d10c4ab6159f75aafe48
    event QCRejected(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when inspector fee is paid
    /// @custom:selector 66a0ee0e1e04e690057977ea001e29370975ca4cb2edf05f3f274400ac60107e
    event InspectorFeePaid(uint32 indexed objIdx, address indexed inspector, uint256 fee);
    /// @dev Emitted when QC inspection times out and inspector fee is unreserved
    /// @custom:selector f4a61831b610c582d0087536f502e0a93efae3f12b83a68a087fed6e0f8688b5
    event QCInspectionTimeout(uint32 indexed objIdx, address indexed feePayer, uint256 fee);
    /// @dev Emitted when permissions are set for a private object
    /// @custom:selector 8bd1bc05f187ab2e18d77e8ff98a49c3946c5d8a6c0a4bf7a452afed0635abda
    event PermissionsSet(uint32 indexed objIdx, address indexed setter);

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
     * @param inspectorFee The fee to be paid to the inspector (in wei)
     * @param qcTimeout Custom QC timeout in blocks (5-100,000)
     * @param isSelfProved Whether this is a self-proved object
     * @param proofOfExistence The proof of existence hash (if self-proved)
     * @param ipfsLink The IPFS link (if self-proved)
     * @return success True if the object was submitted for QC
     * @dev Replicas must be submitted as self-proved objects. Submitting a replica as a regular object will revert.
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
        address inspector,
        uint256 inspectorFee,
        uint32 qcTimeout,
        bool isSelfProved,
        bytes32 proofOfExistence,
        string calldata ipfsLink
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
} 