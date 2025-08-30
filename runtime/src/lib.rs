#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "512"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod fee;
mod check;
mod algo;

// Include the genesis helper module when building to std
#[cfg(feature = "std")]
pub mod genesis;
mod weights;

use core::convert::TryInto;
use log;
use static_assertions::const_assert;

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256, H160};
use sp_std::{cmp, collections::btree_map::BTreeMap, prelude::*};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill, Percent};

// Frontier
use fp_rpc::TransactionStatus;
use pallet_ethereum::{Call::transact, Transaction as EthereumTransaction};
use pallet_evm::{
	Account as EVMAccount, EnsureAddressTruncated, FeeCalculator, HashedAddressMapping, Runner,
};

use pallet_evm_precompileset_assets_erc20::AccountIdAssetIdConversion;

use frame_support::{
	construct_runtime, parameter_types, ord_parameter_types,
	traits::{
		AsEnsureOriginWithArg, ChangeMembers, ConstU128, ConstU16, ConstU32, ConstU64, Currency,
		EitherOfDiverse, EqualPrivilegeOnly, InitializeMembers, KeyOwnerProofSystem,
		LockIdentifier, OnRuntimeUpgrade, OnUnbalanced, U128CurrencyToVote, InstanceFilter,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		ConstantMultiplier, DispatchClass, Weight,
	},
	PalletId,
	ConsensusEngineId,
};

use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureSignedBy,
};

use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, ConvertInto, IdentifyAccount,
		Identity as IdentityTrait, NumberFor, OpaqueKeys, Verify,
		Extrinsic, SaturatedConversion, StaticLookup, AccountIdConversion,
		Get, UniqueSaturatedInto, DispatchInfoOf, PostDispatchInfoOf, Dispatchable
	},
	transaction_validity::{
		TransactionPriority, TransactionSource, TransactionValidity, TransactionValidityError
	},
	ApplyExtrinsicResult, MultiSignature,
};

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
use pallet_contracts::{migration, DefaultContractAccessWeight};
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::CurrencyAdapter;
// use pallet_mining_pool::crypto::PoolAuthorityId;
use pallet_mining_pool::sr25519::PoolAuthorityId;

use pallet_asset_conversion::{NativeOrAssetId, NativeOrAssetIdConverter};

use sp_consensus_poscan::{
	deposit, BLOCK_TIME, CENTS, DAYS, DOLLARS, HOURS, MICROCENTS, MILLICENTS, MINUTES,
	POSCAN_COIN_ID, POSCAN_ENGINE_ID,
};

use mining_pool_stat_api::{MiningPoolStatApi, CheckMemberError};
use poscan_api::PoscanApi;
use serial_numbers_api;

mod precompiles;
use precompiles::FrontierPrecompiles;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// AssetId type
pub type AssetId = u32;

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
/// to even the core data structures.
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
		pub struct OldSessionKeys {
			pub grandpa: Grandpa,
		}
	}

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub grandpa: Grandpa,
			pub imonline: ImOnline,
		}
	}
}
// To learn more about runtime versioning and what each of the following value means:
//   https://docs.substrate.io/v3/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("poscan-runtime"),
	impl_name: create_runtime_str!("poscan-runtime"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 131,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

type MoreThanHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

/// We allow for 20 seconds of compute with a 60 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 20 * WEIGHT_PER_SECOND;
const WEIGHT_PER_GAS: u64 = 20_000;

// Prints debug output of the `contracts` pallet to stdout if the node is
// started with `-lruntime::contracts=debug`.
const CONTRACTS_DEBUG_OUTPUT: bool = true;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();

	pub const SS58Prefix: u8 = POSCAN_COIN_ID;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, AccountIndex>;
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
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

pub struct FindBlockAuthor;
impl FindAuthor<AccountId> for FindBlockAuthor {
	fn find_author<'a, I>(digests: I) -> Option<AccountId>
	where I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])> {
		digests.into_iter().filter_map(|(id, mut data)| {
			if id == POSCAN_ENGINE_ID {
				AccountId::decode(&mut data).ok()
			} else {
				None
			}
		})
		.next()
	}
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = FindBlockAuthor;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = ();
}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Difficulty;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
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
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
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
	pub const TransactionByteFee: Balance = MICROCENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type Event = Event;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = fee::WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = (); // SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	// Retry a scheduled item every 10 blocks (1 minute) until the preimage exists.
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PreimageProvider = Preimage;
	type NoPreimagePostponement = NoPreimagePostponement;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 1 * CENTS;
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Event = Event;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type MaxSize = PreimageMaxSize;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	pub const IndexDeposit: Balance = 100 * CENTS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TargetBlockTime: u64 = BLOCK_TIME;
}

impl difficulty::Config for Runtime {
	type TargetBlockTime = TargetBlockTime;
}

