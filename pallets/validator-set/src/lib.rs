//! # Validator Set Pallet
//!
//! This file is part of 3DPass.
//! Copyright (c) 2022 3DPass.
//!
//! The Validator Set Pallet allows for addition and removal of
//! authorities/validators via extrinsics (transaction calls), in
//! Substrate-based PoA networks. It also integrates with the im-online pallet
//! to automatically remove offline validators.
//!
//! The pallet uses the Session pallet and implements related traits for session
//! management. Currently, it uses periodic session rotation provided by the
//! session pallet to automatically rotate sessions. For this reason, the
//! validator addition and removal becomes effective only after 2 sessions
//! (queuing + applying).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate core;

mod mock;
mod tests;

use lazy_static::lazy_static;
use core::sync::atomic::AtomicBool;

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		Currency, LockableCurrency, EstimateNextSessionRotation,
		Get, ValidatorSet, ValidatorSetWithIdentification,
		OnUnbalanced, ExistenceRequirement, LockIdentifier, WithdrawReasons,
		fungible::{Inspect, Unbalanced},
		KeyOwnerProofSystem,
	},
	sp_runtime::SaturatedConversion,
};

use frame_system::offchain::{
	CreateSignedTransaction,
	SubmitTransaction,
};
use sp_application_crypto::RuntimeAppPublic;
pub use pallet::*;
use sp_runtime::{
	traits::{Convert, Zero, One, Saturating},
	offchain::{
		storage_lock::{StorageLock, Time},
		storage::{
			// MutateStorageError,
			StorageRetrievalError,
			StorageValueRef,
		},
	},
	Permill,
	Perbill,
	KeyTypeId,
};
use sp_finality_grandpa::{AuthorityId as GrandpaId, SetId};
use sp_session::ValidatorCount;
use frame_system::pallet_prelude::*;
use pallet_session::ShouldEndSession;
use sp_staking::offence::{Offence, OffenceError, ReportOffence, OnOffenceHandler};
use sp_staking::SessionIndex;
use sp_version::RuntimeVersion;
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
use core::convert::TryInto;
use sp_core::H256;
use sp_consensus_poscan::HOURS;
use poscan_algo;
use sp_consensus_poscan::POSCAN_ALGO_GRID2D_V3A;

use rewards_api::RewardLocksApi;
use validator_set_api::ValidatorSetApi;

const CUR_SPEC_VERSION: u32 = 101;
const UPGRADE_SLASH_DELAY: u32 = 5 * 24 * HOURS;
const THROW_WO_SLASH_WINDOW: u32 = 14 * 24 * HOURS;
const LOCK_ID: LockIdentifier = *b"validatr";
pub const LOG_TARGET: &str = "runtime::validator-set";
const ESTIMATION_LOCK: &'static [u8] = b"validator-set::estimate";

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

