//! A Super Runtime. This runtime demonstrates all the recipes in the kitchen
//! in a single super runtime.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]
// #![allow(clippy::large_enum_variant)]
#![allow(clippy::from_over_into)]
// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// Include the genesis helper module when building to std
#[cfg(feature = "std")]
pub mod genesis;
mod fee;
mod weights;

use core::convert::TryInto;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_transaction_payment::CurrencyAdapter;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
// use sp_core::u32_trait::{_1, _2, _4, _5};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, IdentifyAccount, IdentityLookup, NumberFor, Verify,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	transaction_validity::{TransactionSource, TransactionValidity, TransactionPriority},
	ApplyExtrinsicResult, MultiSignature, // ModuleId,
	traits::ConvertInto,
};
// use runtime_common::{
// 	auctions, claims, crowdloan, impl_runtime_weights, impls::DealWithFees, paras_registrar,
// 	prod_or_fast, slots, BlockHashCount, BlockLength, CurrencyToVote, SlowAdjustingFeeUpdate,
// };

use frame_system::EnsureRoot;
use sp_std::convert::TryFrom;
use sp_std::{
	cmp,
	collections::btree_map::BTreeMap,
	prelude::*,
};
use sp_arithmetic::Percent;
use sp_consensus_poscan::{DOLLARS, CENTS, MICROCENTS, MILLICENTS, DAYS, BLOCK_TIME, deposit};
use sp_consensus_poscan::POSCAN_COIN_ID;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;


pub use frame_support::{
	StorageValue, StorageMap, construct_runtime, parameter_types,
	traits::{
		ConstU32, Contains, EnsureOneOf,
		Currency, Randomness, LockIdentifier, OnUnbalanced, InstanceFilter, KeyOwnerProofSystem,
		PrivilegeCmp,
	},

	weights::{
		Weight, RuntimeDbWeight, DispatchClass, IdentityFee,
		constants::{
			WEIGHT_PER_SECOND, BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight
		},
	},
	weights::ConstantMultiplier,
	PalletId, RuntimeDebug,
};


#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	pub type SessionHandlers = Grandpa;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub grandpa: Grandpa,
		}
	}
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("poscan-runtime"),
	impl_name: create_runtime_str!("poscan-runtime"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

/// We currently allow all calls.
pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
	fn contains(_c: &Call) -> bool {
		true
	}
}

type MoreThanHalfCouncil = EnsureOneOf<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = POSCAN_COIN_ID;
}

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = BaseFilter;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProofSystem = ();
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;
	type HandleEquivocation = ();
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Difficulty;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	// type Event = Event;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = fee::WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = (); // SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 500 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

type ApproveOrigin = EnsureOneOf<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = ApproveOrigin;
	type RejectOrigin = MoreThanHalfCouncil;
	type Event = Event;
	type OnSlash = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = (); // Society;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = (); // weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = Bounties;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item=NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// Burn base fees.
			drop(fees);
			if let Some(tips) = fees_then_tips.next() {
				// Pay tips to miners.
				Author::on_unbalanced(tips);
			}
		}
	}
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type WeightInfo = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
}

parameter_types! {
	pub const BountyDepositBase: Balance = 100 * CENTS;
	pub const BountyDepositPayoutDelay: BlockNumber = 4 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * CENTS;
	pub const CuratorDepositMax: Balance = 500 * CENTS;
	pub const BountyValueMinimum: Balance = 200 * CENTS;
}

impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = (); // weights::pallet_bounties::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxActiveChildBountyCount: u32 = 100;
	pub const ChildBountyValueMinimum: Balance = BountyValueMinimum::get() / 10;
}

impl pallet_child_bounties::Config for Runtime {
	type Event = Event;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = (); // weights::pallet_child_bounties::WeightInfo<Runtime>;
}

// PoScan -->
impl pallet_poscan::Config for Runtime {
	type Event = Event;
	// type MaxBytesInHash = frame_support::traits::ConstU32<64>;
}

parameter_types! {
	pub const TargetBlockTime: u64 = BLOCK_TIME;
}