//------------------- rewards
const TESTNET_LAST_BLOCK: u32 = 106390;
const REWARDS_STEP: usize = 243000;
const MAX_REWARS_IDX: usize = 90;
const REWARDS: [u128; MAX_REWARS_IDX] = [
	50_000_000 * MILLICENTS, 41_666_670 * MILLICENTS, 34_722_220 * MILLICENTS,
	28_935_190 * MILLICENTS, 24_112_650 * MILLICENTS, 20_093_880 * MILLICENTS,
	16_744_900 * MILLICENTS, 13_954_080 * MILLICENTS, 11_628_400 * MILLICENTS,
	09_690_330 * MILLICENTS, 08_075_280 * MILLICENTS, 06_729_400 * MILLICENTS,
	05_607_830 * MILLICENTS, 04_673_190 * MILLICENTS, 03_894_330 * MILLICENTS,
	03_245_270 * MILLICENTS, 02_704_390 * MILLICENTS, 02_253_660 * MILLICENTS,
	01_878_050 * MILLICENTS, 01_565_040 * MILLICENTS, 01_304_200 * MILLICENTS,
	01_086_840 * MILLICENTS, 00_905_700 * MILLICENTS, 00_754_750 * MILLICENTS,
	00_628_960 * MILLICENTS, 00_524_130 * MILLICENTS, 00_436_770 * MILLICENTS,
	00_363_980 * MILLICENTS, 00_303_320 * MILLICENTS, 00_252_760 * MILLICENTS,
	00_210_640 * MILLICENTS, 00_175_530 * MILLICENTS, 00_146_280 * MILLICENTS,
	00_121_900 * MILLICENTS, 00_101_580 * MILLICENTS, 00_084_650 * MILLICENTS,
	00_070_540 * MILLICENTS, 00_058_780 * MILLICENTS, 00_048_990 * MILLICENTS,
	00_040_820 * MILLICENTS, 00_034_020 * MILLICENTS, 00_028_350 * MILLICENTS,
	00_023_620 * MILLICENTS, 00_019_690 * MILLICENTS, 00_016_410 * MILLICENTS,
	00_013_670 * MILLICENTS, 00_011_390 * MILLICENTS, 00_009_490 * MILLICENTS,
	00_007_910 * MILLICENTS, 00_006_590 * MILLICENTS, 00_005_490 * MILLICENTS,
	00_004_580 * MILLICENTS, 00_003_820 * MILLICENTS, 00_003_180 * MILLICENTS,
	00_002_650 * MILLICENTS, 00_002_210 * MILLICENTS, 00_001_840 * MILLICENTS,
	00_001_530 * MILLICENTS, 00_001_280 * MILLICENTS, 00_001_060 * MILLICENTS,
	00_000_890 * MILLICENTS, 00_000_740 * MILLICENTS, 00_000_620 * MILLICENTS,
	00_000_510 * MILLICENTS, 00_000_430 * MILLICENTS, 00_000_360 * MILLICENTS,
	00_000_300 * MILLICENTS, 00_000_250 * MILLICENTS, 00_000_210 * MILLICENTS,
	00_000_170 * MILLICENTS, 00_000_140 * MILLICENTS, 00_000_120 * MILLICENTS,
	00_000_100 * MILLICENTS, 00_000_080 * MILLICENTS, 00_000_070 * MILLICENTS,
	00_000_060 * MILLICENTS, 00_000_050 * MILLICENTS, 00_000_040 * MILLICENTS,
	00_000_030 * MILLICENTS, 00_000_030 * MILLICENTS, 00_000_020 * MILLICENTS,
	00_000_020 * MILLICENTS, 00_000_020 * MILLICENTS, 00_000_010 * MILLICENTS,
	00_000_010 * MILLICENTS, 00_000_010 * MILLICENTS, 00_000_010 * MILLICENTS,
	00_000_010 * MILLICENTS, 00_000_010 * MILLICENTS, 00_000_000 * MILLICENTS,
];

pub struct GenerateRewardLocks;

impl rewards::GenerateRewardLocks<Runtime> for GenerateRewardLocks {
	fn generate_reward_locks(
		current_block: BlockNumber,
		total_reward: Balance,
		lock_parameters: Option<rewards::LockParameters>,
	) -> BTreeMap<BlockNumber, Balance> {
		let mut locks = BTreeMap::new();
		let locked_reward = total_reward.saturating_sub(1 * DOLLARS / 1000);

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
		let idx: usize = ((current_block + TESTNET_LAST_BLOCK) / b)
			.try_into()
			.unwrap();
		if idx >= MAX_REWARS_IDX {
			return 0;
		}
		REWARDS[idx].clone()
	}
}

parameter_types! {
	pub DonationDestination: AccountId = Treasury::account_id();
	pub const LockBounds: rewards::LockBounds = rewards::LockBounds {period_max: 500, period_min: 20,
																	divide_max: 50, divide_min: 2};
	pub const MinerRewardsPercent: Percent = Percent::from_percent(70);
	pub const MiningPoolMaxRate: Percent = Percent::from_percent(20);
}

impl rewards::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type DonationDestination = DonationDestination;
	type GenerateRewardLocks = GenerateRewardLocks;
	type WeightInfo = crate::weights::rewards::WeightInfo<Self>;
	type LockParametersBounds = LockBounds;
	type ValidatorSet = ValidatorSet;
	type MinerRewardsPercent = MinerRewardsPercent;
	type MiningPool = MiningPool;
	type MiningPoolMaxRate = MiningPoolMaxRate;
	type MinerShareOrigin = EnsureRootOrHalfCouncil;
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
	// Minimum 100 bytes/P3D deposited (1 CENT/byte)
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
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(0);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 500 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
}

type ApproveOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

pub struct SpendOrigin;
impl frame_support::traits::EnsureOrigin<Origin> for SpendOrigin {
	type Success = u128;
	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Result::<frame_system::RawOrigin<_>, Origin>::from(o).and_then(|o| match o {
			frame_system::RawOrigin::Root => Ok(u128::MAX),
			// frame_system::RawOrigin::Signed(AccountId::from(10)) => Ok(5),
			// frame_system::RawOrigin::Signed(AccountId::from(11)) => Ok(10),
			// frame_system::RawOrigin::Signed(AccountId::from(12)) => Ok(20),
			// frame_system::RawOrigin::Signed(AccountId::from(13)) => Ok(50),
			r => Err(Origin::from(r)),
		})
	}
}

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
	type BurnDestination = Treasury;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type SpendFunds = Bounties;
	type SpendOrigin = SpendOrigin;
}

impl pallet_remark::Config for Runtime {
	type WeightInfo = pallet_remark::weights::SubstrateWeight<Self>;
	type Event = Event;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 7 * 24 * 60 * MINUTES;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
}

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 30 * DAYS;
}

impl pallet_conviction_voting::Config for Runtime {
	type WeightInfo = pallet_conviction_voting::weights::SubstrateWeight<Self>;
	type Event = Event;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout = frame_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 100 * DOLLARS;
	pub const UndecidingTimeout: BlockNumber = 28 * DAYS;

