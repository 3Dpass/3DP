// Copyright 2025 3Dpass
// Mock runtime and tests for identity precompile helper validation

use super::*;
use sp_core::U256;
use sp_runtime::{testing::{Header, H256}, traits::{BlakeTwo256, IdentityLookup, Everything, ConstU32, ConstU64}};
use sp_runtime::AccountId32;
use frame_support::parameter_types;
use frame_system as system;
use frame_system::mocking::{MockOrigin, MockPalletInfo};
use frame_system::EnsureRoot;

// Test runtime
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TestRuntime;
type AccountId = AccountId32;
type BlockNumber = u64;
type Balance = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxConsumers: u32 = 16;
    pub const BasicDeposit: Balance = 1;
    pub const FieldDeposit: Balance = 1;
    pub const SubAccountDeposit: Balance = 1;
}

impl system::Config for TestRuntime {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = MockOrigin<AccountId>;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = ();
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = MockPalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_identity::Config for TestRuntime {
    type Event = ();
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
    use super::super::{decode_identity_info, decode_judgement};

    #[test]
    fn test_boundedvec_overflow_in_additional_fields() {
        // MaxAdditionalFields = 1, so 2 entries should fail
        let info = (
            vec![((false, vec![]), (false, vec![])), ((false, vec![]), (false, vec![]))],
            (false, vec![]), (false, vec![]), (false, vec![]), (false, vec![]), (false, vec![]),
            false, vec![], (false, vec![]), (false, vec![])
        );
        let res = decode_identity_info::<TestRuntime>(info);
        assert!(res.is_err());
        let msg = format!("{:?}", res);
        assert!(msg.contains("BoundedVec overflow"));
    }

    #[test]
    fn test_invalid_pgp_fingerprint_length() {
        // hasPgpFingerprint = true, but length != 20
        let info = (
            vec![],
            (false, vec![]), (false, vec![]), (false, vec![]), (false, vec![]), (false, vec![]),
            true, vec![1,2,3], (false, vec![]), (false, vec![])
        );
        let res = decode_identity_info::<TestRuntime>(info);
        assert!(res.is_err());
        let msg = format!("{:?}", res);
        assert!(msg.contains("pgp_fingerprint must be 20 bytes"));
    }

    #[test]
    fn test_multiple_discriminants_in_judgement() {
        // Both isUnknown and isFeePaid set
        let j = (true, true, U256::zero(), false, false, false, false, false);
        let res = decode_judgement::<TestRuntime>(j);
        assert!(res.is_err());
        let msg = format!("{:?}", res);
        assert!(msg.contains("Only one Judgement discriminant"));
    }

    #[test]
    fn test_no_discriminant_in_judgement() {
        // All false
        let j = (false, false, U256::zero(), false, false, false, false, false);
        let res = decode_judgement::<TestRuntime>(j);
        assert!(res.is_err());
        let msg = format!("{:?}", res);
        assert!(msg.contains("Invalid Judgement discriminant"));
    }
} 