lazy_static! {
    pub static ref CALC: AtomicBool = {
        AtomicBool::default()
    };
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, TypeInfo, Default)]
pub enum RemoveReason {
	#[default]
	Normal,
	DepositBelowLimit,
	ImOnlineSlash,
	CouncilSlash,
	Equivocation,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Estimation<AuthorityId> {
	pub obj_idx: u32,
	pub t: u64,
	pub authority_id: AuthorityId,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Configure the pallet by specifying the parameters and types on which it
	/// depends.
	#[pallet::config]
	pub trait Config: frame_system::Config
		+ pallet_session::Config
		+ pallet_treasury::Config
		+ pallet_balances::Config
		+ pallet_poscan::Config
		+ CreateSignedTransaction<Call<Self>>
	{
		/// The Event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Origin for adding or removing a validator.
		type AddRemoveOrigin: EnsureOrigin<Self::Origin>;

		/// Minimum number of validators to leave in the validator set during
		/// auto removal.
		type MinAuthorities: Get<u32>;

		type Currency: LockableCurrency<Self::AccountId> + Unbalanced<Self::AccountId> + Inspect<Self::AccountId>;

		#[pallet::constant]
		type PoscanEngineId: Get<[u8; 4]>;

		#[pallet::constant]
		type EstimatePriority: Get<TransactionPriority>;

		#[pallet::constant]
		type EstimateUnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		type FilterLevels: Get<[(u128, u32); 4]>;

		#[pallet::constant]
		type MaxMinerDepth: Get<u32>;

		type RewardLocksApi: RewardLocksApi<Self::AccountId, BalanceOf<Self>>;

		#[pallet::constant]
		type PenaltyOffline: Get<u128>;

		#[pallet::constant]
		type MinLockAmount: Get<u128>;

		#[pallet::constant]
		type MinLockPeriod: Get<u32>;

		#[pallet::constant]
		type SlashValidatorFor: Get<u32>;

		#[pallet::constant]
		type AddAfterSlashPeriod: Get<u32>;

		type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

		#[pallet::constant]
		type DefaultOffset: Get<u32>;

		#[pallet::constant]
		type DefaultPeriod: Get<u32>;

		#[pallet::constant]
		type MaxVal: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_validators)]
	pub type ApprovedValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn validators_to_remove)]
	pub type OfflineValidators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn authors)]
	pub type Authors<T: Config> = StorageMap<_, Twox64Concat, T::BlockNumber, Option<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type ValidatorLock<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, BalanceOf<T>, Option<u32>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn enter_deposit)]
	pub type EnterDeposit<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, BalanceOf<T>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn removed)]
	pub type AccountRemoveReason<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Option<(T::BlockNumber, RemoveReason)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn upgrades)]
	pub type LastUpgrade<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn penalty)]
	pub type Penalty<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BalanceOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn last_throw)]
	pub type LastThrow<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::BlockNumber, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pallet_version)]
	pub type PalletVersion<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The current session duration.
	///
	/// SessionDuration: BlockNumberFor<T>
	#[pallet::storage]
	#[pallet::getter(fn session_duration)]
	pub type SessionDuration<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// The current session duration offset.
	///
	/// DurationOffset: BlockNumberFor<T>
	#[pallet::storage]
	#[pallet::getter(fn duration_offset)]
	pub type DurationOffset<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Mapping from block number to new session index and duration.
	///
	/// SessionDurationChanges: map BlockNumber => (SessionIndex, SessionDuration)
	#[pallet::storage]
	#[pallet::getter(fn session_duration_changes)]
	pub type SessionDurationChanges<T: Config> =
	StorageMap<_, Twox64Concat, BlockNumberFor<T>, (SessionIndex, BlockNumberFor<T>), ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator addition initiated. Effective in ~2 sessions.
		ValidatorAdditionInitiated(T::AccountId),

		/// Validator removal initiated. Effective in ~2 sessions.
		ValidatorRemovalInitiated(T::AccountId),

		ValidatorSlash(T::AccountId, BalanceOf<T>),

		ValidatorLockBalance(T::AccountId, T::BlockNumber, BalanceOf<T>, Option<u32>),

		ValidatorUnlockBalance(T::AccountId, BalanceOf<T>),

		PenaltySet(T::AccountId, BalanceOf<T>),

		PenaltyCanceled(T::AccountId, BalanceOf<T>),

		/// Scheduled session duration.
		ScheduledSessionDuration {
			block_number: BlockNumberFor<T>,
			session_index: SessionIndex,
			session_duration: BlockNumberFor<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Target (post-removal) validator count is below the minimum.
		TooLowValidatorCount,
		/// Validator is already in the validator set.
		Duplicate,
		/// Validator is not approved for re-addition.
		ValidatorNotApproved,
		/// Only the validator can add itself back after coming online.
		BadOrigin,
		/// Has not mined.
		ValidatorHasNotMined,
		/// Locked amount too low.
		AmountLockedBelowLimit,
		/// decrease lock amount not allowed .
		DecreaseLockAmountNotAllowed,
		/// Decrease lock prolongation not allowed.
		DecreaseLockPeriodNotAllowed,
		/// Lock prolongation period too little.
		PeriodLockBelowLimit, // {pub limit: u32},
		/// No lock.
		NotLocked,
		/// Insufficient Balance,
		UnsufficientBalance,
		/// lock ia active
		LockIsActive, // {pub upto_block: u32},
		/// temporary disallowed
		TmpDisalowed,
		/// Unlock amount is invalid
		UnlockAmountInvalid,
		/// Validator not found
		ValidatorNotFound,
		/// Penalty found
		PenaltyFound,
		/// The session is invalid.
		InvalidSession,
		/// The duration is invalid.
		InvalidDuration,
		/// Failed to estimate next session.
		EstimateNextSessionFailed,
		/// Too many validators.
		TooManyValidators,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut skip = true;
			SessionDurationChanges::<T>::mutate_exists(n, |maybe_changes| {
				if let Some((_, duration)) = maybe_changes.take() {
					skip = false;
					SessionDuration::<T>::put(duration);
					DurationOffset::<T>::put(n);
				}
			});
			0
		}

		fn on_finalize(n: T::BlockNumber) {
			let author = frame_system::Pallet::<T>::digest()
				.logs
				.iter()
				.filter_map(|s| s.as_pre_runtime())
				.filter_map(|(id, mut data)| {
					if id == <T as pallet::Config>::PoscanEngineId::get() {
						T::AccountId::decode(&mut data).ok()
					} else {
						None
					}
				}
				)
				.next();

			if let Some(author) = author {
				let deposit = T::RewardLocksApi::locks(&author);
				let d = u128::from_le_bytes(deposit.encode().try_into().unwrap());

				log::debug!(target: LOG_TARGET, "Account: {:?}", &author);
				log::debug!(target: LOG_TARGET, "Deposit: {}", d);
				Authors::<T>::insert(n, Some(author));
			}
			else {
				log::debug!(target: LOG_TARGET, "No author");
			}
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let current_block = frame_system::Pallet::<T>::block_number();
			<LastUpgrade<T>>::put(current_block);

			if PalletVersion::<T>::get() == 0 {
				SessionDuration::<T>::put::<BlockNumberFor<T>>(T::DefaultPeriod::get().into());
				DurationOffset::<T>::put::<BlockNumberFor<T>>(T::DefaultOffset::get().into());
				PalletVersion::<T>::put(1);
			}
			else {
				log::info!(" >>> Unused migration!");
			}
			0
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			log::debug!(target: LOG_TARGET, "offchain_worker: try to estimate objects");

			let local_keys = T::AuthorityId::all();
			let local_key = local_keys.get(0);

			if let Some(local_key) = local_key {
				let acc = T::AccountId::decode(&mut &local_key.encode()[..]).unwrap();
				if Self::validators().contains(&acc) {
					let objects = pallet_poscan::Pallet::<T>::created_objects();
					if objects.len() > 0 {
						// TODO: check local storage
						let obj = objects[0].clone();
						let algo_id = POSCAN_ALGO_GRID2D_V3A;
						log::debug!(target: LOG_TARGET, "offchain_worker: estimate obj_idx {}", &obj.0);
						let timeout = pallet_poscan::Pallet::<T>::max_algo_time();
						let raw_obj = match obj.1.compressed_with {
							None => obj.1.obj.clone().into(),
							Some(compress_mode) => compress_mode.decompress(&obj.1.obj),
						};
						let res = poscan_algo::hashable_object::estimate_obj(&algo_id, &raw_obj, timeout);

						if let Some((t, hashes)) = res {
							let t: u64 = if let Ok(t) = t.try_into() {
								t
							}
							else {
								log::error!(target: LOG_TARGET, "offchain_worker: estimation too big ({}) obj_idx {}", &t, &obj.0);
								return
							};
							let calc_hashes: Vec<H256> = obj.1.hashes.into();
							if hashes == calc_hashes {
								log::debug!(target: LOG_TARGET, "offchain_worker: estimated obj_idx {}: {}", &obj.0, &t);
								Self::save_estimation(block_number, obj.0, t);
							} else {
								log::debug!(target: LOG_TARGET, "offchain_worker: estimated but hashes are invalid obj_idx {}: {}", &obj.0, &t);
							}
						} else {
							log::debug!(target: LOG_TARGET, "offchain_worker: estimation failed obj_idx {}", &obj.0);
						}
					}
					let _ = Self::send_estimations();
				}
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
			SessionDuration::<T>::put::<BlockNumberFor<T>>(T::DefaultPeriod::get().into());
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new validator.
		///
		/// New validator's session keys should be set in Session pallet before
		/// calling this.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::weight(10_000_000)]
		pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			Self::do_add_validator(validator_id.clone(), false)?;
			Self::approve_validator(validator_id)?;

			Ok(())
		}

		/// Remove a validator.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::weight(10_000_000)]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			let current_block = frame_system::Pallet::<T>::block_number();

			if !Self::validators().contains(&validator_id) {
				return Err(Error::<T>::ValidatorNotFound.into());
			}

			Self::do_remove_validator(validator_id.clone())?;
			Self::unapprove_validator(validator_id.clone())?;
			AccountRemoveReason::<T>::insert(&validator_id, Some((current_block, RemoveReason::Normal)));

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn remove_validator_with_slash(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			let current_block = frame_system::Pallet::<T>::block_number();

			if !Self::validators().contains(&validator_id) {
				return Err(Error::<T>::ValidatorNotFound.into());
			}

			let penalty: BalanceOf<T> = T::PenaltyOffline::get().saturated_into();
			Self::slash(
				&validator_id,
				penalty,
				|acc, amount| Event::<T>::ValidatorSlash(acc.clone(), amount),
			);

			Self::do_remove_validator(validator_id.clone())?;
			Self::unapprove_validator(validator_id.clone())?;
			AccountRemoveReason::<T>::insert(&validator_id, Some((current_block, RemoveReason::CouncilSlash)));

			Ok(())
		}

		/// Add an approved validator again when it comes back online.
		///
		/// For this call, the dispatch origin must be the validator itself.
		#[pallet::weight(10_000_000)]
		pub fn rejoin_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(who == validator_id, Error::<T>::BadOrigin);

			let approved_set: BTreeSet<_> = <ApprovedValidators<T>>::get().into_iter().collect();
			ensure!(approved_set.contains(&validator_id), Error::<T>::ValidatorNotApproved);

			if let Some(_amount) = Penalty::<T>::get(&validator_id) {
				return Err(Error::<T>::PenaltyFound.into());
			}

			let current_number = frame_system::Pallet::<T>::block_number();

			let suspend_period = T::SlashValidatorFor::get();
			let allow_period = T::AddAfterSlashPeriod::get();
			let mut check_block_num = true;
			let maybe_removed = AccountRemoveReason::<T>::get(&validator_id);

			if let Some(remove_data) = maybe_removed {
				check_block_num = false;
				match remove_data.1 {
					RemoveReason::Normal => { },
					RemoveReason::DepositBelowLimit |
					RemoveReason::CouncilSlash |
					RemoveReason::Equivocation |
					RemoveReason::ImOnlineSlash => {
						let t1 = remove_data.0 + suspend_period.into();
						let t2 = t1 + allow_period.into();

						if current_number < t1 {
							return Err(Error::<T>::TmpDisalowed.into());

						}
						else if current_number >= t2 {
							check_block_num = true;
						}
					}
				}
			}

			Self::do_add_validator(validator_id, check_block_num)?;

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn lock(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			until: T::BlockNumber,
			period: Option<u32>,
		) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;
			let min_period = T::MinLockPeriod::get();
			let free = <T as pallet::Config>::Currency::free_balance(&validator_id);
			let current_number = frame_system::Pallet::<T>::block_number();

			if free < amount {
				return Err(Error::<T>::UnsufficientBalance.into());
			}

			if until - current_number < min_period.into() {
				return Err(Error::<T>::PeriodLockBelowLimit.into());
			}

			if let Some(per) = period {
				if per < min_period {
					return Err(Error::<T>::PeriodLockBelowLimit.into());
				}
			}

			if let Some((to_block, val, _)) = ValidatorLock::<T>::get(&validator_id) {
				if amount < val {
					return Err(Error::<T>::DecreaseLockAmountNotAllowed.into());
				}
				if until < to_block {
					return Err(Error::<T>::DecreaseLockPeriodNotAllowed.into());
				}
			}

			Self::set_lock(validator_id.clone(), until, amount, period);

			Self::deposit_event(Event::ValidatorLockBalance(validator_id.clone(), until, amount, period));
			log::debug!(target: LOG_TARGET, "Locked {:?} for validator_id: {:?} up to block {:?}.", amount, validator_id, until);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn unlock(
			origin: OriginFor<T>,
			amount: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;
			let lock_item = ValidatorLock::<T>::get(&validator_id).ok_or(Error::<T>::NotLocked)?;
			let current_number = frame_system::Pallet::<T>::block_number();

			if lock_item.0 > current_number {
				return Err(Error::<T>::LockIsActive.into())
			}

			let mut unlock_amount = amount.unwrap_or(lock_item.1);

			if unlock_amount > lock_item.1 {
				return Err(Error::<T>::UnlockAmountInvalid.into())
			}

			if let Some(penalty) = Penalty::<T>::get(&validator_id) {
				if unlock_amount > lock_item.1 - penalty {
					unlock_amount = lock_item.1 - penalty;
				}
			}

			if unlock_amount == lock_item.1 {
				<T as pallet::Config>::Currency::remove_lock(
					LOCK_ID,
					&validator_id,
				);
				ValidatorLock::<T>::remove(&validator_id);
			}
			else {
				let new_lock_amount = lock_item.1 - unlock_amount;
				<T as pallet::Config>::Currency::set_lock(
					LOCK_ID,
					&validator_id,
					new_lock_amount,
					WithdrawReasons::all(),
				);
				ValidatorLock::<T>::insert(&validator_id, Some((lock_item.0, new_lock_amount, lock_item.2)));
			}
			Self::deposit_event(Event::ValidatorUnlockBalance(validator_id.clone(), unlock_amount));
			log::debug!(target: LOG_TARGET, "Unlocked {:?} for validator_id: {:?}.", unlock_amount, validator_id);

			Ok(())
		}

		#[pallet::weight(10_000_000)]
		pub fn unlock_force(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
			amount: Option<BalanceOf<T>>,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			let lock_item = ValidatorLock::<T>::get(&validator_id).ok_or(Error::<T>::NotLocked)?;

			let remove_all;
			let unlock_amount;

			if let Some(amount) = amount {
				if amount > lock_item.1 {
					return Err(Error::<T>::UnlockAmountInvalid.into())
				}
				unlock_amount = amount;
				remove_all = amount == lock_item.1;
			}
			else {
				unlock_amount = lock_item.1;
				remove_all = true;
			}

			if remove_all {
				<T as pallet::Config>::Currency::remove_lock(
					LOCK_ID,
					&validator_id,
				);
				ValidatorLock::<T>::remove(&validator_id);
			}
			else {
				let new_lock_amount = lock_item.1 - unlock_amount;
				<T as pallet::Config>::Currency::set_lock(
					LOCK_ID,
					&validator_id,
					new_lock_amount,
					WithdrawReasons::all(),
				);
				ValidatorLock::<T>::insert(&validator_id, Some((lock_item.0, new_lock_amount, lock_item.2)));
			}
			Self::deposit_event(Event::ValidatorUnlockBalance(validator_id.clone(), unlock_amount));
			log::debug!(target: LOG_TARGET, "Unlocked {:?} for validator_id: {:?} by council.", unlock_amount, validator_id);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn submit_estimation(
			origin: OriginFor<T>,
			est: Estimation<T::AuthorityId>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResultWithPostInfo {
			log::debug!(target: LOG_TARGET, "submit_estimation");

			ensure_none(origin)?;

			let acc = T::AccountId::decode(&mut &est.authority_id.encode()[..]).unwrap();
			pallet_poscan::Pallet::<T>::add_obj_estimation(&acc, est.obj_idx, est.t);

			Ok(().into())
		}

		/// Pay penalty if it exists
		#[pallet::weight(10_000_000)]
		pub fn pay_penalty(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let validator_id = ensure_signed(origin)?;

			Penalty::<T>::get(&validator_id)
				.map_or(Ok(()), |amount|
					{
						let pot_id = pallet_treasury::Pallet::<T>::account_id();
						<T as pallet::Config>::Currency::transfer(
							&validator_id, &pot_id, amount, ExistenceRequirement::KeepAlive,
						)?;
						Penalty::<T>::remove(&validator_id);
						Ok(())
					}
				)
		}

		/// Cancel penalty if it exists
		#[pallet::weight(1_000_000)]
		pub fn cancel_penalty(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			if let Some(amount) = Penalty::<T>::get(&validator_id) {
				Penalty::<T>::remove(&validator_id);
				Self::deposit_event(Event::PenaltyCanceled(validator_id, amount));
			}

			Ok(())
		}

		/// Set penalty
		#[pallet::weight(1_000_000)]
		pub fn set_penalty(
			origin: OriginFor<T>,
			validator_id: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;
			Penalty::<T>::insert(&validator_id, amount);
			Self::deposit_event(Event::PenaltySet(validator_id, amount));

			Ok(())
		}

		/// Service extrinsic to fix total issuance
		#[pallet::weight(1_000_000)]
		pub fn fix_total_supply(
			origin: OriginFor<T>,
			amount: <<T as Config>::Currency as Inspect<T::AccountId>>::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			<T as pallet::Config>::Currency::set_total_issuance(amount);
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn schedule_session_duration(
			origin: OriginFor<T>,
			#[pallet::compact] start_session: SessionIndex,
			#[pallet::compact] duration: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let target_block_number = Self::do_schedule_session_duration(start_session, duration)?;

			Self::deposit_event(Event::ScheduledSessionDuration {
				block_number: target_block_number,
				session_index: start_session,
				session_duration: duration,
			});
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn fix_set_id(
			origin: OriginFor<T>,
			set_id: SetId,
		) -> DispatchResult {
			ensure_root(origin)?;

			const PALLET_NAME: &'static str = "Grandpa";
			const STORAGE_NAME: &'static str = "CurrentSetId";

			let pallet_hash = sp_io::hashing::twox_128(PALLET_NAME.as_bytes());
			let storage_hash = sp_io::hashing::twox_128(STORAGE_NAME.as_bytes());

			let mut final_key = Vec::new();
			final_key.extend_from_slice(&pallet_hash);
			final_key.extend_from_slice(&storage_hash);
			frame_support::storage::unhashed::put::<SetId>(&final_key, &set_id);

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_estimation { est, signature } = call {
				let authority_id = &est.authority_id;
				let authority_id = T::AuthorityId::decode(&mut &authority_id.encode()[..]).unwrap();

				log::debug!(target: LOG_TARGET, "validate_unsigned for estimation");

				let signature_valid = est.using_encoded(|encoded_est| {
					authority_id.verify(&encoded_est, signature)
				});

				if !signature_valid {
					log::debug!(target: LOG_TARGET, "validate_unsigned::InvalidTransaction::BadProof");
					return InvalidTransaction::BadProof.into()
				}

				ValidTransaction::with_tag_prefix("EstimateObject")
					.priority(T::EstimateUnsignedPriority::get())
					.and_provides(call.encode())
					.and_provides(authority_id)
					.longevity(
						5u64,
					)
					.propagate(true)
					.build()
			} else {
				log::debug!(target: LOG_TARGET, "validate_unsigned::InvalidTransaction::Call");
				InvalidTransaction::Call.into()
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	fn initialize_validators(validators: &[T::AccountId]) {
		assert!(validators.len() as u32 >= T::MinAuthorities::get(), "Initial set of validators must be at least T::MinAuthorities");
		assert!(<Validators<T>>::get().is_empty(), "Validators are already initialized!");

		<Validators<T>>::put(validators);
		<ApprovedValidators<T>>::put(validators);
	}

	fn do_add_validator(validator_id: T::AccountId, check_block_num: bool) -> DispatchResult {
		let cur_block_number = <frame_system::Pallet<T>>::block_number();

		ensure!(<Validators<T>>::get().len() < T::MaxVal::get() as usize, Error::<T>::TooManyValidators);

		let item_lock = ValidatorLock::<T>::get(&validator_id).ok_or(Error::<T>::AmountLockedBelowLimit)?;
		let deposit =
			if item_lock.0 < cur_block_number  {
				BalanceOf::<T>::zero()
			}
			else {
				item_lock.1
			};
		{
			let d = u128::from_le_bytes(deposit.encode().try_into().unwrap());
			log::debug!(target: LOG_TARGET, "Deposit: {}", d);
		}
		if check_block_num {
			let levels = T::FilterLevels::get();
			let mut depth: u32 = T::MaxMinerDepth::get();

			if deposit < levels[0].0.saturated_into() {
				log::debug!(target: LOG_TARGET, "Too low deposit to be validator");
				return Err(Error::<T>::AmountLockedBelowLimit.into());
			}

			for i in (0..levels.len()).rev() {
				if deposit >= levels[i].0.saturated_into() {
					depth = levels[i].1;
					break
				}
			}

			let mut found = false;
			let mut n = 0u32;
			loop {
				n += 1;
				let block_num = cur_block_number - n.into();
				if block_num < 1u32.into() || n > depth {
					break;
				}
				if let Some(author_id) = Authors::<T>::get(block_num) {
					if validator_id == author_id {
						log::debug!(target: LOG_TARGET, "Validator found as miner in block {:?}", block_num);
						found = true;
						break;
					}
				}
			}
			if !found {
				log::debug!(target: LOG_TARGET, "Validator NOT found as miner within {} blocks", depth);
				return Err(Error::<T>::ValidatorHasNotMined.into());
			}
		}
		else if !Self::check_lock(&validator_id) {
			return Err(Error::<T>::AmountLockedBelowLimit.into());
		}

		let validator_set: BTreeSet<_> = <Validators<T>>::get().into_iter().collect();
		ensure!(!validator_set.contains(&validator_id), Error::<T>::Duplicate);
		<Validators<T>>::mutate(|v| v.push(validator_id.clone()));

		EnterDeposit::<T>::insert(&validator_id, Some((cur_block_number, deposit)));

		Self::deposit_event(Event::ValidatorAdditionInitiated(validator_id));
		log::debug!(target: LOG_TARGET, "Validator addition initiated.");

		Ok(())
	}

	fn do_remove_validator(validator_id: T::AccountId) -> DispatchResult {
		let mut validators = <Validators<T>>::get();

		// Ensuring that the post removal, target validator count doesn't go
		// below the minimum.
		ensure!(
			validators.len() as u32 > T::MinAuthorities::get(),
			Error::<T>::TooLowValidatorCount
		);

		validators.retain(|v| *v != validator_id);

		<Validators<T>>::put(validators);

		Self::deposit_event(Event::ValidatorRemovalInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator removal initiated.");

		Ok(())
	}

	fn approve_validator(validator_id: T::AccountId) -> DispatchResult {
		let approved_set: BTreeSet<_> = <ApprovedValidators<T>>::get().into_iter().collect();
		ensure!(!approved_set.contains(&validator_id), Error::<T>::Duplicate);
		<ApprovedValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		Ok(())
	}

	fn unapprove_validator(validator_id: T::AccountId) -> DispatchResult {
		let mut approved_set = <ApprovedValidators<T>>::get();
		approved_set.retain(|v| *v != validator_id);
		Ok(())
	}

	// Adds offline validators to a local cache for removal at new session.
	fn mark_for_removal(validator_id: T::AccountId, reason: RemoveReason) {
		let current_block = <frame_system::Pallet<T>>::block_number();
		AccountRemoveReason::<T>::insert(&validator_id, Some((current_block, reason)));

		<OfflineValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Offline validator marked for auto removal: {:#?}", validator_id);
	}

	fn slash<F>(validator_id: &T::AccountId, slash_amount: BalanceOf<T>, make_evt: F)
	where
		F: FnOnce(&T::AccountId, BalanceOf<T>) -> Event<T>,
	{
		let pot_id = pallet_treasury::Pallet::<T>::account_id();
		let min_bal = <<T as pallet::Config>::Currency as Currency<T::AccountId>>::minimum_balance();
		let maybe_lock = ValidatorLock::<T>::get(validator_id);
		let usable: u128 = pallet_balances::Pallet::<T>::usable_balance(validator_id).saturated_into();
		let mut usable: BalanceOf<T> = usable.saturated_into();

		if usable < slash_amount + min_bal {
			let unlock_amount = slash_amount + min_bal - usable;

			if let Some(lock_item) = maybe_lock {
				Self::set_lock(
					validator_id.clone(),
					lock_item.0,
					lock_item.1.saturating_sub(unlock_amount),
					lock_item.2,
				);
				let usable_val: u128 = pallet_balances::Pallet::<T>::usable_balance(validator_id).saturated_into();
				usable = usable_val.saturated_into();
			}
		}

		let usable = usable.saturating_sub(min_bal);
		let amount = core::cmp::min(slash_amount, usable.saturated_into());
		let res = <T as pallet::Config>::Currency::transfer(
			validator_id, &pot_id, amount, ExistenceRequirement::KeepAlive,
		);

		if let Err(e) = res {
			log::error!(target: LOG_TARGET, "Error slash account {:#?} by {:?}: {:?}.", validator_id, &amount, &e);
			return
		}

		log::debug!(target: LOG_TARGET, "Slash account {:?} by {:?}.", validator_id, &amount);

		if amount < slash_amount {
			Penalty::<T>::insert(validator_id, slash_amount - amount);
		}

		Self::deposit_event(make_evt(validator_id, amount));
	}


	// Removes offline validators from the validator set and clears the offline
	// cache. It is called in the session change hook and removes the validators
	// who were reported offline during the session that is ending. We do not
	// check for `MinAuthorities` here, because the offline validators will not
	// produce blocks and will have the same overall effect on the runtime.
	fn remove_offline_validators() {
		let validators_to_remove: BTreeSet<_> = <OfflineValidators<T>>::get().into_iter().collect();

		let mut validators = <Validators<T>>::get();
		let mut to_remove = 0;
		for r in validators_to_remove.iter() {
			if validators.len() as u32 <= T::MinAuthorities::get() {
				break
			}
			validators.retain(|v| *v != *r);
			to_remove += 1;
		}
		if to_remove > 0 {
			<Validators<T>>::put(validators);
			log::debug!(
				target: LOG_TARGET,
				"Initiated removal of {:?} offline validators.",
				to_remove,
			);
		}
		// Clear the offline validator list to avoid repeated deletion.
		<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
	}

	fn mark_if_no_locks() {
		let current_block = <frame_system::Pallet<T>>::block_number();
		if current_block < 100u32.into() {
			return
		}

		for v in Self::validators().into_iter() {
			if !Self::check_lock(&v) {
				Self::mark_for_removal(v, RemoveReason::DepositBelowLimit)
			}
		}
	}

	fn check_lock(validator_id: &T::AccountId) -> bool {
		let levels = T::FilterLevels::get();
		let zero = BalanceOf::<T>::zero();

		let maybe_enter_depo = EnterDeposit::<T>::get(validator_id);
		let maybe_lock = ValidatorLock::<T>::get(validator_id);
		let mut true_locked: BalanceOf<T> = zero;

		if let Some(lock) = maybe_lock {
			let current_block = frame_system::Pallet::<T>::block_number();
			true_locked = if lock.0 < current_block { zero } else { lock.1
			};
		}

		true_locked >= maybe_enter_depo.map_or_else(|| levels[0].0.saturated_into(), |d| d.1)
	}

	fn set_lock(
		validator_id: T::AccountId,
		when: T::BlockNumber,
		amount: BalanceOf<T>,
		period: Option<u32>,
	) {
		if amount > Zero::zero() {
			<T as pallet::Config>::Currency::set_lock(
				LOCK_ID,
				&validator_id,
				amount,
				WithdrawReasons::all(),
			);

			ValidatorLock::<T>::insert(&validator_id, Some((when, amount, period)));
		}
		else {
			<T as pallet::Config>::Currency::remove_lock(
				LOCK_ID,
				&validator_id,
			);
			ValidatorLock::<T>::remove(&validator_id);
		}
	}

	fn renew_locks() {
		let cur_block_number = <frame_system::Pallet<T>>::block_number();

		for v in Self::validators().into_iter() {
			if let Some((when, amount, Some(period))) = ValidatorLock::<T>::get(&v) {
				if cur_block_number >= when {
					let when = ((cur_block_number - when) / period.into() + 1u32.into()) * period.into();
					Self::set_lock(v.clone(), when, amount, Some(period));
				}
			}
		}
	}

	fn is_slash_delay() -> bool {
		let s: RuntimeVersion = <T as frame_system::Config>::Version::get();
		let sv = s.spec_version;

		if sv == CUR_SPEC_VERSION {
			let current_block = frame_system::Pallet::<T>::block_number();
			let upgrade_block = <LastUpgrade<T>>::get();

			if current_block - upgrade_block <= UPGRADE_SLASH_DELAY.into() {
				return true
			}
		}
		false
	}

	fn send_estimations() -> Result<(), &'static str> {
		log::debug!(target: LOG_TARGET, "send_estimation");
		let local_keys = T::AuthorityId::all();

		log::debug!(target: LOG_TARGET, "Number of AuthorityId keys: {}", local_keys.len());
		let local_key = local_keys.get(0).ok_or("No key for validator in local keystorage")?;
		let _network_state = sp_io::offchain::network_state().map_err(|_| "OffchainErr::NetworkState")?;

		let send_item = |obj_idx: u32, t: u64| -> Result<(), &'static str> {
			let est = Estimation { obj_idx, t, authority_id: local_key.clone() };
			let signature = local_key.sign(&est.encode()).ok_or("OffchainErr::FailedSigning")?;

			log::debug!(target: LOG_TARGET, "Call::submit_estimationfor obj_idx={} - ok", &obj_idx);
			let call = Call::submit_estimation { est, signature };

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| "OffchainErr::SubmitTransaction")?;

			log::debug!(target: LOG_TARGET, "Call::submit_estimation for obj_idx={} - ok", &obj_idx);
			Ok(())
		};

		let mut lock = StorageLock::<Time>::new(ESTIMATION_LOCK);
		let _guard = lock.lock();

		let key = b"estimations";
		let val = StorageValueRef::persistent(key);

		let _res = val.mutate(|est: Result<Option<Vec<(u32, u64, u32)>>, StorageRetrievalError>| {
			match est {
				Ok(Some(mut v)) => { // if block_number < block + T::StatPeriod::get() =>
					for item in v.iter_mut() {
						if item.2 == 0 {
							// send
							let _ = send_item(item.0, item.1);
							item.2 = 1;
						}
					}
					Ok(v)
				},
				Ok(None) => Ok(Vec::new()),
				Err(e) => {
					log::error!(target: LOG_TARGET, "Estimation read from local storage error: {:#?}", e);
					return Err("Send estimation local storage error")
				},
			}
		});

		Ok(())
	}

	fn save_estimation(_block_number: T::BlockNumber, object_idx: u32, t: u64) -> bool {
		let mut lock = StorageLock::<Time>::new(ESTIMATION_LOCK);
		let _guard = lock.lock();

		let key = b"estimations";
		let val = StorageValueRef::persistent(key);

		let res = val.mutate(|est: Result<Option<Vec<(u32, u64, u32)>>, StorageRetrievalError>| {
			match est {
				Ok(Some(mut v)) => {
					let pos = v.iter().position(|&r| r.0 == object_idx);
					if let Some(pos) = pos {
						if v[pos].2 > 0 {
							log::debug!(target: LOG_TARGET, "Estimation has been  already written to local storage: {}", &object_idx);
							return Err("RECENTLY_SENT")
						} else {
							v[pos] = (object_idx, t, 0)
						}
					} else {
						v.push((object_idx, t, 0))
					}
					Ok(v)
				},
				Ok(None) => Ok(vec![(object_idx, t, 0)]),
				Err(e) => {
					log::error!(target: LOG_TARGET, "Estimation write to local storage error: {:#?}", e);
					return Err("ERROR")
				},
			}
		});

		log::debug!(target: LOG_TARGET, "Estimation written to local storage: {}", res.is_ok());
		// TODO: check res correctly
		res.is_ok()
	}

	// fn get_estimations() -> Vec<u32> {
	// 	let mut lock = StorageLock::<Time>::new(ESTIMATION_LOCK);
	// 	let _guard = lock.lock();
	// 	let key = b"estimations";
	// 	let val = StorageValueRef::persistent(key);
	//
	// 	let res = val.get();
	// 	let v = match res {
	// 		Ok(Some(v)) => v,
	// 		_ => Vec::new(),
	// 	};
	//
	// 	// log::debug!(target: LOG_TARGET, "Estimations: {}", &v);
	// 	v
	// }

	pub fn do_schedule_session_duration(
		start_session: SessionIndex,
		duration: BlockNumberFor<T>,
	) -> Result<BlockNumberFor<T>, DispatchError> {
		let block_number = <frame_system::Pallet<T>>::block_number();
		let current_session = Self::session_index();

		ensure!(start_session > current_session, Error::<T>::InvalidSession);
		ensure!(!duration.is_zero(), Error::<T>::InvalidDuration);

		if duration == Self::session_duration() {
			return Ok(block_number);
		}

		let next_session = Self::estimate_next_session_rotation(block_number)
			.0
			.ok_or(Error::<T>::EstimateNextSessionFailed)?;
		let target_block_number =
			Into::<BlockNumberFor<T>>::into(start_session.saturating_sub(current_session).saturating_sub(1))
				.saturating_mul(Self::session_duration())
				.saturating_add(next_session);

		SessionDurationChanges::<T>::insert(target_block_number, (start_session, duration));

		Ok(target_block_number)
	}

}

// Provides the new set of validators to the session module when session is
// being rotated.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
	// Plan a new session and provide new validator set.
	fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
		Self::renew_locks();

		if Self::is_slash_delay() {
			log::debug!(target: LOG_TARGET, "New session called; within slash delay.");
			<OfflineValidators<T>>::put(Vec::<T::AccountId>::new());
			return Some(Self::validators())
		}

		Self::mark_if_no_locks();
		// Remove any offline and slashed validators.
		Self::remove_offline_validators();
		log::debug!(target: LOG_TARGET, "New session called; updated validator set provided.");

		Some(Self::validators())
	}

	fn end_session(_end_index: u32) {}

	fn start_session(_start_index: u32) {}
}

impl<T: Config> ShouldEndSession<BlockNumberFor<T>> for Pallet<T> {
	fn should_end_session(now: BlockNumberFor<T>) -> bool {
		let offset = Self::duration_offset();
		let period = Self::session_duration();

		if period.is_zero() {
			return false;
		}

		now >= offset && (now.saturating_sub(offset) % period).is_zero()
	}
}

impl<T: Config> EstimateNextSessionRotation<BlockNumberFor<T>> for Pallet<T> {
	fn average_session_length() -> BlockNumberFor<T> {
		Self::session_duration()
	}

	fn estimate_current_session_progress(now: BlockNumberFor<T>) -> (Option<Permill>, Weight) {
		let offset = Self::duration_offset();
		let period = Self::session_duration();

		if period.is_zero() {
			return (None, 0);
		}

		// NOTE: we add one since we assume that the current block has already elapsed,
		// i.e. when evaluating the last block in the session the progress should be 100%
		// (0% is never returned).
		let progress = if now >= offset {
			let current = (now.saturating_sub(offset) % period).saturating_add(One::one());
			Some(Permill::from_rational(current, period))
		} else {
			None
		};

		(progress, 0)
	}

	fn estimate_next_session_rotation(now: BlockNumberFor<T>) -> (Option<BlockNumberFor<T>>, Weight) {
		let offset = Self::duration_offset();
		let period = Self::session_duration();

		if period.is_zero() {
			return (None, 0);
		}

		let next_session = if now > offset {
			let block_after_last_session = now.saturating_sub(offset) % period;
			if block_after_last_session > Zero::zero() {
				now.saturating_add(period.saturating_sub(block_after_last_session))
			} else {
				// this branch happens when the session is already rotated or will rotate in this
				// block (depending on being called before or after `session::on_initialize`). Here,
				// we assume the latter, namely that this is called after `session::on_initialize`,
				// and thus we add period to it as well.
				now.saturating_add(period)
			}
		} else {
			offset
		};

		(Some(next_session), 0)
	}
}

// Implementation of Convert trait for mapping ValidatorId with AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::ValidatorId, Option<T::ValidatorId>> for ValidatorOf<T> {
	fn convert(account: T::ValidatorId) -> Option<T::ValidatorId> {
		Some(account)
	}
}

impl<T: Config> ValidatorSet<T::AccountId> for Pallet<T> {
	type ValidatorId = T::ValidatorId;
	type ValidatorIdOf = T::ValidatorIdOf;

	fn session_index() -> sp_staking::SessionIndex {
		pallet_session::Pallet::<T>::current_index()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		pallet_session::Pallet::<T>::validators()
	}
}

impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for Pallet<T> {
	type Identification = T::ValidatorId;
	type IdentificationOf = ValidatorOf<T>;
}

// Offence reporting and unresponsiveness management.
impl<T: Config, O: Offence<(T::AccountId, T::AccountId)>>
	ReportOffence<T::AccountId, (T::AccountId, T::AccountId), O> for Pallet<T>
{
	fn report_offence(_reporters: Vec<T::AccountId>, offence: O) -> Result<(), OffenceError> {
		if Self::is_slash_delay() {
			return Ok(())
		}

		let offenders = offence.offenders();
		let penalty: u128 = T::PenaltyOffline::get();
		let val: BalanceOf<T> = penalty.saturated_into();

		for (v, _) in offenders.into_iter() {
			log::debug!(target: LOG_TARGET, "offender reported: {:?}", &v);

			if !Self::validators().contains(&v) {
				continue
			}

			let current_block = frame_system::Pallet::<T>::block_number();
			let last_slash = <LastThrow<T>>::get(&v);
			match last_slash {
				Some(when) if current_block - when < THROW_WO_SLASH_WINDOW.into() =>
					Self::slash(&v, val,
						|acc, amount| Event::<T>::ValidatorSlash(acc.clone(), amount)
					),
				_ => {},
			}
			<LastThrow<T>>::insert(&v, current_block);
			Self::mark_for_removal(v, RemoveReason::ImOnlineSlash);
		}

		Ok(())
	}

	fn is_known_offence(
		_offenders: &[(T::AccountId, T::AccountId)],
		_time_slot: &O::TimeSlot,
	) -> bool {
		false
	}
}

impl<T: Config> ValidatorSetApi<T::AccountId, T::BlockNumber, BalanceOf::<T>> for Pallet<T> {
	fn validators() -> Vec<T::AccountId> {
		pallet_session::Pallet::<T>::validators().iter()
			.map(|v| T::AccountId::decode(&mut &v.encode()[..]).unwrap())
			.collect()
	}
	fn author(block_num: T::BlockNumber) -> Option<T::AccountId> {
		Authors::<T>::get(block_num)
	}
}

pub type IdentificationTuple<T> = (
	<T as pallet_session::Config>::ValidatorId,
	<Pallet::<T> as ValidatorSetWithIdentification<
		<T as frame_system::Config>::AccountId,
	>>::Identification,
);

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
pub struct ValidatorProof {
	/// Authority Id
	pub authority_id: GrandpaId,
	/// The session index on which the specific key is a member.
	pub session: SessionIndex,
	/// The validator count of the session on which the specific key is a member.
	pub validator_count: ValidatorCount,
}

use sp_session::{GetSessionNumber, GetValidatorCount};

impl GetSessionNumber for ValidatorProof {
	fn session(&self) -> SessionIndex {
		self.session
	}
}

impl GetValidatorCount for ValidatorProof {
	fn validator_count(&self) -> ValidatorCount {
		self.validator_count
	}
}

impl<T: Config + pallet_session::Config> KeyOwnerProofSystem<(KeyTypeId, GrandpaId)> for Pallet<T> {
	type Proof = ValidatorProof;
	type IdentificationTuple = IdentificationTuple<T>;

	fn prove(key: (KeyTypeId, GrandpaId)) -> Option<Self::Proof> {
		let session = Self::session_index();
		let validators = Self::validators();
		let validator_count = validators.len() as ValidatorCount;

		log::debug!(target: LOG_TARGET, "create_prove: {:#?}", &key.1.encode());

		let vp = ValidatorProof {authority_id: key.1, session, validator_count};

		Some(vp)
	}

	fn check_proof(key: (KeyTypeId, GrandpaId), _proof: Self::Proof) -> Option<IdentificationTuple<T>>
	{
		log::debug!(target: LOG_TARGET, "check_proof");
		let maybe_val_id = pallet_session::KeyOwner::<T>::get((key.0, key.1.encode()));
		match maybe_val_id {
			Some(val_id) => {
				log::debug!(target: LOG_TARGET, "check_proof: {:#?}", &val_id);
				Some((val_id.clone(), val_id))
			},
			None => None,
		}
	}
}

impl<
	T: Config,
	Offender: Clone + Into<(<T as frame_system::Config>::AccountId, <T as pallet_session::Config>::ValidatorId)>,
	Res: Default
>
OnOffenceHandler<T::AccountId, Offender, Res> for Pallet<T> {
	fn on_offence(
		offenders: &[sp_staking::offence::OffenceDetails<T::AccountId, Offender>],
		slash_fraction: &[Perbill],
		_session: SessionIndex,
		_disable_strategy: sp_staking::offence::DisableStrategy,
	) -> Res {

		let n_off = offenders.len();
		log::debug!(target: LOG_TARGET, "Equivocation OffenceDetails size: {}", &n_off);
		for sf in slash_fraction {
			log::debug!(target: LOG_TARGET, "Equivocation slash_fraction: {:#?}", sf);
		}

		for off_det in offenders {
			let acc_id = off_det.offender.clone().into().0;
			let val_id = off_det.offender.clone().into().1;

			log::debug!(target: LOG_TARGET, "Equivocation offender: {:#?}", hex::encode(acc_id.encode()));
			let n_rep = off_det.reporters.len();
			log::debug!(target: LOG_TARGET, "Equivocation offender size: {}", &n_rep);
			for rep_id in off_det.reporters.iter() {
				let acc = hex::encode(rep_id.encode());
				log::debug!(target: LOG_TARGET, "Reporter:: {:#?}", &acc);
			}

			if !Self::validators().contains(&acc_id) {
				continue
			}

			let penalty: u128 = 2 * T::PenaltyOffline::get();
			let val: BalanceOf<T> = penalty.saturated_into();

			Self::slash(
				&acc_id,
				val,
				|acc, amount|
					Event::<T>::ValidatorSlash(acc.clone(), amount)
			);

			if <Validators<T>>::get().len() > T::MinAuthorities::get() as usize {
				let _ = pallet_session::Pallet::<T>::disable(&val_id);
				pallet_session::NextKeys::<T>::remove(&val_id);
				let mut session_keys = pallet_session::QueuedKeys::<T>::get();
				session_keys.retain(|v| (*v).0 != val_id);
				pallet_session::QueuedKeys::<T>::put(session_keys);
			}
			Self::mark_for_removal(acc_id, RemoveReason::Equivocation);
		}

		Default::default()
	}
}