	pub const MaxQueued: u32 = 100;
	pub const Votes: u32 = 100;
}

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type Origin = <Origin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		static DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 1] = [(
			0u16,
			pallet_referenda::TrackInfo {
				name: "root",
				max_deciding: 1,
				decision_deposit: 100 * 1000 * DOLLARS,
				prepare_period: 2 * HOURS,
				decision_period: 28 * DAYS,
				confirm_period: 24 * HOURS,
				min_enactment_period: 24 * HOURS,
				min_approval: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(50),
					ceil: Perbill::from_percent(100),
				},
				min_support: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(0),
					ceil: Perbill::from_percent(100),
				},
			},
		)];
		&DATA[..]
	}
	fn track_for(id: &Self::Origin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
				frame_system::RawOrigin::Root => Ok(0),
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

impl pallet_referenda::Config for Runtime {
	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
	type Call = Call;
	type Event = Event;
	/// Weight information for extrinsics in this pallet.
	/// The Scheduler.
	type Scheduler = Scheduler;
	/// Currency type for this pallet.

	type Currency = pallet_balances::Pallet<Self>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;

	type Votes = pallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pallet_conviction_voting::TallyOf<Runtime>;

	/// Handler for the unbalanced reduction when slashing a preimage deposit.
	type Slash = ();
	type SubmissionDeposit = SubmissionDeposit;
	/// Maximum size of the referendum queue for a single track.
	type MaxQueued = MaxQueued;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	/// Information concerning the different referendum tracks.
	type Tracks = TracksInfo;
}

impl pallet_referenda::Config<pallet_referenda::Instance2> for Runtime {
	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
	type Call = Call;
	type Event = Event;
	type Scheduler = Scheduler;
	type Currency = pallet_balances::Pallet<Self>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Slash = ();
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
}

impl pallet_ranked_collective::Config for Runtime {
	type WeightInfo = pallet_ranked_collective::weights::SubstrateWeight<Self>;
	type Event = Event;
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type DemoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type Polls = RankedPolls;
	type MinRankOfClass = IdentityTrait;
	type VoteWeight = pallet_ranked_collective::Geometric;
}

parameter_types! {
	pub const MaxMembers: u32 = 100;
}

impl pallet_membership::Config for Runtime {
	type Event = Event;
	type AddOrigin = EnsureRootOrHalfCouncil;
	type RemoveOrigin = EnsureRootOrHalfCouncil;
	type SwapOrigin = EnsureRootOrHalfCouncil;
	type ResetOrigin = EnsureRootOrHalfCouncil;
	type PrimeOrigin = EnsureRootOrHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CandidacyBond: Balance = 100 * DOLLARS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	/// Weekly council elections; scaling up to monthly eventually.
	pub TermDuration: BlockNumber = 7 * DAYS; // prod_or_fast!(7 * DAYS, 2 * MINUTES, "DOT_TERM_DURATION");
	/// 13 members initially, to be increased to 23 eventually.
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 20;
	pub const MaxVoters: u32 = 10 * 1000;
	pub const MaxCandidates: u32 = 1000;
	pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
}
// Make sure that there are no more than `MaxMembers` members elected via phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type Event = Event;
	type PalletId = PhragmenElectionPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	type InitializeMembers = Council;
	type CurrencyToVote = U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
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
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
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
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU16<100>;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

impl pallet_transaction_storage::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type Call = Call;
	type FeeDestination = ();
	type WeightInfo = pallet_transaction_storage::weights::SubstrateWeight<Runtime>;
	type MaxBlockTransactions =
		ConstU32<{ pallet_transaction_storage::DEFAULT_MAX_BLOCK_TRANSACTIONS }>;
	type MaxTransactionSize =
		ConstU32<{ pallet_transaction_storage::DEFAULT_MAX_TRANSACTION_SIZE }>;
}

// TODO: setup pallet_scored_pool
pub struct TestChangeMembers;
impl ChangeMembers<AccountId> for TestChangeMembers {
	fn change_members_sorted(_incoming: &[AccountId], _outgoing: &[AccountId], _new: &[AccountId]) {
		// let mut old_plus_incoming = MEMBERS.with(|m| m.borrow().to_vec());
		// old_plus_incoming.extend_from_slice(incoming);
		// old_plus_incoming.sort();
		//
		// let mut new_plus_outgoing = new.to_vec();
		// new_plus_outgoing.extend_from_slice(outgoing);
		// new_plus_outgoing.sort();
		//
		// assert_eq!(old_plus_incoming, new_plus_outgoing);
		//
		// MEMBERS.with(|m| *m.borrow_mut() = new.to_vec());
	}
}

impl InitializeMembers<AccountId> for TestChangeMembers {
	fn initialize_members(_new_members: &[AccountId]) {
		// MEMBERS.with(|m| *m.borrow_mut() = new_members.to_vec());
	}
}

parameter_types! {
	pub const CandidateDeposit: u64 = 25;
	// pub BlockWeights: frame_system::limits::BlockWeights =
	// 	frame_system::limits::BlockWeights::simple_max(1024);
}

impl pallet_scored_pool::Config for Runtime {
	type Event = Event;
	type KickOrigin = EnsureSigned<AccountId>;
	type MembershipInitialized = TestChangeMembers;
	type MembershipChanged = TestChangeMembers;
	type Currency = Balances;
	type CandidateDeposit = CandidateDeposit;
	type Period = ConstU32<4>;
	type Score = u64;
	type ScoreOrigin = EnsureSigned<AccountId>;
}
type EnsureRootOrHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

const LEVELS: [(u128, u32); 4] = [
	(100 * 1000 * DOLLARS, 100),
	(200 * 1000 * DOLLARS, 2000),
	(300 * 1000 * DOLLARS, 4000),
	(400 * 1000 * DOLLARS, 8000),
];

