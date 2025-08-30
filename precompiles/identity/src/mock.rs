// Copyright 2025 3Dpass
// Mock runtime and tests for identity precompile helper validation

use super::*;
use sp_core::U256;
use sp_runtime::{testing::{Header, H256}, traits::{BlakeTwo256, IdentityLookup, ConstU32, ConstU64}};
use sp_runtime::AccountId32;
use frame_support::parameter_types;
use frame_system as system;
use frame_system::EnsureRoot;
use frame_support::construct_runtime;
extern crate hex;
use crate::{
    SELECTOR_LOG_IDENTITY_SET,
    SELECTOR_LOG_IDENTITY_CLEARED,
    SELECTOR_LOG_JUDGEMENT_REQUESTED,
    SELECTOR_LOG_JUDGEMENT_UNREQUESTED,
    SELECTOR_LOG_JUDGEMENT_GIVEN,
    SELECTOR_LOG_SUB_IDENTITY_ADDED,
    SELECTOR_LOG_SUB_IDENTITY_REMOVED,
    SELECTOR_LOG_SUB_IDENTITY_REVOKED,
    SELECTOR_LOG_REGISTRAR_FEE_SET,
};
use fp_evm::PrecompileFailure;
use std::str;

// Remove manual struct TestRuntime and type aliases
// Remove manual impl blocks for frame_system::Config and pallet_identity::Config
// Use only the construct_runtime! macro and generated types

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxConsumers: u32 = 16;
    pub const BasicDeposit: u32 = 1;
    pub const FieldDeposit: u32 = 1;
    pub const SubAccountDeposit: u32 = 1;
}

type AccountId = sp_runtime::AccountId32;
type BlockNumber = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

frame_support::construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>},
    }
);

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_identity::Config for TestRuntime {
    type Event = Event;
    type Currency = (); // Not used in these tests
    type Slashed = ();
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = ConstU32<2>;
    type MaxAdditionalFields = ConstU32<1>;
    type MaxRegistrars = ConstU32<2>;
    type RegistrarOrigin = EnsureRoot<AccountId>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{decode_identity_info, decode_judgement, UnboundedBytes};
    use frame_support::BoundedVec;
    use sp_std::vec;
    use frame_support::traits::ConstU32;

    type TestBoundedBytes = BoundedVec<u8, ConstU32<64>>;

    fn revert_message<T>(res: &EvmResult<T>) -> Option<String> {
        if let Err(PrecompileFailure::Revert { output, .. }) = res {
            // Try to decode the output as UTF-8, skipping the first 68 bytes (EVM revert prefix)
            if output.len() > 68 {
                if let Ok(msg) = str::from_utf8(&output[68..]) {
                    return Some(msg.trim_end_matches(char::from(0)).to_string());
                }
            }
        }
        None
    }

    #[test]
    fn test_boundedvec_overflow_in_additional_fields() {
        // MaxAdditionalFields = 1, so 2 entries should fail
        let info = (
            vec![((false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap())), ((false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()))],
            (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()),
            false, UnboundedBytes::try_from(vec![]).unwrap(), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap())
        );
        let res = decode_identity_info::<TestRuntime>(info);
        let msg = revert_message(&res).unwrap_or_default();
        println!("test_boundedvec_overflow_in_additional_fields error: {}", msg);
        assert!(msg.contains("BoundedVec overflow in additional fields"));
    }

    #[test]
    fn test_invalid_pgp_fingerprint_length() {
        // hasPgpFingerprint = true, but length != 20
        let info = (
            vec![],
            (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap()),
            true, UnboundedBytes::try_from(vec![1,2,3]).unwrap(), (false, UnboundedBytes::try_from(vec![]).unwrap()), (false, UnboundedBytes::try_from(vec![]).unwrap())
        );
        let res = decode_identity_info::<TestRuntime>(info);
        let msg = revert_message(&res).unwrap_or_default();
        println!("test_invalid_pgp_fingerprint_length error: {}", msg);
        assert!(msg.contains("pgp_fingerprint must be 20 bytes if present") || msg.contains("pgp_fingerprint must be 20 bytes"));
    }

    #[test]
    fn test_multiple_discriminants_in_judgement() {
        // Both isUnknown and isFeePaid set
        let j = (true, true, U256::zero(), false, false, false, false, false);
        let res = decode_judgement::<TestRuntime>(j);
        let msg = revert_message(&res).unwrap_or_default();
        println!("test_multiple_discriminants_in_judgement error: {}", msg);
        assert!(msg.contains("Only one Judgement discriminant can be set"));
    }

    #[test]
    fn test_no_discriminant_in_judgement() {
        // All false
        let j = (false, false, U256::zero(), false, false, false, false, false);
        let res = decode_judgement::<TestRuntime>(j);
        let msg = revert_message(&res).unwrap_or_default();
        println!("test_no_discriminant_in_judgement error: {}", msg);
        assert!(msg.contains("Invalid Judgement discriminant"));
    }

    #[test]
    pub fn print_event_selectors() {
        println!("IdentitySet: 0x{}", hex::encode(SELECTOR_LOG_IDENTITY_SET));
        println!("IdentityCleared: 0x{}", hex::encode(SELECTOR_LOG_IDENTITY_CLEARED));
        println!("JudgementRequested: 0x{}", hex::encode(SELECTOR_LOG_JUDGEMENT_REQUESTED));
        println!("JudgementUnrequested: 0x{}", hex::encode(SELECTOR_LOG_JUDGEMENT_UNREQUESTED));
        println!("JudgementGiven: 0x{}", hex::encode(SELECTOR_LOG_JUDGEMENT_GIVEN));
        println!("SubIdentityAdded: 0x{}", hex::encode(SELECTOR_LOG_SUB_IDENTITY_ADDED));
        println!("SubIdentityRemoved: 0x{}", hex::encode(SELECTOR_LOG_SUB_IDENTITY_REMOVED));
        println!("SubIdentityRevoked: 0x{}", hex::encode(SELECTOR_LOG_SUB_IDENTITY_REVOKED));
        println!("RegistrarFeeSet: 0x{}", hex::encode(SELECTOR_LOG_REGISTRAR_FEE_SET));
    }
} 