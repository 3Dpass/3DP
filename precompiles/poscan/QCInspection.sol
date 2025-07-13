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
    event QCInspecting(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object passes QC
    event QCPassed(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when an object is rejected by QC
    event QCRejected(uint32 indexed objIdx, address indexed inspector);
    /// @dev Emitted when inspector fee is paid
    event InspectorFeePaid(uint32 indexed objIdx, address indexed inspector, uint256 fee);
    /// @dev Emitted when QC inspection times out and inspector fee is unreserved
    event QCInspectionTimeout(uint32 indexed objIdx, address indexed feePayer, uint256 fee);
    /// @dev Emitted when permissions are set for a private object
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