parameter_types! {
	pub const MinAuthorities: u32 = 3;
	pub const PoscanEngineId: [u8;4] = POSCAN_ENGINE_ID;
	pub const FilterLevels: [(u128, u32);4] = LEVELS;
	pub const MaxMinerDepth: u32 = 10000;
	pub const PenaltyOffline: u128 = 20_000 * DOLLARS;
	pub const MinLockAmount: u128 = 100_000 * DOLLARS;
	pub const MinLockPeriod: u32 = 30 * 24 * HOURS;
	pub const SlashValidatorFor: u32 = 2 * HOURS;
	pub const AddAfterSlashPeriod: u32 = 7 * 24 * HOURS;
	pub const MaxVal: u32 = 1000;
	pub const EnterFee: u128 = 10_000 * DOLLARS;
}

impl pallet_validator_set::Config for Runtime {
	type Event = Event;
	type AuthorityId = ImOnlineId;
	type EstimateUnsignedPriority = ConstU64<{TransactionPriority::MAX / 2}>;
	type EstimatePriority = ConstU64<{TransactionPriority::MAX / 2}>;
	type AddRemoveOrigin = EnsureRootOrHalfCouncil;
	// type AddRemoveOrigin = EnsureRoot<AccountId>;
	type MinAuthorities = MinAuthorities;
	type Currency = Balances;
	type PoscanEngineId = PoscanEngineId;
	type FilterLevels = FilterLevels;
	type MaxMinerDepth = MaxMinerDepth;
	type RewardLocksApi = Rewards;
	type PenaltyOffline = PenaltyOffline;
	type MinLockAmount = MinLockAmount;
	type MinLockPeriod = MinLockPeriod;
	type SlashValidatorFor = SlashValidatorFor;
	type AddAfterSlashPeriod = AddAfterSlashPeriod;
	type Slash = Treasury;
	type DefaultOffset = Offset;
	type DefaultPeriod = Period;
	type MaxVal = MaxVal;
	type EnterFee = EnterFee;
}
use sp_core::U256;

parameter_types! {
	pub const MaxPoolPercent: Percent = Percent::from_percent(50);
}

impl pallet_mining_pool::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type AddRemoveOrigin = EnsureRootOrHalfCouncil;
	// type AddRemoveOrigin = EnsureRoot<AccountId>;
	type MaxKeys = ConstU32<1000>;
	type MaxPools = ConstU32<100>;
	type MaxMembers = ConstU32<1000>;
	type Currency = Balances;
	type PoscanEngineId = PoscanEngineId;
	type RewardLocksApi = Rewards;
	type Difficulty = U256;
	type PoolAuthorityId = PoolAuthorityId;
	type UnsignedPriority = ConstU64<{TransactionPriority::max_value() / 2}>;
	type StatPeriod = ConstU32<10>;
	type MaxPoolPercent = MaxPoolPercent;
	// type PoolAuthorityId = AccountId; // pallet_mining_pool::crypto::PoolAuthorityId;
}

parameter_types! {
	pub const Period: u32 = 30 * MINUTES;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_validator_set::ValidatorOf<Self>;
	type ShouldEndSession = ValidatorSet;
	type NextSessionRotation = ValidatorSet;
	type SessionManager = ValidatorSet;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
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
	pub const DataDepositPerByte: Balance = 1 * CENTS;
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
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// pub const AssetDeposit: Balance = 100 * DOLLARS;
	// pub const ApprovalDeposit: Balance = 1 * DOLLARS;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}
//
// impl pallet_assets::Config for Runtime {
// 	type Event = Event;
// 	type Balance = u128;
// 	type AssetId = u32;
// 	type Currency = Balances;
// 	type ForceOrigin = EnsureRoot<AccountId>;
// 	type AssetDeposit = AssetDeposit;
// 	type AssetAccountDeposit = ConstU128<DOLLARS>;
// 	type MetadataDepositBase = MetadataDepositBase;
// 	type MetadataDepositPerByte = MetadataDepositPerByte;
// 	type ApprovalDeposit = ApprovalDeposit;
// 	type StringLimit = StringLimit;
// 	type Freezer = ();
// 	type Extra = ();
// 	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
// }

parameter_types! {
	pub const CollectionDeposit: Balance = 100 * DOLLARS;
	pub const ItemDeposit: Balance = 1 * DOLLARS;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 256;
}

impl pallet_uniques::Config for Runtime {
	type Event = Event;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type WeightInfo = pallet_uniques::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub const MaxActiveChildBountyCount: u32 = 100;
	pub const ChildBountyValueMinimum: Balance = BountyValueMinimum::get() / 10;
}

impl pallet_child_bounties::Config for Runtime {
	type Event = Event;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const ReportLongevity: u64 = 120;
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type KeyOwnerProofSystem = ValidatorSet;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	type HandleEquivocation = pallet_grandpa::EquivocationHandler<
		Self::KeyOwnerIdentification,
		Offences,
		ReportLongevity,
	>;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
}


impl pallet_offences::Config for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_validator_set::IdentificationTuple::<Runtime>;
	type OnOffenceHandler = ValidatorSet;
}

parameter_types! {
	pub const NposSolutionPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type ValidatorSet = ValidatorSet;
	type NextSessionRotation = ValidatorSet;
	type ReportUnresponsiveness = ValidatorSet; // Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * CENTS;
}