impl difficulty::Config for Runtime {
	type TargetBlockTime = TargetBlockTime;
}

const REWARDS_STEP: usize = 243000;
const MAX_REWARS_IDX: usize = 90;
const REWARDS: [u128; MAX_REWARS_IDX] = [
500_000_000 * MICROCENTS, 416_666_700 * MICROCENTS, 347_222_200 * MICROCENTS,
289_351_900 * MICROCENTS, 241_126_500 * MICROCENTS, 200_938_800 * MICROCENTS,
167_449_000 * MICROCENTS, 139_540_800 * MICROCENTS, 116_284_000 * MICROCENTS,
096_903_300 * MICROCENTS, 080_752_800 * MICROCENTS, 067_294_000 * MICROCENTS,
056_078_300 * MICROCENTS, 046_731_900 * MICROCENTS, 038_943_300 * MICROCENTS,
032_452_700 * MICROCENTS, 027_043_900 * MICROCENTS, 022_536_600 * MICROCENTS,
018_780_500 * MICROCENTS, 015_650_400 * MICROCENTS, 013_042_000 * MICROCENTS,
010_868_400 * MICROCENTS, 009_057_000 * MICROCENTS, 007_547_500 * MICROCENTS,
006_289_600 * MICROCENTS, 005_241_300 * MICROCENTS, 004_367_700 * MICROCENTS,
003_639_800 * MICROCENTS, 003_033_200 * MICROCENTS, 002_527_600 * MICROCENTS,
002_106_400 * MICROCENTS, 001_755_300 * MICROCENTS, 001_462_800 * MICROCENTS,
001_219_000 * MICROCENTS, 001_015_800 * MICROCENTS, 000_846_500 * MICROCENTS,
000_705_400 * MICROCENTS, 000_587_800 * MICROCENTS, 000_489_900 * MICROCENTS,
000_408_200 * MICROCENTS, 000_340_200 * MICROCENTS, 000_283_500 * MICROCENTS,
000_236_200 * MICROCENTS, 000_196_900 * MICROCENTS, 000_164_100 * MICROCENTS,
000_136_700 * MICROCENTS, 000_113_900 * MICROCENTS, 000_094_900 * MICROCENTS,
000_079_100 * MICROCENTS, 000_065_900 * MICROCENTS, 000_054_900 * MICROCENTS,
000_045_800 * MICROCENTS, 000_038_200 * MICROCENTS, 000_031_800 * MICROCENTS,
000_026_500 * MICROCENTS, 000_022_100 * MICROCENTS, 000_018_400 * MICROCENTS,
000_015_300 * MICROCENTS, 000_012_800 * MICROCENTS, 000_010_600 * MICROCENTS,
000_008_900 * MICROCENTS, 000_007_400 * MICROCENTS, 000_006_200 * MICROCENTS,
000_005_100 * MICROCENTS, 000_004_300 * MICROCENTS, 000_003_600 * MICROCENTS,
000_003_000 * MICROCENTS, 000_002_500 * MICROCENTS, 000_002_100 * MICROCENTS,
000_001_700 * MICROCENTS, 000_001_400 * MICROCENTS, 000_001_200 * MICROCENTS,
000_001_000 * MICROCENTS, 000_000_800 * MICROCENTS, 000_000_700 * MICROCENTS,
000_000_600 * MICROCENTS, 000_000_500 * MICROCENTS, 000_000_400 * MICROCENTS,
000_000_300 * MICROCENTS, 000_000_300 * MICROCENTS, 000_000_200 * MICROCENTS,
000_000_200 * MICROCENTS, 000_000_200 * MICROCENTS, 000_000_100 * MICROCENTS,
000_000_100 * MICROCENTS, 000_000_100 * MICROCENTS, 000_000_100 * MICROCENTS,
000_000_100 * MICROCENTS, 000_000_100 * MICROCENTS, 000_000_000 * MICROCENTS,
];

pub struct GenerateRewardLocks;

