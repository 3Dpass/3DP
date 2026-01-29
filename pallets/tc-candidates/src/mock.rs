//! Mock for tests.

use crate::pallet::{Config, DecodeOngoingProposerHash, ReferendumApprovedChecker};
use frame_support::traits::{ConstU32, Get};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		TcCandidates: pallet_tc_candidates,
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

pub struct ReferendaPalletName;
impl Get<&'static str> for ReferendaPalletName {
	fn get() -> &'static str {
		"Referenda"
	}
}

pub struct DecodeOngoingImpl;
impl DecodeOngoingProposerHash for DecodeOngoingImpl {
	type AccountId = u64;
	type Hash = H256;
	fn decode_ongoing(_bytes: &[u8]) -> Option<(Self::AccountId, Self::Hash)> {
		None
	}
}

pub struct ReferendumApprovedImpl;
impl ReferendumApprovedChecker for ReferendumApprovedImpl {
	fn is_approved(_index: u32) -> bool {
		false
	}
}

pub struct SetCodeCheckerImpl;
impl crate::pallet::CheckSetCodeCall for SetCodeCheckerImpl {
	type Hash = H256;
	fn is_set_code(_hash: &Self::Hash) -> bool {
		false
	}
}

impl Config for Test {
	type Event = RuntimeEvent;
	type ReferendaPalletName = ReferendaPalletName;
	type DecodeOngoing = DecodeOngoingImpl;
	type ReferendumApproved = ReferendumApprovedImpl;
	type SetCodeChecker = SetCodeCheckerImpl;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