impl pallet_vesting::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pallet_whitelist::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WhitelistOrigin = EnsureRoot<AccountId>;
	type DispatchWhitelistedOrigin = EnsureRoot<AccountId>;
	type PreimageProvider = Preimage;
	type WeightInfo = pallet_whitelist::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const DeletionQueueDepth: u32 = 128;
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = RuntimeBlockWeights::get()
		.per_class
		.get(DispatchClass::Normal)
		.max_total
		.unwrap_or(RuntimeBlockWeights::get().max_block);
	pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type Event = Event;
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
	type CallStack = [pallet_contracts::Frame<Self>; 31];
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	type ChainExtension = ();
	type DeletionQueueDepth = DeletionQueueDepth;
	type DeletionWeightLimit = DeletionWeightLimit;
	type Schedule = Schedule;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type ContractAccessWeight = DefaultContractAccessWeight<RuntimeBlockWeights>;
	// This node is geared towards development and testing of contracts.
	// We decided to increase the default allowed contract size for this
	// reason (the default is `128 * 1024`).
	//
	// Our reasoning is that the error code `CodeTooLarge` is thrown
	// if a too-large contract is uploaded. We noticed that it poses
	// less friction during development when the requirement here is
	// just more lax.
	type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
	type RelaxedMaxCodeLen = ConstU32<{ 512 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

use codec::Encode;
use frame_support::instances::{Instance1, Instance2};
use frame_support::traits::FindAuthor;

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
	where
		Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		let _tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u32>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		// let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(
				period,
				current_block.saturated_into(),
			)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl pallet_atomic_swap::Config<Instance1> for Runtime {
	type Event = Event;
	type SwapAction = pallet_atomic_swap::BalanceSwapAction<AccountId, Balances>;
	type ProofLimit = ConstU32<1024>;
}

impl pallet_atomic_swap::Config<Instance2> for Runtime {
	type Event = Event;
	type SwapAction = pallet_poscan_assets::swap::TokenSwapAction<
		Self,
		Instance1,
		AccountId,
		<Self as pallet_poscan_assets::Config<Instance1>>::AssetId,
	>;
	type ProofLimit = ConstU32<1024>;
}

impl pallet_asset_rate::Config for Runtime {
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = u32;
	type Event = Event;
	type WeightInfo = pallet_asset_rate::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub AllowMultiAssetPools: bool = true;
	pub const PoolSetupFee: Balance = 1 * DOLLARS; // should be more or equal to the existential deposit
	pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency has 10-12 decimals.
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);  // should be non-zero if AllowMultiAssetPools is true, otherwise can be zero.
}

ord_parameter_types! {
	pub const AssetConversionOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&AssetConversionPalletId::get());
}

impl pallet_asset_conversion::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type AssetBalance = <Self as pallet_balances::Config>::Balance;
	type HigherPrecisionBalance = u128;
	type Assets = PoscanAssets;
	type Balance = u128;
	type PoolAssets = PoscanPoolAssets;
	type AssetId = <Self as pallet_poscan_assets::Config<Instance1>>::AssetId;
	type MultiAssetId = NativeOrAssetId<u32>;
	type PoolAssetId = <Self as pallet_poscan_assets::Config<Instance2>>::AssetId;
	type PalletId = AssetConversionPalletId;
	type LPFee = ConstU32<3>; // means 0.3%
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeReceiver = AssetConversionOrigin;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type WeightInfo = pallet_asset_conversion::weights::SubstrateWeight<Runtime>;
	type AllowMultiAssetPools = AllowMultiAssetPools;
	type MaxSwapPathLength = ConstU32<4>;
	type MintMinLiquidity = MintMinLiquidity;
	type MultiAssetIdConverter = NativeOrAssetIdConverter<u32>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const RewardsDefault: u128 = 500 * DOLLARS;
	pub const AuthorPartDefault: Percent = Percent::from_percent(30);
	pub const EstimatorsShareDefault: Percent = Percent::from_percent(70);
	pub const DynamicRewardsGrowthRate: u32 = 10; // Controls exponential growth rate (higher = slower growth)
}

impl pallet_poscan::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type ValidatorSet = ValidatorSet;
	type PoscanEngineId = PoscanEngineId;
	type AdminOrigin = MoreThanHalfCouncil;
	type EstimatePeriod = ConstU32<3>;
	type ApproveTimeout = ConstU32<5>;
	// type MaxObjectSize = ConstU32<100_000>;
	type RewardsDefault = RewardsDefault;
	type AuthorPartDefault = AuthorPartDefault;
	type DynamicRewardsGrowthRate = DynamicRewardsGrowthRate;
	type Currency = Balances;
	type SerialNumbers = Runtime;
	type PoscanAssets = pallet_poscan_assets::Pallet<Runtime, Instance1>;
	type CouncilOrigin = EnsureRootOrHalfCouncil;
}

parameter_types! {
	pub const PosAssetDeposit: Balance = 100 * DOLLARS;
	pub const PosApprovalDeposit: Balance = 1 * DOLLARS;
	pub const PosStringLimit: u32 = 50;
	pub const PosMetadataDepositBase: Balance = 10 * DOLLARS;
	pub const PosMetadataDepositPerByte: Balance = 1 * DOLLARS;
}

impl pallet_poscan_assets::Config<Instance1> for Runtime {
	type Event = Event;
	type Balance = u128;
	type AssetId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type Poscan = pallet_poscan::Pallet::<Self>;
	type AssetDeposit = PosAssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = PosMetadataDepositBase;
	type MetadataDepositPerByte = PosMetadataDepositPerByte;
	type ApprovalDeposit = PosApprovalDeposit;
	type StringLimit = PosStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pallet_poscan_assets::weights::SubstrateWeight<Runtime>;
}

impl pallet_poscan_assets::Config<Instance2> for Runtime {
	type Event = Event;
	type Balance = u128;
	type AssetId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type Poscan = pallet_poscan::Pallet::<Self>;
	type AssetDeposit = PosAssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = PosMetadataDepositBase;
	type MetadataDepositPerByte = PosMetadataDepositPerByte;
	type ApprovalDeposit = PosApprovalDeposit;
	type StringLimit = PosStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pallet_poscan_assets::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

impl pallet_evm_chain_id::Config for Runtime {}

pub struct FindAuthorTruncated;
impl FindAuthor<H160> for FindAuthorTruncated {
	fn find_author<'a, I>(digests: I) -> Option<H160>
		where I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])> {
		digests.into_iter().filter_map(|(id, data)| {
			if id == POSCAN_ENGINE_ID {
				Some(H160::from_slice(&data[4..24]))
			} else {
				None
			}
		})
			.next()
	}
}

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT / WEIGHT_PER_GAS);
	pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
	pub WeightPerGas: u64 = 20_000;
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type Currency = Balances;
	type Event = Event;
	type PrecompilesType = FrontierPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = EVMChainId;
	type BlockGasLimit = BlockGasLimit;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type OnChargeTransaction = ();
	type FindAuthor = FindAuthorTruncated;
}

