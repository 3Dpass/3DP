// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The Identity contract's address.
address constant IDENTITY_ADDRESS = 0x0000000000000000000000000000000000000904;

/// @dev The Identity contract's instance.
Identity constant IDENTITY_CONTRACT = Identity(IDENTITY_ADDRESS);

/// @author 3dpass
/// @title Pallet Identity Interface
/// @title The interface through which solidity contracts will interact with the Identity pallet
/// @custom:address 0x0000000000000000000000000000000000000904
interface Identity {
    /// @dev Associated raw data.
    struct Data {
        /// Is `true` if it represents data, else the abscense of data is represented by `false`.
        bool hasData;
        /// The contained value.
        bytes value;
    }

    /// @dev The super-identity of an alternative "sub" identity.
    struct SuperOf {
        /// Is `true` if the struct is valid, `false` otherwise.
        bool isValid;
        /// The super account.
        address account; // Changed from bytes32 to address (H160)
        /// The associated data.
        Data data;
    }

    /// @dev Alternative "sub" identities of an account.
    struct SubsOf {
        /// The deposit against this identity.
        uint256 deposit;
        /// The sub accounts
        address[] accounts; // Changed from bytes32[] to address[]
    }

    /// @dev Registrar judgements are limited to attestations on these fields.
    struct IdentityFields {
        /// Set to `true` if the display field is supported, `false` otherwise.
        bool display;
        /// Set to `true` if the legal field is supported, `false` otherwise.
        bool legal;
        /// Set to `true` if the web field is supported, `false` otherwise.
        bool web;
        /// Set to `true` if the riot field is supported, `false` otherwise.
        bool riot;
        /// Set to `true` if the email field is supported, `false` otherwise.
        bool email;
        /// Set to `true` if the PGP Fingerprint field is supported, `false` otherwise.
        bool pgpFingerprint;
        /// Set to `true` if the image field is supported, `false` otherwise.
        bool image;
        /// Set to `true` if the twitter field is supported, `false` otherwise.
        bool twitter;
    }

    /// @dev Registrar info.
    struct Registrar {
        /// Is `true` if the struct is valid, `false` otherwise.
        bool isValid;
        /// The registrar's index.
        uint32 index;
        /// The account address.
        address account; // Changed from bytes32 to address (H160)
        /// Amount required to be given to the registrar for them to provide judgement.
        uint256 fee;
        /// Relevant fields for this registrar.
        IdentityFields fields;
    }

    /// @dev Represents an additional field in identity info.
    struct Additional {
        /// The associated key.
        Data key;
        /// The associated value.
        Data value;
    }

    /// @dev The identity information set for an account.
    struct IdentityInfo {
        /// Represents the additional fields for the identity.
        Additional[] additional;
        /// Represents the display info for the identity.
        Data display;
        /// Represents the legal info for the identity.
        Data legal;
        /// Represents the web info for the identity.
        Data web;
        /// Represents the riot info for the identity.
        Data riot;
        /// Represents the email info for the identity.
        Data email;
        /// Set to `true` if `pgpFingerprint` is set, `false` otherwise.
        bool hasPgpFingerprint;
        /// Represents a 20-byte the PGP fingerprint info for the identity.
        bytes pgpFingerprint;
        /// Represents the image info for the identity.
        Data image;
        /// Represents the twitter info for the identity.
        Data twitter;
    }

    /// @dev Judgement provided by a registrar.
    struct Judgement {
        /// The default value; no opinion is held.
        bool isUnknown;
        /// No judgement is yet in place, but a deposit is reserved as payment for providing one.
        bool isFeePaid;
        /// The deposit reserved for providing a judgement.
        uint256 feePaidDeposit;
        /// The data appears to be reasonably acceptable in terms of its accuracy.
        bool isReasonable;
        /// The target is known directly by the registrar and the registrar can fully attest to it.
        bool isKnownGood;
        /// The data was once good but is currently out of date.
        bool isOutOfDate;
        /// The data is imprecise or of sufficiently low-quality to be problematic.
        bool isLowQuality;
        /// The data is erroneous. This may be indicative of malicious intent.
        bool isErroneous;
    }

    /// @dev Judgement item provided by a registrar.
    struct JudgementInfo {
        /// The registrar's index that provided this judgement.
        uint32 registrarIndex;
        /// The registrar's provided judgement.
        Judgement judgement;
    }

    /// @dev Registrar info.
    struct Registration {
        /// Is `true` if the struct is valid, `false` otherwise.
        bool isValid;
        /// The judgments provided on this identity.
        JudgementInfo[] judgements;
        /// Amount required to be given to the registrar for them to provide judgement.
        uint256 deposit;
        /// The associated identity info.
        IdentityInfo info;
    }

    /// @dev Alternative "sub" identity of an account.
    struct SubAccount {
        /// The account address.
        address account; // Changed from bytes32 to address (H160)
        /// The associated data.
        Data data;
    }

    /// @dev Retrieve identity information for an account.
    /// @custom:selector 0x3839f783
    /// @param who The requested account
    function identity(address who) external view returns (Registration memory);

    /// @dev Retrieve super account for an account.
    /// @custom:selector 0xc18110d6
    /// @param who The requested account
    function superOf(address who) external view returns (SuperOf memory);

    /// @dev Retrieve sub accounts for an account.
    /// @custom:selector 0x3f08986b
    /// @param who The requested account
    function subsOf(address who) external view returns (SubsOf memory);

    /// @dev Retrieve the registrars.
    /// @custom:selector 0xe88e512e
    function registrars() external view returns (Registrar[] memory);