impl rewards::GenerateRewardLocks<Runtime> for GenerateRewardLocks {
	fn generate_reward_locks(
		current_block: BlockNumber,
		total_reward: Balance,
		lock_parameters: Option<rewards::LockParameters>,
	) -> BTreeMap<BlockNumber, Balance> {
		let mut locks = BTreeMap::new();
		let locked_reward = total_reward.saturating_sub(1 * DOLLARS);

		if locked_reward > 0 {
			let total_lock_period: BlockNumber;
			let divide: BlockNumber;

			if let Some(lock_parameters) = lock_parameters {
				total_lock_period = u32::from(lock_parameters.period) * DAYS;
				divide = u32::from(lock_parameters.divide);
			} else {
				total_lock_period = 100 * DAYS;
				divide = 10;
			}
			for i in 0..divide {
				let one_locked_reward = locked_reward / divide as u128;

				let estimate_block_number =
					current_block.saturating_add((i + 1) * (total_lock_period / divide));
				let actual_block_number = estimate_block_number / DAYS * DAYS;

				locks.insert(actual_block_number, one_locked_reward);
			}
		}

		locks
	}

	fn max_locks(lock_bounds: rewards::LockBounds) -> u32 {
		// Max locks when a miner mines at least one block every day till the lock period of
		// the first mined block ends.
		cmp::max(100, u32::from(lock_bounds.period_max))
	}

	fn calc_rewards(current_block: BlockNumber) -> Balance {
		let b: BlockNumber = REWARDS_STEP.try_into().unwrap();
		let idx: usize = (current_block / b).try_into().unwrap();
		if idx >= MAX_REWARS_IDX {
			return 0
		}
		REWARDS[idx].clone()
	}
}

parameter_types! {
	pub DonationDestination: AccountId = Treasury::account_id();
	pub const LockBounds: rewards::LockBounds = rewards::LockBounds {period_max: 500, period_min: 20,
																	divide_max: 50, divide_min: 2};
}

impl rewards::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type DonationDestination = DonationDestination;
	type GenerateRewardLocks = GenerateRewardLocks;
	type WeightInfo = crate::weights::rewards::WeightInfo<Self>;
	type LockParametersBounds = LockBounds;
}

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = Rewards::author() {
			Balances::resolve_creating(&author, amount);
		} else {
			drop(amount);
		}
	}
}