impl pallet_ethereum::Config for Runtime {
	type Event = Event;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
}

parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(1_000_000_000);
	pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl pallet_base_fee::Config for Runtime {
	type Event = Event;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

impl pallet_hotfix_sufficients::Config for Runtime {
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type WeightInfo = pallet_hotfix_sufficients::weights::SubstrateWeight<Runtime>;
}


use scale_info::TypeInfo;
use codec::MaxEncodedLen;

/// The type used to represent the kinds of proxying allowed.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(
Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, MaxEncodedLen, TypeInfo,
)]
pub enum ProxyType {
	/// All calls can be proxied. This is the trivial/most permissive filter.
	Any = 0,
	/// Only extrinsics that do not transfer funds.
	NonTransfer = 1,
	/// Only extrinsics related to governance (democracy and collectives).
	Governance = 2,
	/// Only extrinsics related to staking.
	//Staking = 3,
	/// Allow to veto an announced proxy call.
	CancelProxy = 4,
	/// Allow extrinsic related to Balances.
	Balances = 5,
	/// Allow extrinsic related to AuthorMapping.
	// AuthorMapping = 6,
	/// Allow extrinsic related to IdentityJudgement.
	IdentityJudgement = 7,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => {
				matches!(
					c,
					Call::System(..)
						//| Call::Timestamp(..) | Call::ParachainStaking(..)
						| Call::Democracy(..) | Call::Council(..)
						| Call::Identity(..) | Call::TechnicalCommittee(..)
						| Call::Utility(..) | Call::Proxy(..)
						//| Call::AuthorMapping(..)
						// | Call::CrowdloanRewards(pallet_crowdloan_rewards::Call::claim { .. })
				)
			}
			ProxyType::Governance => matches!(
				c,
				Call::Democracy(..)
					| Call::Council(..)
					| Call::TechnicalCommittee(..)
					| Call::Utility(..)
			),
			// ProxyType::Staking => matches!(
			// 	c,
			// 	Call::ParachainStaking(..)
			// 		| Call::Utility(..) | Call::AuthorMapping(..)
			// 		 Call::MoonbeamOrbiters(..)
			// ),
			ProxyType::CancelProxy => matches!(
				c,
				Call::Proxy(pallet_proxy::Call::reject_announcement { .. })
			),
			ProxyType::Balances => matches!(c, Call::Balances(..) | Call::Utility(..)),
			// ProxyType::AuthorMapping => matches!(c, Call::AuthorMapping(..)),
			ProxyType::IdentityJudgement => matches!(
				c,
				Call::Identity(pallet_identity::Call::provide_judgement { .. }) | Call::Utility(..)
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	// One storage item; key size 32, value size 8
	type ProxyDepositBase = ConstU128<{ deposit(1, 8) }>;
	// Additional storage item size of 21 bytes (20 bytes AccountId + 1 byte sizeof(ProxyType)).
	type ProxyDepositFactor = ConstU128<{ deposit(0, 21) }>;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = ConstU128<{ deposit(1, 8) }>;
	// Additional storage item size of 56 bytes:
	// - 20 bytes AccountId
	// - 32 bytes Hasher (Blake2256)
	// - 4 bytes BlockNumber (u32)
	type AnnouncementDepositFactor = ConstU128<{ deposit(0, 56) }>;
}


pub struct Migrations;
impl OnRuntimeUpgrade for Migrations {
	fn on_runtime_upgrade() -> Weight {
		migration::migrate::<Runtime>()
	}
}

const LOG_TARGET: &'static str = "runtime::runtime";

pub struct SessionUpgrade;
impl frame_support::traits::OnRuntimeUpgrade for SessionUpgrade {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use sp_core::sr25519;
		log::debug!(target: LOG_TARGET, "SessionUpgrade::on_runtime_upgrade");
		if VERSION.spec_version < 103 {
			Session::upgrade_keys(|v, old_keys: opaque::OldSessionKeys| {
				let keys = opaque::SessionKeys {
					grandpa: old_keys.get(KeyTypeId([b'g', b'r', b'a', b'n'])).unwrap(),
					imonline: sr25519::Public::from_raw(v.into()).into(),
				};
				keys
			});
		}
		0
	}
}

// Add SerialNumbers config impl here, before construct_runtime!
impl pallet_serial_numbers::Config for Runtime {
    type Event = Event;
    type MaxSerialNumbersPerBlock = ConstU32<10000>; // Allow up to 10,000 SNs per block per owner
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system,
		Utility: pallet_utility,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		Scheduler: pallet_scheduler,
		Preimage: pallet_preimage,
		Indices: pallet_indices,
		Rewards: rewards::{Pallet, Call, Storage, Event<T>, Config<T>},
		Balances: pallet_balances,
		Difficulty: difficulty::{Pallet, Call, Storage, Config} ,
		Authorship: pallet_authorship,
		TransactionPayment: pallet_transaction_payment,
		Democracy: pallet_democracy,
		Referenda: pallet_referenda,
		Membership: pallet_membership,
		ConvictionVoting: pallet_conviction_voting,
		Remark: pallet_remark,
		Council: pallet_collective::<Instance1>,
		TechnicalCommittee: pallet_collective::<Instance2>,
		PhragmenElection: pallet_elections_phragmen,
		Treasury: pallet_treasury,
		Multisig: pallet_multisig,
		TransactionStorage: pallet_transaction_storage,
		ScoredPool: pallet_scored_pool,
		Uniques: pallet_uniques,
		AssetRate: pallet_asset_rate,
		ValidatorSet: pallet_validator_set,
		Offences: pallet_offences,
		// Historical: pallet_session_historical::{Pallet},
		Session: pallet_session,
		Bounties: pallet_bounties,
		ChildBounties: pallet_child_bounties,
		Grandpa: pallet_grandpa,
		ImOnline: pallet_im_online,
		Identity: pallet_identity, // ::{Pallet, Call, Storage, Event<T>},
		Vesting: pallet_vesting,
		Whitelist: pallet_whitelist,
		Contracts: pallet_contracts,
		RankedPolls: pallet_referenda::<Instance2>,
		RankedCollective: pallet_ranked_collective,
		AssetConversion: pallet_asset_conversion,
		AtomicSwap: pallet_atomic_swap::<Instance1>,
		PoscanAtomicSwap: pallet_atomic_swap::<Instance2>,
		PoScan: pallet_poscan, // ::{Pallet, Call, Storage, Event<T>, Inherent},
		PoscanAssets: pallet_poscan_assets::<Instance1>,
		PoscanPoolAssets: pallet_poscan_assets::<Instance2>,
		MiningPool: pallet_mining_pool,
		Sudo: pallet_sudo,
		Ethereum: pallet_ethereum,
		EVM: pallet_evm,
		EVMChainId: pallet_evm_chain_id,
		DynamicFee: pallet_dynamic_fee,
		BaseFee: pallet_base_fee,
		HotfixSufficients: pallet_hotfix_sufficients,
		Proxy: pallet_proxy,
		SerialNumbers: pallet_serial_numbers::{Pallet, Call, Storage, Event<T>},
	}
);