    /// @notice Returns the list of suspended registrar indices
    /// @custom:selector 0x6e2b7b1a
    /// @return suspended Array of suspended registrar indices
    function suspendedRegistrars() external view returns (uint32[] memory suspended);

    /// @dev Set identity info for the caller.
    /// @custom:selector 0x7e08b4cb
    /// @param info The identity info
    /// @notice Emits IdentitySet(address indexed who) event
    function setIdentity(IdentityInfo memory info) external;

    /// @dev Set sub accounts for the caller.
    /// @custom:selector 0x5a5a3591
    /// @param subs The sub accounts
    function setSubs(SubAccount[] memory subs) external;

    /// @dev Clears identity of the caller.
    /// @custom:selector 0x7a6a10c7
    /// @notice Emits IdentityCleared(address indexed who) event
    function clearIdentity() external;

    /// @dev Requests registrar judgement on caller's identity.
    /// @custom:selector 0xd523ceb9
    /// @param regIndex The registrar's index
    /// @param maxFee The maximum fee the caller is willing to pay
    /// @notice Emits JudgementRequested(address indexed who, uint32 regIndex) event
    function requestJudgement(uint32 regIndex, uint256 maxFee) external;

    /// @dev Cancels the caller's request for judgement from a registrar.
    /// @custom:selector 0xc79934a5
    /// @param regIndex The registrar's index
    /// @notice Emits JudgementUnrequested(address indexed who, uint32 regIndex) event
    function cancelRequest(uint32 regIndex) external;

    /// @dev Sets the registrar's fee for providing a judgement. Caller must be the account at the index.
    /// @custom:selector 0xa541b37d
    /// @param regIndex The registrar's index
    /// @param fee The fee the registrar will charge
    /// @notice Emits RegistrarFeeSet(uint32 indexed regIndex, uint256 fee) event
    function setFee(uint32 regIndex, uint256 fee) external;

    /// @dev Sets the registrar's account. Caller must be the account at the index.
    /// @custom:selector 0x889bc198
    /// @param regIndex The registrar's index
    /// @param newAccount The new account to set (address for H160)
    function setAccountId(uint32 regIndex, address newAccount) external;

    /// @dev Sets the registrar's identity fields. Caller must be the account at the index.
    /// @custom:selector 0x05297450
    /// @param regIndex The registrar's index
    /// @param fields The identity fields
    function setFields(uint32 regIndex, IdentityFields memory fields) external;

    /// @dev Provides judgement on an accounts identity.
    /// @custom:selector 0xcd7663a4
    /// @param regIndex The registrar's index
    /// @param target The target account to provide judgment for (address for H160)
    /// @param judgement The judgement to provide
    /// @param identity The hash of the identity info
    /// @notice Emits JudgementGiven(address indexed target, uint32 regIndex) event
    function provideJudgement(
        uint32 regIndex,
        address target,
        Judgement memory judgement,
        bytes32 identity
    ) external;

    /// @dev Add a "sub" identity account for the caller.
    /// @custom:selector 0x98717196
    /// @param sub The sub account (address for H160)
    /// @param data The associated data
    /// @notice Emits SubIdentityAdded(address indexed sub, address indexed main) event
    function addSub(address sub, Data memory data) external;

    /// @dev Rename a "sub" identity account of the caller.
    /// @custom:selector 0x452df561
    /// @param sub The sub account (address for H160)
    /// @param data The new associated data
    function renameSub(address sub, Data memory data) external;

    /// @dev Removes a "sub" identity account of the caller.
    /// @custom:selector 0xb0a323e0
    /// @param sub The sub account (address for H160)
    /// @notice Emits SubIdentityRemoved(address indexed sub, address indexed main) event
    function removeSub(address sub) external;

    /// @dev Removes the sender as a sub-account.
    /// @custom:selector 0xd5a3c2c4
    /// @notice Emits SubIdentityRevoked(address indexed sub) event
    function quitSub() external;

    /// @dev An identity was set or reset (which will remove all judgements).
    /// @param who Address of the target account
    event IdentitySet(address indexed who);

    /// @dev An identity was cleared, and the given balance returned.
    /// @param who Address of the target account
    event IdentityCleared(address indexed who);

    /// @dev A judgement was asked from a registrar.
    /// @param who Address of the requesting account
    /// @param regIndex The registrar's index
    event JudgementRequested(address indexed who, uint32 regIndex);

    /// @dev A judgement request was retracted.
    /// @param who Address of the target account.
    /// @param regIndex The registrar's index
    event JudgementUnrequested(address indexed who, uint32 regIndex);

    /// @dev A judgement was given by a registrar.
    /// @param target Address of the target account
    /// @param regIndex The registrar's index
    event JudgementGiven(address indexed target, uint32 regIndex);

    /// @dev A sub-identity was added to an identity and the deposit paid.
    /// @param sub Address of the sub account
    /// @param main Address of the main account
    event SubIdentityAdded(address indexed sub, address indexed main);

    /// @dev A sub-identity was removed from an identity and the deposit freed.
    /// @param sub Address of the sub account
    /// @param main Address of the main account
    event SubIdentityRemoved(address indexed sub, address indexed main);

    /// @dev A sub-identity was cleared and the given deposit repatriated from the main identity account to the sub-identity account
    /// @param sub Address of the sub account
    event SubIdentityRevoked(address indexed sub);

    /// @dev A registrar's fee was set.
    /// @param regIndex The registrar's index
    /// @param fee The new fee amount
    event RegistrarFeeSet(uint32 indexed regIndex, uint256 fee);
}

// All account fields and parameters are now EVM address (H160), not bytes32/H256.
