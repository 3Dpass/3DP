// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @dev The SerialNumbers precompile contract's address.
address constant SERIAL_NUMBERS_ADDRESS = 0x0000000000000000000000000000000000000905;

/// @dev The SerialNumbers precompile contract's instance.
SerialNumbers constant SERIAL_NUMBERS_CONTRACT = SerialNumbers(SERIAL_NUMBERS_ADDRESS);

/// @dev Emitted when a new serial number is created
event SerialNumberCreated(address indexed owner, uint64 snIndex, bytes16 snHash, uint32 blockIndex);
/// @dev Emitted when a serial number is used
event SerialNumberUsed(bytes16 indexed snHash, address indexed user);
/// @dev Emitted when a serial number is expired
event SerialNumberExpired(uint64 indexed snIndex, bytes16 snHash);

/// @title Pallet SerialNumbers Interface
/// @notice The interface through which Solidity contracts interact with the SerialNumbers pallet
/// @custom:address 0x0000000000000000000000000000000000000905
interface SerialNumbers {
    /// @dev Details of a serial numbers
    struct SerialNumberDetails {
        bool isValid; // true if the serial number exists, false otherwise
        uint64 snIndex;
        bytes16 snHash;
        address owner;
        uint256 created;
        uint32 blockIndex;
        bool isExpired;
        uint256 expired;
    }

    /// @notice Create a new serial number for the caller for the given block index
    /// @custom:selector 0x0a0b0c01
    /// @param blockIndex The index within the block (0..MaxSerialNumbersPerBlock)
    /// @return snHash The generated serial number hash (bytes16)
    /// @return snIndex The serial number index
    /// @return blockIndexOut The block index used
    function createSerialNumber(uint32 blockIndex) external returns (bytes16 snHash, uint64 snIndex, uint32 blockIndexOut);

    /// @notice Expire a serial number (only owner can call)
    /// @custom:selector 0x0a0b0c02
    /// @param snIndex The serial number index
    /// @return success True if expired
    function expireSerialNumber(uint64 snIndex) external returns (bool success);

    /// @notice Mark a serial number as used (redemption)
    /// @custom:selector 0x0a0b0c03
    /// @param snHash The serial number hash (bytes16)
    /// @return success True if marked as used
    function useSerialNumber(bytes16 snHash) external returns (bool success);

    /// @notice Get details for a serial number by index
    /// @custom:selector 0x0a0b0c04
    /// @param snIndex The serial number index
    /// @return isValid True if the serial number exists, false otherwise
    /// @return snIndexOut The serial number index
    /// @return snHash The serial number hash (bytes32, first 16 bytes are the hash, rest are zero padding)
    /// @return owner The owner address
    /// @return created The creation timestamp
    /// @return blockIndex The block index used
    /// @return isExpired True if expired
    /// @return expired The expiration timestamp
    function getSerialNumber(uint64 snIndex) external view returns (
        bool isValid,
        uint64 snIndexOut,
        bytes32 snHash,
        address owner,
        uint256 created,
        uint32 blockIndex,
        bool isExpired,
        uint256 expired
    );

    /// @notice Check if a serial number has been used
    /// @custom:selector 0x0a0b0c05
    /// @param snHash The serial number hash (bytes16)
    /// @return isValid True if the serial number exists, false otherwise
    /// @return used True if used
    function isSerialNumberUsed(bytes16 snHash) external view returns (bool isValid, bool used);

    /// @notice Get the serial number index for a given serial number hash
    /// @custom:selector 0x0a0b0c06
    /// @param snHash The serial number hash (bytes16)
    /// @return exists True if the serial number exists
    /// @return snIndex The serial number index (0 if not found)
    function snByHash(bytes16 snHash) external view returns (bool exists, uint64 snIndex);
} 