use codec::Decode;

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		)
	}
}

impl fp_rpc::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(
		&self,
		transaction: pallet_ethereum::Transaction,
	) -> opaque::UncheckedExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(
			pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
		);
		let encoded = extrinsic.encode();
		opaque::UncheckedExtrinsic::decode(&mut &encoded[..])
			.expect("Encoded extrinsic is always valid")
	}
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, Call, SignedExtra, H160>;
/// Executive: handles dispatch to the various pallets.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	(Migrations, SessionUpgrade),
>;

impl fp_self_contained::SelfContainedCall for Call {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			Call::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			Call::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<Call>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			Call::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ Call::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
				Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info)),
			)),
			_ => None,
		}
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header);
			// PoScan::initialize_block(header);
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
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
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
			log::debug!("submit_report_equivocation_unsigned_extrinsic 1");

			// let decode::pallet_validator_set::ValidatorProof2::decode();
			use pallet_validator_set::ValidatorProof;
			let key_owner_proof_maybe: Option<ValidatorProof> = key_owner_proof.decode();

			let proof = key_owner_proof_maybe?;

			log::debug!("submit_report_equivocation_unsigned_extrinsic 2: {:#?}", &proof);

			let proof = ValidatorSet::prove((sp_finality_grandpa::KEY_TYPE, proof.authority_id)).unwrap();

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_finality_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<sp_finality_grandpa::OpaqueKeyOwnershipProof> {
			log::debug!("generate_key_ownership_proof: {:#?}", &authority_id);
			Some(sp_finality_grandpa::OpaqueKeyOwnershipProof::new(
				ValidatorSet::prove((sp_finality_grandpa::KEY_TYPE, authority_id)).unwrap().encode()
			))
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _) = EVM::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			EVM::account_codes(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			EVM::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				evm_config,
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

			let is_transactional = false;
			let validate = true;
			let evm_config = config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config());
			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				evm_config,
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				Call::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(BaseFee::elasticity())
		}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>
		for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: u64,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts_primitives::ContractExecResult<Balance> {
			Contracts::bare_call(origin, dest, value, gas_limit, storage_deposit_limit, input_data, CONTRACTS_DEBUG_OUTPUT)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: u64,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts_primitives::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance>
		{
			Contracts::bare_instantiate(origin, value, gas_limit, storage_deposit_limit, code, data, salt, CONTRACTS_DEBUG_OUTPUT)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts_primitives::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	impl pallet_asset_conversion::AssetConversionApi<
		Block,
		Balance,
		u128,
		NativeOrAssetId<u32>
	> for Runtime
	{
		fn quote_price_exact_tokens_for_tokens(asset1: NativeOrAssetId<u32>, asset2: NativeOrAssetId<u32>, amount: u128, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee)
		}

		fn quote_price_tokens_for_exact_tokens(asset1: NativeOrAssetId<u32>, asset2: NativeOrAssetId<u32>, amount: u128, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee)
		}

		fn get_reserves(asset1: NativeOrAssetId<u32>, asset2: NativeOrAssetId<u32>) -> Option<(Balance, Balance)> {
			AssetConversion::get_reserves(&asset1, &asset2).ok()
		}
	}

	impl sp_consensus_poscan::DifficultyApi<Block, sp_consensus_poscan::Difficulty> for Runtime {
		fn difficulty() -> sp_consensus_poscan::Difficulty {
			difficulty::Module::<Runtime>::difficulty() / difficulty::Module::<Runtime>::scale()
		}

		fn hist_steps() -> u32 {
			difficulty::Module::<Runtime>::hist_steps()
		}
	}

	impl sp_consensus_poscan::MiningPoolApi<Block, AccountId> for Runtime {
		fn difficulty(pool_id: &AccountId) -> sp_consensus_poscan::Difficulty {
			// MiningPool::difficulty(pool_id)
			<MiningPool as MiningPoolStatApi<sp_consensus_poscan::Difficulty, AccountId>>::difficulty(pool_id)
		}
		fn member_status(pool_id: &AccountId, member_id: &AccountId) -> Result<(), CheckMemberError> {
			<MiningPool as MiningPoolStatApi<sp_consensus_poscan::Difficulty, AccountId>>::member_status(pool_id, member_id)
		}
		fn get_stat(pool_id: &AccountId) -> Option<(Percent, Percent, Vec<(AccountId,u32)>)> {
			<MiningPool as MiningPoolStatApi<sp_consensus_poscan::Difficulty, AccountId>>::get_stat(pool_id)
		}
	}

	impl sp_consensus_poscan::PoscanApi<Block, AccountId, BlockNumber> for Runtime {
		fn get_poscan_object(i: u32) -> Option<sp_consensus_poscan::ObjData<AccountId, BlockNumber>> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_poscan_object(i)
		}
		fn check_object(alg_id: &[u8;16], obj: &Vec<u8>, hashes: &Vec<H256>) -> bool {
			log::debug!("Runtime object check");
			check::check_obj(alg_id, obj, hashes)
		}
		fn get_obj_hashes_wasm(ver: &[u8; 16], data: &Vec<u8>, pre: &H256, depth: u32, patch_rot: bool) -> Vec<H256> {
			algo::get_obj_hashes_wasm(ver, &data[..], pre, depth as usize, patch_rot)
		}
		fn replicas_of(original_idx: u32) -> Vec<u32> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::replicas_of(original_idx)
		}
		fn get_dynamic_rewards_growth_rate() -> Option<u32> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_dynamic_rewards_growth_rate()
		}
		fn get_author_part() -> Option<u8> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_author_part()
		}
		fn get_unspent_rewards(obj_idx: u32) -> Option<u128> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_unspent_rewards(obj_idx)
		}
		fn get_fee_payer(obj_idx: u32) -> Option<AccountId> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_fee_payer(obj_idx)
		}
		fn get_pending_storage_fees() -> Option<u128> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_pending_storage_fees()
		}
		fn get_rewards() -> Option<u128> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_rewards()
		}
		fn get_object_idx_by_proof_of_existence(proof: H256) -> Option<u32> {
			<PoScan as PoscanApi<AccountId, BlockNumber>>::get_object_idx_by_proof_of_existence(proof)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade().unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block_no_check(block: Block) -> Weight {
			Executive::execute_block_no_check(block)
		}
	}
}