parameter_types! {
	// Minimum 100 bytes/KSM deposited (1 CENT/byte)
	pub const BasicDeposit: Balance = 1000 * CENTS;       // 258 bytes on-chain
	pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
	pub const SubAccountDeposit: Balance = 200 * CENTS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = MoreThanHalfCouncil;
	type RegistrarOrigin = MoreThanHalfCouncil;
	type WeightInfo = (); // weights::pallet_identity::WeightInfo<Runtime>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 100 * CENTS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = (); // weights::pallet_indices::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = (); // weights::pallet_multisig::WeightInfo<Runtime>;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = (); // weights::pallet_preimage::WeightInfo<Runtime>;
	type Event = Event;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type MaxSize = PreimageMaxSize;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

// parameter_types! {
// 	// One storage item; key size 32, value size 8; .
// 	pub const ProxyDepositBase: Balance = deposit(1, 8);
// 	// Additional storage item size of 33 bytes.
// 	pub const ProxyDepositFactor: Balance = deposit(0, 33);
// 	pub const MaxProxies: u16 = 32;
// 	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
// 	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
// 	pub const MaxPending: u16 = 32;
// }

// /// The type used to represent the kinds of proxying allowed.
// #[derive(
// Copy,
// Clone,
// Eq,
// PartialEq,
// Ord,
// PartialOrd,
// Encode,
// Decode,
// RuntimeDebug,
// MaxEncodedLen,
// scale_info::TypeInfo,
// )]
// pub enum ProxyType {
// 	Any,
// 	NonTransfer,
// 	Governance,
// 	Staking,
// 	IdentityJudgement,
// 	CancelProxy,
// 	Auction,
// 	Society,
// }
//
// impl Default for ProxyType {
// 	fn default() -> Self {
// 		Self::Any
// 	}
// }
//
// impl InstanceFilter<Call> for ProxyType {
// 	fn filter(&self, c: &Call) -> bool {
// 		match self {
// 			ProxyType::Any => true,
// 			ProxyType::NonTransfer => matches!(
// 				c,
// 				Call::System(..) |
// 				Call::Babe(..) |
// 				Call::Timestamp(..) |
// 				Call::Indices(pallet_indices::Call::claim {..}) |
// 				Call::Indices(pallet_indices::Call::free {..}) |
// 				Call::Indices(pallet_indices::Call::freeze {..}) |
// 				// Specifically omitting Indices `transfer`, `force_transfer`
// 				// Specifically omitting the entire Balances pallet
// 				Call::Authorship(..) |
// 				Call::Staking(..) |
// 				// Call::Session(..) |
// 				Call::Grandpa(..) |
// 				Call::ImOnline(..) |
// 				Call::Democracy(..) |
// 				Call::Council(..) |
// 				Call::TechnicalCommittee(..) |
// 				Call::PhragmenElection(..) |
// 				Call::TechnicalMembership(..) |
// 				Call::Treasury(..) |
// 				Call::Bounties(..) |
// 				Call::ChildBounties(..) |
// 				Call::Tips(..) |
// 				Call::Claims(..) |
// 				Call::Utility(..) |
// 				Call::Identity(..) |
// 				Call::Society(..) |
// 				Call::Recovery(pallet_recovery::Call::as_recovered {..}) |
// 				Call::Recovery(pallet_recovery::Call::vouch_recovery {..}) |
// 				Call::Recovery(pallet_recovery::Call::claim_recovery {..}) |
// 				Call::Recovery(pallet_recovery::Call::close_recovery {..}) |
// 				Call::Recovery(pallet_recovery::Call::remove_recovery {..}) |
// 				Call::Recovery(pallet_recovery::Call::cancel_recovered {..}) |
// 				// Specifically omitting Recovery `create_recovery`, `initiate_recovery`
// 				Call::Vesting(pallet_vesting::Call::vest {..}) |
// 				Call::Vesting(pallet_vesting::Call::vest_other {..}) |
// 				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
// 				Call::Scheduler(..) |
// 				Call::Proxy(..) |
// 				Call::Multisig(..) |
// 				Call::Gilt(..) |
// 				//Call::Registrar(paras_registrar::Call::register {..}) |
// 				//Call::Registrar(paras_registrar::Call::deregister {..}) |
// 				// Specifically omitting Registrar `swap`
// 				//Call::Registrar(paras_registrar::Call::reserve {..}) |
// 				Call::Crowdloan(..) |
// 				// Call::Slots(..) |
// 				Call::Auctions(..) | // Specifically omitting the entire XCM Pallet
// 				Call::BagsList(..)
// 			),
// 			ProxyType::Governance => matches!(
// 				c,
// 				Call::Democracy(..) |
// 					Call::Council(..) | Call::TechnicalCommittee(..) |
// 					Call::PhragmenElection(..) |
// 					Call::Treasury(..) | Call::Bounties(..) |
// 					Call::Tips(..) | Call::Utility(..) |
// 					Call::ChildBounties(..)
// 			),
// 			ProxyType::Staking => {
// 				matches!(c, Call::Staking(..) | Call::Session(..) | Call::Utility(..))
// 			},
// 			ProxyType::IdentityJudgement => matches!(
// 				c,
// 				Call::Identity(pallet_identity::Call::provide_judgement { .. }) | Call::Utility(..)
// 			),
// 			ProxyType::CancelProxy => {
// 				matches!(c, Call::Proxy(pallet_proxy::Call::reject_announcement { .. }))
// 			},
// 			ProxyType::Auction => matches!(
// 				c,
// 				Call::Auctions(..) | Call::Crowdloan(..) | Call::Registrar(..) | Call::Slots(..)
// 			),
// 			ProxyType::Society => matches!(c, Call::Society(..)),
// 		}
// 	}
// 	fn is_superset(&self, o: &Self) -> bool {
// 		match (self, o) {
// 			(x, y) if x == y => true,
// 			(ProxyType::Any, _) => true,
// 			(_, ProxyType::Any) => false,
// 			(ProxyType::NonTransfer, _) => true,
// 			_ => false,
// 		}
// 	}
// }
//
// impl pallet_proxy::Config for Runtime {
// 	type Event = Event;
// 	type Call = Call;
// 	type Currency = Balances;
// 	type ProxyType = ProxyType;
// 	type ProxyDepositBase = ProxyDepositBase;
// 	type ProxyDepositFactor = ProxyDepositFactor;
// 	type MaxProxies = MaxProxies;
// 	type WeightInfo = (); // weights::pallet_proxy::WeightInfo<Runtime>;
// 	type MaxPending = MaxPending;
// 	type CallHasher = BlakeTwo256;
// 	type AnnouncementDepositBase = AnnouncementDepositBase;
// 	type AnnouncementDepositFactor = AnnouncementDepositFactor;
// }

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

type ScheduleOrigin = EnsureOneOf<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<cmp::Ordering> {
		if left == right {
			return Some(cmp::Ordering::Equal)
		}

		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(cmp::Ordering::Greater),
			// Check which one has more yes votes.
			(
				OriginCaller::Council(pallet_collective::RawOrigin::Members(l_yes_votes, l_count)),
				OriginCaller::Council(pallet_collective::RawOrigin::Members(r_yes_votes, r_count)),
			) => Some((l_yes_votes * r_count).cmp(&(r_yes_votes * l_count))),
			// For every other origin we don't care, as they are not used for `ScheduleOrigin`.
			_ => None,
		}
	}
}

impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = ScheduleOrigin;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = (); // weights::pallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type PreimageProvider = Preimage;
	type NoPreimagePostponement = NoPreimagePostponement;
}

// parameter_types! {
// 	/// Authorities are changing every 5 minutes.
// 	pub const Period: BlockNumber = bp_millau::SESSION_LENGTH;
// 	pub const Offset: BlockNumber = 0;
// }
//
// impl pallet_session::Config for Runtime {
// 	type Event = Event;
// 	type ValidatorId = <Self as frame_system::Config>::AccountId;
// 	type ValidatorIdOf = ();
// 	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
// 	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
// 	type SessionManager = pallet_shift_session_manager::Pallet<Runtime>;
// 	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
// 	type Keys = SessionKeys;
// 	// TODO: update me (https://github.com/paritytech/parity-bridges-common/issues/78)
// 	type WeightInfo = ();
// }

// impl pallet_tips::Config for Runtime {
// 	type MaximumReasonLength = MaximumReasonLength;
// 	type DataDepositPerByte = DataDepositPerByte;
// 	type Tippers = PhragmenElection;
// 	type TipCountdown = TipCountdown;
// 	type TipFindersFee = TipFindersFee;
// 	type TipReportDepositBase = TipReportDepositBase;
// 	type Event = Event;
// 	type WeightInfo = (); // weights::pallet_tips::WeightInfo<Runtime>;
// }

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = ();
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = ();
}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
		RuntimeBlockWeights::get().max_block;
	// The weight needed for decoding the queue should be less or equal than a fifth
	// of the overall weight dedicated to the lazy deletion.
	pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
			<Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
		)) / 5) as u32;
	pub Schedule: pallet_contracts::Schedule<Runtime> = {
		let mut schedule = pallet_contracts::Schedule::<Runtime>::default();
		// We decided to **temporarily* increase the default allowed contract size here
		// (the default is `128 * 1024`).
		//
		// Our reasoning is that a number of people ran into `CodeTooLarge` when trying
		// to deploy their contracts. We are currently introducing a number of optimizations
		// into ink! which should bring the contract sizes lower. In the meantime we don't
		// want to pose additional friction on developers.
		schedule.limits.code_len = 256 * 1024;
		schedule
	};
}

impl pallet_contracts::Config for Runtime {
	type Event = Event;
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type Call = Call;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = frame_support::traits::Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * CENTS;
}


construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 3,
		Rewards: rewards::{Pallet, Call, Storage, Event<T>, Config<T>} = 4,

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 5,
		Difficulty: difficulty::{Pallet, Call, Storage, Config} = 6,
		// Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 8,
		// ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>} = 11,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>, Config},
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>},
		ChildBounties: pallet_child_bounties::{Pallet, Call, Storage, Event<T>},
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},

		// Less simple identity module.
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 25,

		// Multisig module. Late addition.
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 31,
		// Preimage registrar.
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 32,

		// System scheduler.
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 29,


		// Tips module.
		// Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 36,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 28,

		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 30,

		Authorship: pallet_authorship::{Pallet, Call, Storage, Event<T>, Config<T>} = 34,
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>, Config<T>} = 35,

		PoScan: pallet_poscan::{Pallet, Call, Storage, Event<T>} = 40,
		// PoScan: pallet_poscan::{Module},
	}
);

impl pallet_vesting::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = (); // weights::pallet_vesting::WeightInfo<Runtime>;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

// parameter_types! {
// 	pub NposSolutionPriority: TransactionPriority =
// 		Perbill::from_percent(90) * TransactionPriority::max_value();
// 	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
// }
//
// impl pallet_im_online::Config for Runtime {
// 	type AuthorityId = ImOnlineId;
// 	type Event = Event;
// 	type ValidatorSet = Historical;
// 	type NextSessionRotation = Babe;
// 	type ReportUnresponsiveness = Offences;
// 	type UnsignedPriority = ImOnlineUnsignedPriority;
// 	type WeightInfo = (); // weights::pallet_im_online::WeightInfo<Runtime>;
// 	type MaxKeys = MaxKeys;
// 	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
// 	type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
// }

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various pallets.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	(),
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		// fn random_seed() -> <Block as BlockT>::Hash {
		// 	RandomnessCollectiveFlip::random_seed()
		// }
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_finality_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_finality_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_finality_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: sp_finality_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_finality_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_finality_grandpa::OpaqueKeyOwnershipProof> {
			None
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	// impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>
	// 	for Runtime
	// {
	// 	fn call(
	// 		origin: AccountId,
	// 		dest: AccountId,
	// 		value: Balance,
	// 		gas_limit: u64,
	// 		storage_deposit_limit: Option<Balance>,
	// 		input_data: Vec<u8>,
	// 	) -> pallet_contracts_primitives::ContractExecResult<Balance> {
	// 		Contracts::bare_call(origin, dest, value, gas_limit, storage_deposit_limit, input_data, CONTRACTS_DEBUG_OUTPUT)
	// 	}
	//
	// 	fn instantiate(
	// 		origin: AccountId,
	// 		value: Balance,
	// 		gas_limit: u64,
	// 		storage_deposit_limit: Option<Balance>,
	// 		code: pallet_contracts_primitives::Code<Hash>,
	// 		data: Vec<u8>,
	// 		salt: Vec<u8>,
	// 	) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance>
	// 	{
	// 		Contracts::bare_instantiate(origin, value, gas_limit, storage_deposit_limit, code, data, salt, CONTRACTS_DEBUG_OUTPUT)
	// 	}
	//
	// 	fn upload_code(
	// 		origin: AccountId,
	// 		code: Vec<u8>,
	// 		storage_deposit_limit: Option<Balance>,
	// 	) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
	// 	{
	// 		Contracts::bare_upload_code(origin, code, storage_deposit_limit)
	// 	}
	//
	// 	fn get_storage(
	// 		address: AccountId,
	// 		key: Vec<u8>,
	// 	) -> pallet_contracts_primitives::GetStorageResult {
	// 		Contracts::get_storage(address, key)
	// 	}
	// }

	impl sp_consensus_poscan::TimestampApi<Block, u64> for Runtime {
		fn timestamp() -> u64 {
			pallet_timestamp::Pallet::<Runtime>::get()
		}
	}

	impl sp_consensus_poscan::DifficultyApi<Block, sp_consensus_poscan::Difficulty> for Runtime {
		fn difficulty() -> sp_consensus_poscan::Difficulty {
			difficulty::Module::<Runtime>::difficulty()
		}
	}

}