impl serial_numbers_api::SerialNumbersApi<AccountId> for Runtime {
	fn verify_serial_number(sn_hash: [u8; 16], block: u32) -> bool {
		SerialNumbers::verify_serial_number(sn_hash, block)
	}

	fn verify_serial_number_stateless(
		sn_hash: [u8; 16], 
		owner: &AccountId, 
		block: u32, 
		block_index: u32
	) -> bool {
		SerialNumbers::verify_serial_number_stateless(sn_hash, owner, block, block_index)
	}

	fn generate_serial_numbers_for_block(
		owner: &AccountId, 
		block: u32, 
		count: u32
	) -> Vec<[u8; 16]> {
		SerialNumbers::generate_serial_numbers_for_block(owner, block, count)
	}

	fn get_serial_numbers(sn_index: Option<u64>) -> Vec<serial_numbers_api::SerialNumberDetails<AccountId, u32>> {
		SerialNumbers::get_serial_numbers(sn_index)
			.into_iter()
			.map(|details| serial_numbers_api::SerialNumberDetails {
				sn_index: details.sn_index,
				sn_hash: details.sn_hash,
				initial_owner: details.initial_owner,
				owner: details.owner,
				created: details.created,
				block_index: details.block_index,
				expired: details.expired.map(|b| b.saturated_into()),
				is_expired: details.is_expired,
			})
			.collect()
	}

	fn get_sn_owners(owner: AccountId) -> Vec<u64> {
		SerialNumbers::get_sn_owners(owner)
	}

	fn is_serial_number_used(sn_hash: [u8; 16]) -> bool {
		SerialNumbers::is_serial_number_used(sn_hash)
	}

	fn sn_count() -> u64 {
		SerialNumbers::sn_count()
	}

	fn transfer_ownership(_sn_index: u64, _new_owner: AccountId) -> bool {
		false
	}
}

use precompiles::LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX;

// Instruct how to go from an H160 to an AssetID
// We just take the lowest 128 bits
impl AccountIdAssetIdConversion<AccountId, AssetId> for Runtime {
	/// The way to convert an account to assetId is by ensuring that the prefix is 0XFFFFFFFF
	/// and by taking the lowest 128 bits as the assetId
	fn account_to_asset_id(account: AccountId) -> Option<(Vec<u8>, AssetId)> {
		let h160_account: &[u8] = account.as_ref();
		let mut data = [0u8; 4];
		let (prefix_part, _padding, id_part) = (&h160_account[0..4], &h160_account[4..16], &h160_account[16..20]); //; h160_account.as_fixed_bytes().split_at(4);
		if prefix_part == LOCAL_ASSET_PRECOMPILE_ADDRESS_PREFIX
		{
			data.copy_from_slice(id_part);
			let asset_id: AssetId = u32::from_be_bytes(data).into();
			Some((prefix_part.to_vec(), asset_id))
		} else {
			None
		}
	}

	// The opposite conversion
	fn asset_id_to_account(prefix: &[u8], asset_id: AssetId) -> AccountId {
		let mut data = [0u8; 32];
		data[0..4].copy_from_slice(prefix);
		data[4..16].copy_from_slice(&[0;12]);
		data[16..20].copy_from_slice(&asset_id.to_be_bytes());
		// AccountId::from(data)
		//use scale_info::prelude::format;
		//let dd: [u8;32] = [1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6,7,8,9,0,1,2];
		AccountId::new(data)
	}
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;
extern crate core;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[pallet_assets, Assets]
		[pallet_poscan_assets, PoscanAssets]
		[pallet_balances, Balances]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_collective, Council]
		[pallet_conviction_voting, ConvictionVoting]
		[pallet_contracts, Contracts]
		[pallet_democracy, Democracy]
		[pallet_elections_phragmen, Elections]
		[pallet_grandpa, Grandpa]
		[pallet_identity, Identity]
		[pallet_im_online, ImOnline]
		[pallet_indices, Indices]
		[pallet_membership, TechnicalMembership]
		[pallet_multisig, Multisig]
		[pallet_preimage, Preimage]
		[pallet_ranked_collective, RankedCollective]
		[pallet_referenda, Referenda]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_transaction_storage, TransactionStorage]
		[pallet_treasury, Treasury]
		[pallet_uniques, Uniques]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_whitelist, Whitelist]
	);
}
